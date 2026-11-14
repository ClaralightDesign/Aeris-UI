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

const exec = promisify(execFile);
const cli = fileURLToPath(new URL('./cli.mjs', import.meta.url));
const client = await readFile(new URL('../crates/cargo-aeris-devbox/src/agent-client.mjs', import.meta.url), 'utf8');

test('Agent CLI observes and operates an isolated DemoMode preview', { timeout: 45_000 }, async (t) => {
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
  const timerState = async () => (await agent('snapshot')).nodes.find((node) => node.component === 'TimerState').value;
  assert.equal(await timerState(), '准备就绪');
  await agent('tap', '168', '380', '--preview', previewId);
  assert.equal(await timerState(), '进行中');
  await agent('tap', '168', '380', '--preview', previewId);
  assert.equal(await timerState(), '已暂停');
  await agent('tap', '168', '430', '--preview', previewId);
  assert.equal(await timerState(), '准备就绪');

  const filename = join(output, 'preview.png');
  const screenshot = await agent('screenshot', '--out', filename);
  assert.equal(screenshot.previewId, previewId);
  const png = await readFile(filename);
  assert.deepEqual([screenshot.width, screenshot.height], [336, 480]);
  assert.deepEqual([png.readUInt32BE(16), png.readUInt32BE(20)], [336, 480]);
  await expectError('OUTPUT_EXISTS', 'screenshot', '--out', filename);
  await expectError('INVALID_ARGUMENT', 'tap', '336', '0', '--preview', previewId);
  const { entries, cursor } = await agent('logs', '--source', 'runtime');
  assert.deepEqual(entries.filter(({ previewId: id }) => id === previewId).map(({ text }) => text),
    ['演示画面已加载', '倒计时已开始', '倒计时已暂停', '倒计时已重置']);
  assert.deepEqual((await agent('logs', '--since', String(cursor))).entries, []);
  await expectError('TIMEOUT', 'wait', '--after', String(buildId), '--timeout', '200');

  assert.notEqual((await agent('reload')).previewId, previewId);
  await expectError('STALE_PREVIEW', 'tap', '168', '380', '--preview', previewId);
  await stop();
  try {
    // Windows kills the child directly when a signal is sent over child_process.
    if (process.platform !== 'win32') await assert.rejects(() => stat(session), { code: 'ENOENT' });
    await expectError('SESSION_UNAVAILABLE', 'status');
  } finally {
    if (process.platform === 'win32') await rm(join(session, '..'), { recursive: true, force: true });
  }
});

test('Workbench settings independently grant CLI and MCP access to one preview', { timeout: 60_000 }, async (t) => {
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
  assert.equal((await call('wait')).structuredContent.ok, true);
  const snapshot = await call('snapshot');
  const { previewId } = snapshot.structuredContent;
  assert.equal((await call('tap', { x: 168, y: 380, preview: previewId })).structuredContent.previewId, previewId);
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
  assert.equal(tree.nodes.find((node) => node.component === 'TimerState').value, '进行中');

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
