import { randomBytes, timingSafeEqual } from 'node:crypto';
import { rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const protocol = 1;
const timeout = 10_000;
const success = (result) => ({ version: protocol, ok: true, ...result });
const error = (code, message, details) => Object.assign(new Error(message), { code, details });
const nonnegativeInteger = (value) => Number.isSafeInteger(value) && value >= 0;

export function createAgent({ buildState, directory }) {
  const session = join(directory, 'session.json');
  const access = { cli: undefined, mcp: undefined };
  const entries = [];
  let cursor = 0;
  let url;
  let browser;
  let launch;
  const previews = new Map();
  let stopped = false;

  function deviceFor(id) {
    const { devices } = buildState();
    const device = id === undefined ? devices[0] : devices.find((entry) => entry.id === id);
    if (!device) throw error('INVALID_ARGUMENT', '设备不存在。');
    return device;
  }

  function deviceState(id) {
    if (!previews.has(id)) previews.set(id, { operations: Promise.resolve() });
    return previews.get(id);
  }

  function start(port, cliEnabled) {
    url = 'http://127.0.0.1:' + port;
    if (cliEnabled) {
      setAccess('cli', true);
      console.log(JSON.stringify(success({ session, url })));
    }
  }

  function setAccess(kind, enabled) {
    if (Boolean(access[kind]) === enabled) return;
    if (!enabled) {
      const { controller } = access[kind];
      access[kind] = undefined;
      controller.abort();
      if (kind === 'cli') rmSync(session, { force: true });
      return;
    }
    const token = randomBytes(32).toString('hex');
    if (kind === 'cli') {
      writeFileSync(session, JSON.stringify({ version: protocol, url, token }) + '\n', { mode: 0o600, flag: 'wx' });
    }
    access[kind] = { token, controller: new AbortController() };
  }

  function settings() {
    return {
      cli: Boolean(access.cli),
      mcp: Boolean(access.mcp),
      cliSession: access.cli ? session : undefined,
      mcpConfig: access.mcp ? {
        mcpServers: { 'aeris-devbox': { url: url + '/mcp', headers: { Authorization: 'Bearer ' + access.mcp.token } } },
      } : undefined,
    };
  }

  function log(source, buildId, previewId, level, text, device) {
    entries.push({ cursor: ++cursor, source, buildId, previewId, level, text: String(text).slice(0, 16_384), device });
    if (entries.length > 500) entries.shift();
  }

  function status(device = deviceFor()) {
    const { title, scenes, devices, state, buildId, publishedBuildId, queuedBuildId } = buildState();
    const { current, failure, openingBuildId, sceneId } = deviceState(device.id);
    const preview = current?.buildId === publishedBuildId ? current : undefined;
    const failed = failure?.buildId === publishedBuildId ? failure : undefined;
    return {
      title, scenes, devices, device: device.id,
      scene: scenes.some(({ id }) => id === sceneId) ? sceneId : scenes[0]?.id ?? null,
      build: { state, buildId, publishedBuildId, queuedBuildId },
      preview: {
        state: preview ? 'ready' : failed ? 'failed' : openingBuildId === publishedBuildId ? 'loading' : 'idle',
        previewId: preview?.id ?? null, buildId: preview?.buildId ?? null,
        error: failed && { code: failed.error.code, message: failed.error.message },
      },
    };
  }

  function assertBuild(buildId) {
    const { state, buildId: latest } = buildState();
    if (stopped) throw error('SESSION_UNAVAILABLE', 'DevBox 会话已关闭。');
    if (state === 'failed') throw error('BUILD_FAILED', '最新构建失败，请查看构建日志。', status());
    if (state !== 'ready' || buildId !== latest) {
      throw error('BUILD_PENDING', '有新的构建等待完成。', status());
    }
  }

  async function launchBrowser() {
    const { chromium } = await import('playwright');
    try {
      browser = await chromium.launch({ timeout, handleSIGINT: false, handleSIGTERM: false, handleSIGHUP: false });
    } catch (cause) {
      if (/Executable doesn't exist/i.test(cause.message)) {
        throw error('BROWSER_NOT_INSTALLED', '未安装匹配的 Chromium，请运行 agent install-browser。');
      }
      if (cause.name === 'TimeoutError') throw error('TIMEOUT', '启动 Chromium 超时。');
      throw error('PREVIEW_FAILED', '无法启动 Chromium。', { reason: cause.message });
    }
    browser.once('disconnected', () => {
      launch = undefined;
      for (const state of previews.values()) {
        state.context = undefined;
        if (!state.current) continue;
        const { buildId, scene } = state.current;
        state.failure = { buildId, scene, error: error('PREVIEW_FAILED', 'Chromium 已退出。') };
        state.current = undefined;
      }
    });
    return browser;
  }

  async function getContext(device, state) {
    if (!launch) launch = launchBrowser().catch((cause) => { launch = undefined; throw cause; });
    const running = await launch;
    return state.context ??= await running.newContext({ viewport: { width: device.width, height: device.height } });
  }

  async function load(device, state, buildId, scene, force) {
    assertBuild(buildId);
    const { current, failure } = state;
    if (!force && current?.buildId === buildId && current.scene === scene) return current;
    if (!force && failure?.buildId === buildId && failure.scene === scene) throw failure.error;
    state.current = undefined;
    state.failure = undefined;
    const id = randomBytes(12).toString('hex');
    state.openingBuildId = buildId;
    let page;
    let runtimeFailed = false;
    function previewFailed(message) {
      runtimeFailed = true;
      log('runtime', buildId, id, 'error', message, device.id);
      if (state.current?.id !== id) return;
      state.current = undefined;
      state.failure = { buildId, scene, error: error('PREVIEW_FAILED', message) };
      void page.close().catch(() => {});
    }
    try {
      await current?.page.close();
      assertBuild(buildId);
      page = await (await getContext(device, state)).newPage();
      page.setDefaultTimeout(timeout);
      await page.exposeFunction('devboxAgentEvent', (level, text) => {
        if (level === null) {
          previewFailed('应用预览已停止，请查看运行日志。');
        } else if (typeof level === 'string' && typeof text === 'string') {
          log('runtime', buildId, id, level, text, device.id);
        }
      });
      await page.addInitScript(() => {
        if (window !== window.top) return;
        window.addEventListener('message', (event) => {
          if (event.origin !== location.origin || event.source !== document.querySelector('iframe')?.contentWindow) return;
          const data = event.data;
          if (data?.type === 'aeris-debug-log' && typeof data.level === 'string' && typeof data.text === 'string') {
            void window.devboxAgentEvent(data.level, data.text);
          } else if (data?.type === 'aeris-debug-error') {
            void window.devboxAgentEvent(null);
          }
        });
      });
      page.on('pageerror', (cause) => log('runtime', buildId, id, 'error', cause.message, device.id));
      page.once('crash', () => previewFailed('预览页面崩溃。'));
      const params = new URLSearchParams({ scene: scene ?? '', device: device.id, debug: '1', agentBuild: String(buildId) });
      await page.goto(url + '/devbox-agent-preview?' + params, { waitUntil: 'domcontentloaded', timeout });
      const iframe = await page.locator('iframe').elementHandle();
      const frame = await iframe.contentFrame();
      if (!frame) throw error('PREVIEW_FAILED', '无法打开预览框架。');
      await frame.waitForURL((address) => address.pathname === '/preview.html', { timeout });
      const ready = await frame.waitForFunction(() => {
        if (document.querySelector('#error:not([hidden])')) return 'failed';
        return window.aerisDebugger?.snapshot() ? 'ready' : false;
      }, undefined, { timeout });
      const readyState = await ready.jsonValue();
      await ready.dispose();
      if (readyState !== 'ready' || runtimeFailed) throw error('PREVIEW_FAILED', '应用预览未能启动，请查看运行日志。');
      assertBuild(buildId);
      state.current = { id, buildId, page, frame, scene, device: device.id };
      return state.current;
    } catch (cause) {
      await page?.close().catch(() => {});
      assertBuild(buildId);
      const problem = typeof cause.code === 'string' ? cause
        : cause.name === 'TimeoutError' ? error('TIMEOUT', '等待预览首帧超时，请查看运行日志。')
          : error('PREVIEW_FAILED', '无法加载应用预览。', { reason: cause.message });
      if (problem.code !== 'BROWSER_NOT_INSTALLED') state.failure = { buildId, scene, error: problem };
      log('runtime', buildId, id, 'error', problem.message, device.id);
      throw problem;
    } finally {
      state.openingBuildId = undefined;
    }
  }

  function serialize(state, operation) {
    const result = state.operations.then(operation);
    state.operations = result.catch(() => {});
    return result;
  }

  async function observe(preview, screenshot) {
    const result = await preview.frame.evaluate((capture) => {
      const snapshot = window.aerisDebugger.snapshot();
      const width = document.querySelector('canvas').width;
      if (!capture) return { width, ...snapshot };
      return { width, height: snapshot.height, png: window.aerisDebugger.screenshot().split(',')[1] };
    }, screenshot);
    assertBuild(preview.buildId);
    return { ...result, device: preview.device, buildId: preview.buildId, previewId: preview.id };
  }

  async function input(preview, command, points, signal) {
    const canvas = preview.frame.locator('canvas');
    if (command === 'back') {
      await canvas.evaluate((element) => {
        if (!element.hasAttribute('tabindex')) element.tabIndex = -1;
        element.focus();
      });
      assertBuild(preview.buildId);
      signal.throwIfAborted();
      await preview.page.keyboard.press('Escape');
    } else {
      const { width, height } = await observe(preview, false);
      const count = command === 'tap' ? 2 : 4;
      if (!Array.isArray(points) || points.length !== count || points.some((point, index) =>
        !nonnegativeInteger(point) || point >= (index % 2 === 0 ? width : height))) {
        throw error('INVALID_ARGUMENT', '坐标超出设备画布范围。', { width, height });
      }
      const hitsDisplay = await preview.frame.evaluate((coordinates) => {
        const contains = window.aerisDebugger.contains;
        return !contains || contains(coordinates[0], coordinates[1])
          && (coordinates.length === 2 || contains(coordinates[2], coordinates[3]));
      }, points);
      if (!hitsDisplay) throw error('INVALID_ARGUMENT', '坐标超出设备屏幕轮廓。', { width, height });
      const bounds = await canvas.boundingBox();
      if (!bounds) throw error('PREVIEW_FAILED', '预览画布不可见。');
      const scaleX = bounds.width / width;
      const canvasHeight = await canvas.evaluate((element) => element.height);
      const scaleY = bounds.height / canvasHeight;
      const [x1, y1, x2, y2] = points;
      const move = (x, y) => preview.page.mouse.move(bounds.x + (x + 0.5) * scaleX, bounds.y + (y + 0.5) * scaleY);
      const mouse = preview.page.mouse;
      await move(x1, y1);
      assertBuild(preview.buildId);
      signal.throwIfAborted();
      await mouse.down();
      try {
        if (command === 'swipe') {
          for (let step = 1; step <= 12; step++) {
            await delay(20, undefined, { signal });
            await move(x1 + (x2 - x1) * step / 12, y1 + (y2 - y1) * step / 12);
          }
        }
      } finally {
        await mouse.up();
      }
    }
    // Let LVGL paint the result before the next command can capture it.
    await preview.frame.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    assertBuild(preview.buildId);
    return { device: preview.device, buildId: preview.buildId, previewId: preview.id };
  }

  async function execute(request, signal) {
    signal.throwIfAborted();
    const { command } = request;
    const device = deviceFor(request.device);
    const state = deviceState(device.id);
    if (command === 'status') return status(device);
    if (command === 'logs') {
      const { since = 0, source } = request;
      if (!nonnegativeInteger(since) || (source !== undefined && !['build', 'runtime'].includes(source))) {
        throw error('INVALID_ARGUMENT', '无效的日志查询。');
      }
      return { cursor, truncated: since < (entries[0]?.cursor ?? 1) - 1,
        entries: entries.filter((entry) => entry.cursor > since && (!source || entry.source === source)
          && (!entry.device || entry.device === device.id)) };
    }
    const after = request.after ?? 0;
    if (command === 'wait' && !nonnegativeInteger(after)) throw error('INVALID_ARGUMENT', '无效的构建编号。');
    const pending = ({ state, buildId }) => (state !== 'ready' && state !== 'failed') || buildId <= after;
    if (command === 'wait' && pending(buildState())) return { ...status(device), pending: true };
    return serialize(state, async () => {
      signal.throwIfAborted();
      const build = buildState();
      if (command === 'wait' && pending(build)) return { ...status(device), pending: true };
      const { buildId, scenes } = build;
      assertBuild(buildId);
      const { current, sceneId } = state;
      let scene = scenes.some(({ id }) => id === sceneId) ? sceneId : scenes[0]?.id;
      if (['tap', 'swipe', 'back'].includes(command)) {
        if (!current || current.buildId !== buildId || current.id !== request.preview) {
          throw error('STALE_PREVIEW', '预览已更换，请重新读取对象树或截图。', status(device));
        }
        return input(current, command, request.points, signal);
      }
      if (command === 'scene') {
        if (!scenes.some(({ id }) => id === request.scene)) throw error('INVALID_ARGUMENT', '场景不存在。');
        scene = request.scene;
      }
      if (!['wait', 'snapshot', 'screenshot', 'scene', 'reload'].includes(command)) {
        throw error('INVALID_ARGUMENT', '未知命令。');
      }
      state.sceneId = scene;
      let preview;
      try {
        preview = await load(device, state, buildId, scene, command === 'scene' || command === 'reload');
      } catch (cause) {
        if (command === 'wait' && cause.code === 'BUILD_PENDING') return { ...status(device), pending: true };
        throw cause;
      }
      signal.throwIfAborted();
      if (command === 'snapshot' || command === 'screenshot') return observe(preview, command === 'screenshot');
      return { ...status(device), buildId, previewId: preview.id, ...(command === 'wait' && { pending: false }) };
    });
  }

  async function serve(path, req, res) {
    if (path === '/devbox-agent-preview' && req.method === 'GET') {
      const address = new URL(req.url, url);
      let device;
      try { device = deviceFor(address.searchParams.get('device') ?? undefined); }
      catch { res.writeHead(404).end(); return; }
      const src = '/preview.html' + address.search.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;');
      res.setHeader('Content-Type', 'text/html; charset=utf-8');
      res.setHeader('Cache-Control', 'no-store');
      res.end('<!doctype html><style>body{margin:0}iframe{border:0;display:block}</style>'
        + `<iframe width="${device.width}" height="${device.height}" title="DevBox Preview" src="${src}"></iframe>`);
      return;
    }
    res.setHeader('Content-Type', 'application/json; charset=utf-8');
    res.setHeader('Cache-Control', 'no-store');
    const isMcp = path === '/mcp';
    const isSettings = path === '/devbox-agent-settings';
    const json = (value) => res.end(JSON.stringify(value));
    const forbid = () => {
      res.statusCode = 403;
      json(isMcp ? { jsonrpc: '2.0', id: null, error: { code: -32000, message: 'MCP 未开启或会话认证失败。' } }
        : { version: protocol, ok: false, error: { code: 'FORBIDDEN', message: '会话认证失败。' } });
    };
    const requestOrigin = 'http://' + req.headers.host;
    if (isSettings) {
      // Browser-controlled headers prevent other origins from reading the credentials or changing access.
      if (req.headers['sec-fetch-site'] !== 'same-origin'
          || (req.headers.origin && req.headers.origin !== requestOrigin)
          || !['GET', 'POST'].includes(req.method)) return forbid();
      if (req.method === 'GET') return json(settings());
      if (req.headers.origin !== requestOrigin) return forbid();
      try {
        const { kind, enabled } = await readRequest(req) ?? {};
        if (!Object.hasOwn(access, kind) || typeof enabled !== 'boolean') {
          res.writeHead(400).end();
          return;
        }
        setAccess(kind, enabled);
        json(settings());
      } catch {
        res.writeHead(500).end();
      }
      return;
    }
    const credential = access[isMcp ? 'mcp' : 'cli'];
    const auth = Buffer.from(req.headers.authorization ?? '');
    const authorization = Buffer.from('Bearer ' + (credential?.token ?? ''));
    if (!credential || (req.headers.origin && (!isMcp || req.headers.origin !== requestOrigin))
        || auth.length !== authorization.length || !timingSafeEqual(auth, authorization)) return forbid();
    if (!isMcp && req.method !== 'POST') return forbid();

    const disconnect = new AbortController();
    res.once('close', () => disconnect.abort());
    const signal = AbortSignal.any([disconnect.signal, credential.controller.signal]);
    try {
      const request = req.method === 'POST' ? await readRequest(req) : undefined;
      if (isMcp) {
        const { serveMcp } = await import('./agent-mcp.mjs');
        if (!res.destroyed) await serveMcp(req, res, request, execute, signal);
        return;
      }
      if (typeof request?.command !== 'string') throw error('INVALID_ARGUMENT', '请求格式错误。');
      const result = await execute(request, signal);
      signal.throwIfAborted();
      if (!res.destroyed) json(success(result));
    } catch (cause) {
      if (res.destroyed || res.headersSent) return;
      if (isMcp) {
        const invalid = cause.code === 'INVALID_ARGUMENT';
        res.statusCode = invalid ? 400 : 500;
        json({ jsonrpc: '2.0', id: null, error: { code: invalid ? -32700 : -32603, message: cause.message } });
        return;
      }
      const disabled = credential.controller.signal.aborted;
      const code = disabled ? 'FORBIDDEN' : typeof cause.code === 'string' ? cause.code
        : cause.name === 'TimeoutError' ? 'TIMEOUT' : 'PREVIEW_FAILED';
      json({ version: protocol, ok: false,
        error: { code, message: disabled ? 'Agent CLI 已关闭。' : cause.message, details: cause.details } });
    }
  }

  async function close() {
    stopped = true;
    for (const kind of Object.keys(access)) access[kind]?.controller.abort();
    if (access.cli) rmSync(session, { force: true });
    try {
      await browser?.close();
    } finally {
      await Promise.all([...previews.values()].map(({ operations }) => operations));
      // A launch already in progress may have created the browser after close began.
      await browser?.close();
    }
  }

  return { start, close, serve, log };
}

async function readRequest(req) {
  if (req.headers['content-type']?.split(';', 1)[0].trim().toLowerCase() !== 'application/json') {
    throw error('INVALID_ARGUMENT', '需要 application/json 请求。');
  }
  let body = '';
  req.setEncoding('utf8');
  for await (const chunk of req) {
    body += chunk;
    if (body.length > 8192) throw error('INVALID_ARGUMENT', '请求过大。');
  }
  try {
    return JSON.parse(body);
  } catch {
    throw error('INVALID_ARGUMENT', '请求格式错误。');
  }
}
