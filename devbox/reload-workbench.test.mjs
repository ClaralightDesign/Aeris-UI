import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { test } from 'node:test';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const cli = fileURLToPath(new URL('./cli.mjs', import.meta.url));
const gallery = fileURLToPath(new URL('../examples/gallery/', import.meta.url));
const library = fileURLToPath(new URL('../crates/aeris-ui', import.meta.url));
const devices = ['xiaomi-band-10', 'xiaomi-band-11'];

async function until(check, timeout = 20_000) {
  const end = Date.now() + timeout;
  do {
    const value = await check();
    if (value) return value;
    await delay(80);
  } while (Date.now() < end);
  assert.fail('Timed out waiting for the preview or build');
}

test('Workbench keeps both devices live during reload, rebuild and recovery', { timeout: 180_000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), 'aeris-reload-test-'));
  let server, browser, exited;
  const logs = [];
  try {
    for (const name of ['src', 'assets', 'fonts', 'Canopus.toml', 'build.rs', 'Cargo.lock']) {
      await cp(join(gallery, name), join(directory, name), { recursive: true });
    }
    const manifest = (await readFile(join(gallery, 'Cargo.toml'), 'utf8'))
      .replace('"../../crates/aeris-ui"', JSON.stringify(library));
    const template = await readFile(join(gallery, 'aeris.config.toml'), 'utf8');
    const config = join(directory, 'aeris.config.toml');
    function configure(accent = '#0088FF', enabled = 'true') {
      return writeFile(config, template.replace('#0088FF', accent).replace('hot_reload = true', 'hot_reload = ' + enabled)
        .replace('agent_cli = false', 'agent_cli = true').replace('mcp = false', 'mcp = true'));
    }
    await writeFile(join(directory, 'Cargo.toml'), manifest);
    await configure();
    function startServer() {
      server = spawn(process.execPath, [cli, config], {
        env: { ...process.env, AERIS_DEVBOX_PORT: '0', NO_COLOR: '1' },
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      exited = once(server, 'exit');
      server.stderr.on('data', (chunk) => logs.push(String(chunk)));
      return Promise.race([
        (async () => {
          for await (const line of createInterface({ input: server.stdout })) {
            const address = line.match(/http:\/\/127\.0\.0\.1:\d+/);
            if (address) return address[0];
          }
          assert.fail('Workbench did not publish its address');
        })(),
        exited.then(() => assert.fail('Workbench exited during startup')),
      ]);
    }
    const origin = await startServer();
    const project = () => fetch(origin + '/devbox-project.json').then((response) => response.json());
    const buildAfter = (build) => until(async () => {
      const next = await project();
      assert.notEqual(next.status, 'failed', next.logs.at(-1)?.text);
      return next.status === 'ready' && next.publishedBuildId > build && next;
    }, 60_000);
    const initial = await buildAfter(0);
    browser = await chromium.launch();
    const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });
    page.setDefaultTimeout(10_000);
    await page.route('https://cdn.cnbj1.fds.api.mi-img.com/**', (route) => route.abort());
    await page.goto(origin);

    const iframe = (device) => page.locator('iframe:not([data-staging])[src*="device=' + device + '"]');
    async function frame(device) {
      const element = await iframe(device).elementHandle();
      const result = await element.contentFrame();
      await result.waitForFunction(() => window.aerisDebugger?.snapshot());
      return result;
    }
    const source = (device) => iframe(device).getAttribute('src');
    async function published(build) {
      await until(async () => {
        const errors = await page.locator('.debug-reload-notice[role=alert]').allTextContents();
        assert.deepEqual(errors, []);
        const urls = await Promise.all(devices.map(source));
        return urls.every((url) => new URL(url, origin).searchParams.get('workbenchBuild') === String(build));
      });
    }
    const state = async (device) => {
      const preview = await frame(device);
      return until(() => preview.evaluate(() => window.aerisReload.capture()?.state));
    };
    const node = async (preview, value, component) => preview.evaluate(({ value, component }) =>
      window.aerisDebugger.snapshot().nodes.find((item) => !item.hidden
        && (!value || item.value === value) && (!component || item.component === component)), { value, component });
    async function point(preview, x, y) {
      const canvas = preview.locator('canvas');
      const box = await canvas.boundingBox();
      const { width, height } = await canvas.evaluate((element) => ({ width: element.width, height: element.height }));
      return [box.x + (x + 0.5) * box.width / width, box.y + (y + 0.5) * box.height / height];
    }
    const settings = page.locator('.debug-preview-menu');
    async function openSettings() {
      if (!await settings.isVisible()) await page.getByRole('button', { name: '设置', exact: true }).click();
      await settings.waitFor();
    }
    async function closeSettings() {
      if (!await settings.isVisible()) return;
      await page.mouse.click(4, 70);
      await settings.waitFor({ state: 'hidden' });
    }
    async function control(label) {
      await openSettings();
      await settings.getByRole('button', { name: label, exact: true }).click();
    }
    const switchFor = (name) => settings.getByRole('switch', { name, exact: true });
    async function setSwitch(name, enabled) {
      await openSettings();
      const toggle = switchFor(name);
      await until(() => toggle.isEnabled());
      if (await toggle.getAttribute('aria-checked') !== String(enabled)) await toggle.click();
      await until(async () => await toggle.getAttribute('aria-checked') === String(enabled));
    }
    await openSettings();
    const originalConfig = await readFile(config, 'utf8');
    for (const name of ['Agent CLI', 'MCP']) {
      await until(async () => await switchFor(name).getAttribute('aria-checked') === 'true');
      await setSwitch(name, false);
    }
    assert.equal(await readFile(config, 'utf8'), originalConfig);
    await page.reload();
    await openSettings();
    for (const name of ['Agent CLI', 'MCP']) {
      await until(() => switchFor(name).isEnabled());
      assert.equal(await switchFor(name).getAttribute('aria-checked'), 'false');
    }
    await closeSettings();

    const first = await frame(devices[0]), second = await frame(devices[1]);
    assert.equal((await first.locator('canvas').boundingBox()).width, 168);
    const entry = await node(first, 'Input Controls');
    await page.mouse.click(...await point(first, entry.x + entry.width / 2, entry.y + entry.height / 2));
    await first.waitForFunction(() => window.aerisDebugger.snapshot().nodes.some((item) =>
      !item.hidden && item.component === 'AerisUI.NumericWheel'));
    const wheel = await node(first, '', 'AerisUI.NumericWheelColumn');
    await page.mouse.move(...await point(first, wheel.x + wheel.width / 2, wheel.y + wheel.height / 2));
    await page.mouse.wheel(0, 100);
    const firstState = await state(devices[0]);
    assert(firstState.states.some((item) => item.path.at(-1)[1] === 'Month' && item.state !== 3));
    await page.mouse.move(...await point(second, 140, 250));
    await page.mouse.wheel(0, 160);
    const secondState = await state(devices[1]);
    assert(secondState.states.some((item) => item.scroll?.[1] > 0));

    const originalTitle = await node(first, 'Input Controls', 'AerisUI.HeaderTitle');
    await configure('#CC55AA');
    const automatic = await buildAfter(initial.publishedBuildId);
    await published(automatic.publishedBuildId);
    const newTitle = await node(await frame(devices[0]), 'Input Controls', 'AerisUI.HeaderTitle');
    assert.notEqual(newTitle.foreground, originalTitle.foreground);
    assert.deepEqual(await state(devices[0]), firstState);
    assert.deepEqual(await state(devices[1]), secondState);

    const beforeManual = await source(devices[0]);
    await control('重载');
    await until(async () => await source(devices[0]) !== beforeManual);
    assert.deepEqual(await state(devices[0]), firstState);
    for (const label of ['重新构建', '清理构建缓存']) {
      const before = await project();
      await control(label);
      const next = await buildAfter(before.publishedBuildId);
      await published(next.publishedBuildId);
      assert.deepEqual(await state(devices[0]), firstState);
      assert.deepEqual(await state(devices[1]), secondState);
    }

    async function autoRefresh(enabled) {
      await setSwitch('保存后自动刷新', enabled);
      await closeSettings();
    }
    await autoRefresh(false);
    const paused = await project(), retainedSources = await Promise.all(devices.map(source));
    await configure('#CC44AA');
    const pausedBuild = await buildAfter(paused.publishedBuildId);
    await delay(400);
    assert.deepEqual(await Promise.all(devices.map(source)), retainedSources);
    await control('重载');
    await until(async () => new URL(await source(devices[0]), origin).searchParams.get('workbenchBuild') === String(pausedBuild.publishedBuildId));
    assert.deepEqual(await state(devices[0]), firstState);
    assert.equal(await source(devices[1]), retainedSources[1]);
    await autoRefresh(true);

    const beforeRace = await project();
    let release;
    const held = new Promise((done) => { release = done; });
    let requested = false;
    const delayOlder = async (route) => {
      const build = new URL(route.request().url()).searchParams.get('workbenchBuild');
      if (Number(build) === beforeRace.publishedBuildId + 1) {
        requested = true;
        await held;
      }
      await route.continue().catch(() => {});
    };
    await page.route('**/preview.html?**', delayOlder);
    try {
      const shown = await source(devices[0]);
      await configure('#AA55AA');
      const older = await buildAfter(beforeRace.publishedBuildId);
      await until(() => requested);
      await configure('#AA33AA');
      const latest = await buildAfter(older.publishedBuildId);
      assert.equal(await source(devices[0]), shown);
      await published(latest.publishedBuildId);
      assert.deepEqual(await state(devices[0]), firstState);
    } finally {
      release();
      await page.unroute('**/preview.html?**', delayOlder);
    }

    const lastGood = await project(), oldSource = await source(devices[0]);
    await configure('#CC55AA', '"invalid"');
    await page.getByText('由于构建失败，当前预览未刷新').waitFor();
    assert.equal(await source(devices[0]), oldSource);
    const failCandidate = (route) => new URL(route.request().url()).searchParams.has('staged')
      ? route.fulfill({ contentType: 'text/html', body: '<canvas id="gallery"></canvas><script>window.aerisReload={restore(){throw new Error("新版应用启动失败")}};</script>' })
      : route.continue();
    await page.route('**/preview.html?**', failCandidate);
    await configure('#BB44AA');
    await until(async () => (await project()).publishedBuildId > lastGood.publishedBuildId);
    await page.getByText(/新版尚未切换。新版应用启动失败/).waitFor();
    assert.equal(await source(devices[0]), oldSource);
    const retained = await frame(devices[0]);
    await retained.locator('canvas').evaluate((element) => { element.tabIndex = 0; element.focus(); });
    await page.keyboard.press('Escape');
    await retained.waitForFunction(() => !window.aerisDebugger.snapshot().nodes.some((item) =>
      !item.hidden && item.component === 'AerisUI.NumericWheel'));
    await page.unroute('**/preview.html?**', failCandidate);

    await page.getByRole('button', { name: '从首页启动新版' }).click();
    const loaded = await frame(devices[0]);
    assert.equal(await node(loaded, '', 'AerisUI.NumericWheel'), undefined);
    const beforeDisabled = await project(), enabledSource = await source(devices[0]);
    await configure('#B044CC', 'false');
    const disabled = await buildAfter(beforeDisabled.publishedBuildId);
    assert.equal(disabled.hotReload, false);
    await openSettings();
    await until(async () => await switchFor('热重载').getAttribute('aria-checked') === 'false');
    assert.equal(await source(devices[0]), enabledSource);
    await control('重载');
    await until(async () => await source(devices[0]) !== enabledSource);
    assert.equal(new URL(await source(devices[0]), origin).searchParams.get('staged'), null);
    const cold = await frame(devices[0]);
    await closeSettings();
    const coldEntry = await node(cold, 'Input Controls');
    await page.mouse.click(...await point(cold, coldEntry.x + coldEntry.width / 2, coldEntry.y + coldEntry.height / 2));
    await until(() => node(cold, '', 'AerisUI.NumericWheel'));
    await setSwitch('热重载', true);
    await control('重载');
    await until(async () => new URL(await source(devices[0]), origin).searchParams.has('staged'));
    assert(await node(await frame(devices[0]), '', 'AerisUI.NumericWheel'));
    assert((await readFile(config, 'utf8')).includes('hot_reload = false'));

    await closeSettings();
    const inspector = page.locator('.debug-inspector');
    const hasScrollPane = async (role, name) => {
      const viewport = inspector.getByRole(role, { name, exact: true });
      await viewport.waitFor();
      assert.equal(await viewport.getAttribute('data-cl-slot'), 'scroll-area-viewport');
    };
    await hasScrollPane('region', 'LVGL 对象树');
    await hasScrollPane('region', '对象详情内容');
    await inspector.getByRole('button', { name: /^控制台/ }).click();
    await hasScrollPane('region', '日志输出');
    await hasScrollPane('navigation', '日志类别');
    await inspector.getByRole('button', { name: '源代码', exact: true }).click();
    await hasScrollPane('navigation', '项目文件');
    await hasScrollPane('group', '已打开的文件');
    await hasScrollPane('region', 'src/gallery.rs');
    await inspector.getByRole('button', { name: '图层', exact: true }).click();
    await hasScrollPane('region', '图层内容');

    await page.setViewportSize({ width: 1000, height: 580 });
    await openSettings();
    await settings.getByRole('button', { name: '清理构建缓存', exact: true }).hover();
    const hint = page.getByRole('tooltip', { name: '清理构建缓存' });
    await hint.waitFor();
    assert(await hint.evaluate((element) => {
      const glass = getComputedStyle(element.querySelector('.debug-tooltip-glass'));
      const fill = getComputedStyle(element.querySelector('.debug-tooltip-fill'));
      return glass.clipPath !== 'none' && glass.clipPath === fill.clipPath && glass.backdropFilter.includes('blur');
    }));
    await settings.getByText('设备、应用与版本', { exact: true }).click();
    await settings.getByText('工具链', { exact: true }).click();
    const scroll = settings.getByRole('region', { name: '设置内容' });
    assert.equal(await scroll.getAttribute('data-cl-slot'), 'scroll-area-viewport');
    await scroll.evaluate((element) => { element.scrollTop = 0; });
    await until(() => scroll.evaluate((element) => getComputedStyle(element).maskImage.includes('linear-gradient')));
    for (let step = 0; step < 8; step++) {
      const box = await scroll.boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.wheel(0, 260);
      await delay(65);
    }
    assert(await settings.getByText('Emscripten', { exact: true }).evaluate((element) => {
      const row = element.getBoundingClientRect();
      const viewport = element.closest('[data-cl-slot=scroll-area-viewport]').getBoundingClientRect();
      return row.top >= viewport.top && row.bottom <= viewport.bottom;
    }));

    server.kill('SIGTERM');
    await exited;
    await page.goto(await startServer());
    await openSettings();
    for (const [label, expected] of [['Agent CLI', 'true'], ['MCP', 'true'], ['热重载', 'false']]) {
      await until(() => switchFor(label).isEnabled());
      assert.equal(await switchFor(label).getAttribute('aria-checked'), expected);
    }
  } catch (error) {
    error.message += '\n' + logs.join('').slice(-2500);
    throw error;
  } finally {
    await browser?.close();
    if (server) {
      server.kill('SIGTERM');
      await exited;
    }
    await rm(directory, { recursive: true, force: true });
  }
});
