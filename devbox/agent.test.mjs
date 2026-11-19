import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';
import sharp from 'sharp';

const exec = promisify(execFile);
const cli = fileURLToPath(new URL('./cli.mjs', import.meta.url));
const client = await readFile(new URL('../crates/cargo-aeris-devbox/src/agent-client.mjs', import.meta.url), 'utf8');

test('Agent CLI observes and operates Gallery in DemoMode', { timeout: 150_000 }, async (t) => {
  const output = await mkdtemp(join(tmpdir(), 'aeris-agent-test-'));
  t.after(() => rm(output, { recursive: true, force: true }));
  const server = spawn(process.execPath, [cli, 'agent', 'serve', '--demo'], { stdio: ['ignore', 'pipe', 'inherit'] });
  const exited = once(server, 'exit');
  const stop = async () => {
    server.kill('SIGTERM');
    await exited;
  };
  t.after(stop);

  const lines = createInterface({ input: server.stdout });
  const [line] = await Promise.race([once(lines, 'line'), exited.then(() => assert.fail('Server exited before startup'))]);
  const { session, url } = JSON.parse(line);
  if (process.platform !== 'win32') assert.equal((await stat(session)).mode & 0o777, 0o600);
  const anonymous = await fetch(url + '/devbox-agent', { method: 'POST' });
  assert.equal(anonymous.status, 403);
  await anonymous.body.cancel();

  const agent = async (...args) => {
    const { stdout } = await exec(process.execPath, ['--input-type=module', '--eval', client, '--', ...args, '--session', session]);
    const result = JSON.parse(stdout);
    assert.equal(result.version, 1);
    return result;
  };
  const expectError = (code, ...args) => assert.rejects(() => agent(...args), (failure) => {
    assert.equal(JSON.parse(failure.stdout).error.code, code);
    return true;
  });

  const { buildId, previewId } = await agent('wait');
  const { nodes, device } = await agent('snapshot');
  assert.equal(device, 'xiaomi-band-10-pro-3.101.043');
  const largeTitle = nodes.find((node) => node.component === 'AerisUI.LargeTitleArea');
  assert(nodes.some((node) => node.parent === largeTitle.id && node.value === 'Gallery' && node.opacity === 255));
  const inputs = nodes.find((node) => node.value === 'Input Controls' && !node.hidden);
  const [x, y] = [String(inputs.x + 10), String(inputs.y + 10)];
  await agent('tap', x, y, '--preview', previewId);
  assert((await agent('snapshot')).nodes.some((node) => node.component === 'AerisUI.NumericWheel' && !node.hidden));
  await agent('back', '--preview', previewId);
  const home = (await agent('snapshot')).nodes;
  assert(home.some((node) => node.value === 'Gallery' && !node.hidden));

  const band11 = 'xiaomi-band-11-4.100.155';
  const narrow = await agent('snapshot', '--device', band11);
  assert.deepEqual([narrow.width, narrow.height], [212, 520]);
  const narrowHeader = narrow.nodes.find((node) => node.component === 'AerisUI.HeaderBar');
  assert.deepEqual([narrowHeader.x, narrowHeader.y, narrowHeader.width], [0, 0, 212]);
  const narrowTitles = narrow.nodes.filter((node) => node.component === 'AerisUI.ShrinkingTitle');
  assert.equal(narrowTitles.length, 1);
  assert.equal(narrowTitles[0].value, 'Gallery');
  assert.equal(narrowTitles[0].fontApx, 7);
  assert.equal(narrowTitles[0].fontSize, 39);
  const narrowPager = narrow.nodes.find((node) => node.component === 'AerisUI.Pager');
  assert.deepEqual([narrowPager.x, narrowPager.y, narrowPager.width, narrowPager.height], [0, 0, 212, 520]);
  const narrowInputs = narrow.nodes.find((node) => node.value === 'Inputs');
  await agent('tap', String(narrowInputs.x + 10), String(narrowInputs.y + 10),
    '--preview', narrow.previewId, '--device', band11);
  assert((await agent('snapshot', '--device', band11)).nodes.some((node) =>
    node.component === 'AerisUI.NumericWheel' && node.width === 188 && !node.hidden));

  const feedbackDot = home.filter((node) => node.component === 'AerisUI.PageIndicatorDot')
    .sort((a, b) => a.x - b.x).at(-1);
  await agent('tap', String(feedbackDot.x + 4), String(feedbackDot.y + 4), '--preview', previewId);
  let feedback;
  for (let attempt = 0; attempt < 15; attempt++) {
    feedback = (await agent('snapshot')).nodes;
    if (feedback.some((node) => node.value === 'Next Sample' && node.x === 3)) break;
    await delay(40);
  }
  const patchButtons = ['Next Sample', 'Open Dialog'].map((label) => {
    const text = feedback.find((node) => node.value === label);
    const button = feedback.find((node) => node.id === text.parent);
    assert.equal(button.className, 'NinePatch');
    assert.equal(button.width, 162);
    assert.deepEqual(feedback.filter((node) => node.parent === button.id).map((node) => node.id), [text.id]);
    return button;
  });

  const filename = join(output, 'preview.png');
  const screenshot = await agent('screenshot', '--out', filename);
  assert.equal(screenshot.previewId, previewId);
  const png = await readFile(filename);
  assert.deepEqual([screenshot.width, screenshot.height], [336, 480]);
  assert.deepEqual([png.readUInt32BE(16), png.readUInt32BE(20)], [336, 480]);
  const { data, info } = await sharp(png).removeAlpha().raw().toBuffer({ resolveWithObject: true });
  for (const button of patchButtons) {
    for (const y of [20, 58]) {
      const pixel = (x, channel) => data[((button.y + y) * info.width + button.x + x) * info.channels + channel];
      for (let channel = 0; channel < info.channels; channel++) {
        for (let x = 79; x <= 82; x++) {
          assert(Math.abs(pixel(x, channel) - pixel(78, channel)) <= 4, 'NinePatch join must stay filled');
        }
      }
    }
  }
  await expectError('OUTPUT_EXISTS', 'screenshot', '--out', filename);
  await expectError('INVALID_ARGUMENT', 'tap', '336', '0', '--preview', previewId);
  const { entries, cursor } = await agent('logs', '--source', 'runtime');
  assert(entries.every((entry) => entry.device === device));
  assert.deepEqual((await agent('logs', '--since', String(cursor))).entries, []);
  await expectError('TIMEOUT', 'wait', '--after', String(buildId), '--timeout', '200');

  const descriptor = JSON.parse(await readFile(session, 'utf8'));
  const request = async (command, device, args = {}) => {
    const response = await fetch(descriptor.url + '/devbox-agent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ' + descriptor.token },
      body: JSON.stringify({ command, device, ...args }),
    });
    const result = await response.json();
    assert.equal(result.ok, true, JSON.stringify(result.error));
    return result;
  };
  await agent('back', '--preview', narrow.previewId, '--device', band11);
  for (const device of ['xiaomi-band-10-pro-3.101.043', band11]) {
    let screen = await request('snapshot', device);
    const dot = screen.nodes.filter((node) => node.component === 'AerisUI.PageIndicatorDot')
      .sort((a, b) => a.x - b.x).at(-1);
    await request('tap', device, { points: [dot.x + 3, dot.y + 3], preview: screen.previewId });
    await delay(350);
    for (const action of ['confirm', 'cancel', 'back']) {
      screen = await request('snapshot', device);
      const open = screen.nodes.find((node) => node.value === 'Open Dialog');
      await request('tap', device, { points: [open.x + 20, open.y + 15], preview: screen.previewId });
      await delay(350);
      screen = await request('snapshot', device);
      if (action === 'back') {
        await request('back', device, { preview: screen.previewId });
      } else {
        const label = screen.nodes.find((node) => node.value === (action === 'confirm' ? 'Apply' : 'Cancel'));
        const points = device === band11
          ? [106, action === 'confirm' ? 477 : 43]
          : [label.x + 30, label.y + 15];
        await request('tap', device, { points, preview: screen.previewId });
      }
      let surface, content;
      for (let attempt = 0; attempt < 10; attempt++) {
        const closing = await request('snapshot', device);
        surface = closing.nodes.find((node) => node.component === 'AerisUI.ConfirmDialog');
        content = closing.nodes.find((node) => node.parent === surface.id && node.kind === 3);
        if (surface.hidden || content.opacity < 255) break;
        await delay(10);
      }
      assert.equal(surface.hidden, false, `${device}: ${action} keeps the dialog visible during its exit`);
      assert(content.opacity > 0 && content.opacity < 255, `${device}: ${action} animates the content out`);
      await delay(300);
      assert((await request('snapshot', device)).nodes.find((node) => node.id === surface.id).hidden);
    }
  }

  assert.notEqual((await agent('reload')).previewId, previewId);
  await expectError('STALE_PREVIEW', 'tap', x, y, '--preview', previewId);
  await stop();
  try {
    // Windows kills the child directly when a signal is sent over child_process.
    if (process.platform !== 'win32') await assert.rejects(() => stat(session), { code: 'ENOENT' });
    await expectError('SESSION_UNAVAILABLE', 'status');
  } finally {
    if (process.platform === 'win32') await rm(join(session, '..'), { recursive: true, force: true });
  }
});

test('Workbench settings independently grant CLI and MCP access to one Gallery preview', { timeout: 150_000 }, async (t) => {
  const { Client } = await import('@modelcontextprotocol/sdk/client/index.js');
  const { StreamableHTTPClientTransport } = await import('@modelcontextprotocol/sdk/client/streamableHttp.js');
  const { chromium } = await import('playwright');
  const server = spawn(process.execPath, [cli, '--demo'], {
    stdio: ['ignore', 'pipe', 'inherit'],
    env: { ...process.env, AERIS_DEVBOX_PORT: '5175', NO_COLOR: '1' },
  });
  const exited = once(server, 'exit');
  t.after(async () => {
    server.kill('SIGTERM');
    await exited;
  });
  const lines = createInterface({ input: server.stdout });
  const origin = await Promise.race([
    (async () => {
      for await (const line of lines) {
        const match = line.match(/http:\/\/127\.0\.0\.1:\d+/);
        if (match) return match[0];
      }
      assert.fail('Workbench did not report its URL');
    })(),
    exited.then(() => assert.fail('Workbench exited before startup')),
  ]);
  // Settings are only readable by the workbench origin.
  const direct = await fetch(origin + '/devbox-agent-settings');
  assert.equal(direct.status, 403);
  await direct.body.cancel();

  const browser = await chromium.launch();
  t.after(() => browser.close());
  const context = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] });
  const page = await context.newPage();
  await page.goto(origin);
  await page.waitForFunction(() => document.title === 'Aeris UI Gallery / Aeris DevBox');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  const cliSwitch = page.getByRole('switch', { name: 'Agent CLI' });
  const mcpSwitch = page.getByRole('switch', { name: 'MCP', exact: true });
  await mcpSwitch.waitFor();
  assert.equal(await cliSwitch.getAttribute('aria-checked'), 'false');
  assert.equal(await mcpSwitch.getAttribute('aria-checked'), 'false');
  await mcpSwitch.click();
  await page.getByRole('button', { name: '复制 MCP 配置' }).click();
  await page.getByText('已复制 MCP 配置').waitFor();
  const config = JSON.parse(await page.evaluate(() => navigator.clipboard.readText())).mcpServers['aeris-devbox'];

  const mcp = new Client({ name: 'devbox-test', version: '1.0.0' });
  t.after(() => mcp.close());
  await mcp.connect(new StreamableHTTPClientTransport(new URL(config.url), { requestInit: { headers: config.headers } }));
  const { tools } = await mcp.listTools();
  assert.equal(tools.length, 10);
  const call = (name, args = {}) => mcp.callTool({ name: 'devbox_' + name, arguments: args });
  assert.equal((await call('wait', { timeoutMs: 120_000 })).structuredContent.ok, true);
  const snapshot = await call('snapshot');
  const { previewId, device, nodes } = snapshot.structuredContent;
  const inputs = nodes.find((node) => node.value === 'Input Controls' && !node.hidden);
  assert.equal((await call('tap', { x: inputs.x + 10, y: inputs.y + 10, preview: previewId, device })).structuredContent.previewId, previewId);
  const image = await call('screenshot');
  assert.equal(image.content[0].type, 'image');
  assert.equal(image.content[0].mimeType, 'image/png');
  const png = Buffer.from(image.content[0].data, 'base64');
  assert.deepEqual([png.readUInt32BE(16), png.readUInt32BE(20)], [336, 480]);

  await cliSwitch.click();
  await page.getByRole('button', { name: '复制会话路径' }).click();
  await page.getByText('已复制会话路径').waitFor();
  const session = await page.evaluate(() => navigator.clipboard.readText());
  const descriptor = JSON.parse(await readFile(session, 'utf8'));
  const readCli = async () => {
    const response = await fetch(descriptor.url + '/devbox-agent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ' + descriptor.token },
      body: JSON.stringify({ command: 'snapshot' }),
    });
    return response.json();
  };
  const tree = await readCli();
  assert.equal(tree.previewId, previewId);
  assert(tree.nodes.some((node) => node.component === 'AerisUI.NumericWheel' && !node.hidden));

  await mcpSwitch.click();
  await page.getByRole('button', { name: '复制 MCP 配置' }).waitFor({ state: 'hidden' });
  await assert.rejects(() => mcp.listTools(), /认证失败/);
  assert.equal((await readCli()).ok, true);
  await cliSwitch.click();
  await page.getByRole('button', { name: '复制会话路径' }).waitFor({ state: 'hidden' });
  assert.equal((await readCli()).error.code, 'FORBIDDEN');
  await assert.rejects(() => stat(session), { code: 'ENOENT' });

  await cliSwitch.click();
  await page.getByRole('button', { name: '复制会话路径' }).waitFor();
  assert.equal((await readCli()).error.code, 'FORBIDDEN');
  await mcpSwitch.click();
  await page.getByRole('button', { name: '复制 MCP 配置' }).waitFor();
  await assert.rejects(() => mcp.listTools(), /认证失败/);
});

test('Agent scopes live preview, input and scenes to the selected device', { timeout: 150_000 }, async (t) => {
  const config = fileURLToPath(new URL('./component-previews/aeris.config.toml', import.meta.url));
  const server = spawn(process.execPath, [cli, 'agent', 'serve', config], { stdio: ['ignore', 'pipe', 'inherit'] });
  const exited = once(server, 'exit');
  t.after(async () => {
    server.kill('SIGTERM');
    await exited;
  });
  const lines = createInterface({ input: server.stdout });
  const [line] = await Promise.race([once(lines, 'line'), exited.then(() => assert.fail('Server exited before startup'))]);
  const { session } = JSON.parse(line);
  const agent = async (...args) => {
    const { stdout } = await exec(process.execPath, ['--input-type=module', '--eval', client, '--', ...args, '--session', session]);
    return JSON.parse(stdout);
  };
  const [stock, round] = await Promise.all([
    agent('wait'),
    agent('wait', '--device', 'round-390'),
  ]);
  assert.equal(stock.devices.length, 5);
  assert.equal(round.device, 'round-390');
  assert.notEqual(round.previewId, stock.previewId);
  const snapshot = await agent('snapshot', '--device', 'round-390');
  assert.deepEqual([snapshot.width, snapshot.height], [390, 390]);
  await assert.rejects(
    agent('tap', '195', '195', '--preview', stock.previewId, '--device', 'round-390'),
    (failure) => JSON.parse(failure.stdout).error.code === 'STALE_PREVIEW',
  );
  await assert.rejects(
    agent('tap', '0', '0', '--preview', round.previewId, '--device', 'round-390'),
    (failure) => JSON.parse(failure.stdout).error.code === 'INVALID_ARGUMENT',
  );
  await agent('scene', 'layout-empty', '--device', 'round-390');
  assert.equal((await agent('status', '--device', 'round-390')).scene, 'layout-empty');
  const capsule = 'xiaomi-band-11-4.100.155';
  await agent('scene', 'layout-header', '--device', capsule);
  const narrow = await agent('snapshot', '--device', capsule);
  const header = narrow.nodes.find((node) => node.component === 'AerisUI.HeaderBar');
  assert.deepEqual([header.x, header.y, header.width], [0, 0, 212]);
  const stockAfter = await agent('status');
  assert.equal(stockAfter.scene, 'layout-settings');
  assert.equal(stockAfter.preview.previewId, stock.previewId);

  // Both modal components must capture a real blurred image at the device's size.
  for (const device of ['xiaomi-band-10-pro-3.101.043', capsule, 'redmi-watch', 'round-390']) {
    for (const [scene, component] of [
      ['adaptive-dialog', 'AerisUI.ConfirmDialog'],
      ['adaptive-confirmation', 'AerisUI.ConfirmationOverlay'],
    ]) {
      await agent('scene', scene, '--device', device);
      let modal;
      for (let attempt = 0; attempt < 40; attempt++) {
        modal = await agent('snapshot', '--device', device);
        const surface = modal.nodes.find((node) => node.component === component && !node.hidden);
        const backdrop = surface && modal.nodes.find((node) => node.parent === surface.id && node.kind === 2 && !node.hidden);
        if (backdrop?.opacity === 255) break;
        await delay(40);
      }
      const surface = modal.nodes.find((node) => node.component === component && !node.hidden);
      assert(surface, `${device}: ${component} is visible`);
      assert.equal(surface.backgroundOpacity, 0, `${device}: blur replaces the fallback shade`);
      const backdrop = modal.nodes.find((node) => node.parent === surface.id && node.kind === 2 && !node.hidden);
      assert(backdrop, `${device}: ${component} has a captured backdrop`);
      assert.equal(backdrop.opacity, 255);
      assert.deepEqual([backdrop.width, backdrop.height], [Math.ceil(modal.width / 4), Math.ceil(modal.height / 4)]);
      if ([capsule, 'xiaomi-band-10-pro-3.101.043'].includes(device) && scene === 'adaptive-confirmation') {
        // Wait for the details transition before checking its final placement.
        const expectedY = device === capsule ? 214 : 244;
        const headerHeight = device === capsule ? 86 : 94;
        for (let attempt = 0; attempt < 40; attempt++) {
          modal = await agent('snapshot', '--device', device);
          const body = modal.nodes.find((node) => node.value === 'Your timer is ready to use.');
          if (body?.opacity === 255 && body.y === expectedY) break;
          await delay(40);
        }
        const body = modal.nodes.find((node) => node.value === 'Your timer is ready to use.');
        assert.equal(body.y, expectedY);
        assert.equal(modal.height - body.y - body.height, headerHeight + 10);
      }
    }
  }
});
