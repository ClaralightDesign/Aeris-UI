// 管理项目场景、WASM 构建及本地预览服务。

import { spawn, spawnSync } from 'node:child_process';
import { appendFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, renameSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { createServer as createHttpServer } from 'node:http';
import { tmpdir } from 'node:os';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { createFavicon } from './native/favicon.mjs';

const root = fileURLToPath(new URL('.', import.meta.url));
if (process.argv.includes('--help')) {
  console.log('用法：cargo aeris-devbox [devbox.json]\n      cargo aeris-devbox agent --help');
  process.exit(0);
}
const [, , entry, action, configArg] = process.argv;
if (entry === 'agent' && action === 'install-browser') {
  const playwright = fileURLToPath(new URL('./node_modules/playwright/cli.js', import.meta.url));
  const result = spawnSync(process.execPath, [playwright, 'install', 'chromium', '--only-shell'], { stdio: 'inherit' });
  if (result.error) console.error(result.error.message);
  process.exit(result.status ?? 1);
}
const agentMode = entry === 'agent' && action === 'serve';
const configInput = agentMode ? configArg : entry;
const demo = configInput === '--demo';
const demoPreview = join(root, 'web/demo.html');
const demoSource = join(root, 'web/demo.ts');
const configPath = resolve(configInput ?? 'devbox.json');
const project = resolve(configPath, '..');
const renderFavicon = createFavicon(join(root, 'web/assets'), project, demo);
const aerisRoot = resolve(process.env.AERIS_DEVBOX_SOURCE_ROOT ?? resolve(root, '..'));
const cacheParents = [resolve(aerisRoot, '..'), project, resolve(project, '..'), resolve(project, '../..'), resolve(project, '../../..')];
const toolCache = (leaf, marker) => cacheParents.map((parent) => resolve(parent, leaf))
  .find((directory) => existsSync(join(directory, marker)));
const defaultCache = toolCache('.cache/aeris-devbox', 'lvgl/lvgl.h')
  ?? toolCache('.cache/aeris-gallery-web', 'lvgl/lvgl.h')
  ?? resolve(project, '.cache/aeris-devbox');
const bundledTools = resolve(defaultCache, 'emsdk/upstream/emscripten');
const existingRust = toolCache('.toolchain', 'rustup');
const target = 'wasm32-unknown-emscripten';
const agentDirectory = mkdtempSync(join(realpathSync(tmpdir()), 'aeris-devbox-'));
// Vite can exit directly on SIGTERM in workbench mode.
if (!agentMode) process.once('exit', () => rmSync(agentDirectory, { recursive: true, force: true }));
const buildRoot = agentMode ? agentDirectory : resolve(project, 'target/aeris-devbox');
const cargoTarget = join(buildRoot, 'cargo');
const native = join(buildRoot, 'native');
const preview = join(buildRoot, 'preview.html');
const storageId = createHash('sha256').update(configPath).digest('hex').slice(0, 16);
const hiddenSourceNames = new Set(['target', 'node_modules', 'dist', 'build', 'vendor', 'Cargo.lock', 'pnpm-lock.yaml', 'package-lock.json', 'yarn.lock']);
const sourceExtensions = new Set(['.rs', '.toml', '.c', '.h', '.cc', '.cpp', '.hpp', '.s', '.ld', '.ts', '.tsx', '.js', '.jsx',
  '.mjs', '.cjs', '.css', '.html', '.svg', '.json', '.yaml', '.yml', '.md', '.mdx', '.txt', '.sh', '.py']);
const maxSourceBytes = 1024 * 1024;

function visibleSourceName(name) {
  return !name.startsWith('.') && !hiddenSourceNames.has(name) && !/[\\:\x00-\x1f]/.test(name);
}

function sourceFileName(name) {
  return visibleSourceName(name) && (name === 'Makefile' || sourceExtensions.has(extname(name).toLowerCase()));
}

function sourcePath(base, path) {
  if (path && path.split('/').some((part) => !part || !visibleSourceName(part))) return;
  const absolute = resolve(base, path);
  const local = relative(base, absolute);
  if (local === '..' || local.startsWith('..' + sep) || isAbsolute(local)) return;
  try {
    if (realpathSync(absolute) === absolute) return absolute;
  } catch (error) {
    if (error.code !== 'ENOENT' && error.code !== 'ENOTDIR') throw error;
  }
}

function configuredSource(settings, sourceRoot, sceneId) {
  const path = settings.scenes?.find(({ id }) => id === sceneId)?.source ?? settings.source;
  return path && relative(sourceRoot, resolve(project, path)).split(sep).join('/');
}

function serveSource(url, res) {
  const json = (data) => {
    res.setHeader('Content-Type', 'application/json; charset=utf-8');
    res.end(JSON.stringify(data));
  };
  const finish = (status) => { res.statusCode = status; res.end(); };
  if (demo) {
    return url.searchParams.get('directory') === '' ? json({ entries: [] }) : finish(404);
  }
  const settings = config();
  const packageRoot = dirname(resolve(project, settings.manifest));
  const sourceRoot = realpathSync(packageRoot);
  const initialSource = () => configuredSource(settings, packageRoot, url.searchParams.get('scene'));
  if (url.searchParams.has('directory')) {
    const directory = url.searchParams.get('directory');
    const absolute = sourcePath(sourceRoot, directory);
    if (!absolute || !statSync(absolute).isDirectory()) return finish(404);
    const entries = readdirSync(absolute, { withFileTypes: true })
      .filter((entry) => entry.isDirectory() ? visibleSourceName(entry.name) : entry.isFile() && sourceFileName(entry.name))
      .map((entry) => ({ name: entry.name, directory: entry.isDirectory() }));
    entries.sort((a, b) => Number(b.directory) - Number(a.directory) || a.name.localeCompare(b.name));
    const initial = directory === '' ? initialSource() : undefined;
    return json({ entries, initial: initial && sourceFileName(initial.split('/').at(-1)) && sourcePath(sourceRoot, initial) ? initial : undefined });
  }
  const file = url.searchParams.get('file') ?? initialSource();
  if (!file || !sourceFileName(file.split('/').at(-1))) return finish(404);
  const absolute = sourcePath(sourceRoot, file);
  if (!absolute) return finish(404);
  const info = statSync(absolute);
  if (!info.isFile()) return finish(404);
  if (info.size > maxSourceBytes) return finish(413);
  const bytes = readFileSync(absolute);
  if (bytes.includes(0)) return finish(415);
  return json({ name: file, text: bytes.toString('utf8') });
}

function config() {
  const data = JSON.parse(readFileSync(configPath, 'utf8'));
  if (typeof data.title !== 'string' || !data.title.trim()
      || typeof data.manifest !== 'string' || !data.manifest.trim()
      || (data.icon !== undefined && (typeof data.icon !== 'string' || !data.icon.trim()))
      || (data.source !== undefined && typeof data.source !== 'string')
      || (data.scenes !== undefined && (!Array.isArray(data.scenes) || !data.scenes.length
        || data.scenes.some((scene) => typeof scene?.id !== 'string' || !/^[a-z0-9][a-z0-9_-]{0,126}$/.test(scene.id)
          || typeof scene.title !== 'string' || !scene.title.trim()
          || (scene.source !== undefined && typeof scene.source !== 'string'))
        || new Set(data.scenes.map(({ id }) => id)).size !== data.scenes.length))
      || (data.features !== undefined && (!Array.isArray(data.features) || data.features.some((feature) => typeof feature !== 'string')))
      || (data.watch !== undefined && (!Array.isArray(data.watch) || data.watch.some((path) => typeof path !== 'string')))) {
    throw new Error('devbox.json 需要 title 和 manifest，场景使用唯一的 id 和 title。');
  }
  return data;
}

function resolveTools() {
  const lvgl = resolve(process.env.LVGL_DIR ?? join(defaultCache, 'lvgl'));
  if (!existsSync(join(lvgl, 'lvgl.h'))) {
    throw new Error('请将 LVGL_DIR 指向 LVGL v9.4.0 的源码目录。');
  }
  const emcc = process.env.EMCC ?? (existsSync(join(bundledTools, 'emcc')) ? join(bundledTools, 'emcc') : 'emcc');
  const emar = process.env.EMAR ?? (emcc.includes(sep) ? resolve(emcc, '../emar') : 'emar');
  const env = { ...process.env };
  if (emcc === join(bundledTools, 'emcc') && !env.EMSDK_CONFIG) {
    env.EMSDK_CONFIG = join(defaultCache, 'emsdk/.emscripten');
  }
  if (!env.RUSTUP_HOME && existingRust) env.RUSTUP_HOME = join(existingRust, 'rustup');
  if (!env.CARGO_HOME && existingRust && existsSync(join(existingRust, 'cargo'))) env.CARGO_HOME = join(existingRust, 'cargo');
  const cargo = env.CARGO ?? 'cargo';
  // 预览使用自己的目标目录和链接配置，允许从固件项目的根目录启动。
  delete env.CARGO_ENCODED_RUSTFLAGS;
  delete env.RUSTFLAGS;
  const info = spawnSync(emcc, ['--version'], { env, encoding: 'utf8' });
  if (info.status !== 0) throw new Error('请激活 Emscripten SDK 4.0.20，或设置 EMCC 和 EMAR。');
  const compiler = createHash('sha256').update(JSON.stringify([emcc, emar, lvgl, info.stdout, env.EMSDK_CONFIG])).digest('hex').slice(0, 12);
  return { lvgl, emcc, emar, env, cargo, compiler };
}

let child;
let stopped = false;
let server;
let httpServer;
let agent;
let buildStatus = existsSync(preview) ? 'ready' : 'building';
let history = [];
let buildId = 0;
let publishedBuildId = null;
let queuedBuildId = null;
let queuedClean = false;
let queuedRefresh = false;
let agentReady = false;
let active = false;
let debounce;
let publishedProject;

function projectInfo(settings) {
  return { title: settings.title, scenes: (settings.scenes ?? []).map(({ id, title }) => ({ id, title })) };
}

function agentBuildState() {
  return {
    ...publishedProject,
    state: queuedBuildId !== null ? 'queued' : publishedBuildId === null && buildStatus === 'ready' ? 'building' : buildStatus,
    buildId,
    publishedBuildId,
    queuedBuildId,
  };
}

function log(level, text) {
  const entry = { level, text };
  history = [...history.slice(-499), entry];
  agent?.log('build', buildId, undefined, level, text);
  server?.ws.send('aeris:log', entry);
  process.stderr.write(text);
}

function updateStatus(status, refresh = false) {
  buildStatus = status;
  server?.ws.send('aeris:build', { status, refresh });
}

async function run(command, args, options, capture = false) {
  if (stopped) throw new Error('DevBox 已退出。');
  return new Promise((done, fail) => {
    let stdout = '';
    const process = spawn(command, args, {
      ...options, detached: globalThis.process.platform !== 'win32', stdio: ['ignore', 'pipe', 'pipe'],
    });
    child = process;
    if (capture) {
      process.stdout.setEncoding('utf8');
      process.stdout.on('data', (chunk) => { stdout += chunk; });
    } else {
      createInterface({ input: process.stdout, crlfDelay: Infinity }).on('line', (line) => log('log', line + '\n'));
    }
    createInterface({ input: process.stderr, crlfDelay: Infinity }).on('line', (line) => log('log', line + '\n'));
    process.once('error', fail);
    process.once('close', (code) => {
      if (child === process) child = undefined;
      if (code === 0) done(stdout);
      else fail(new Error(command + ' 构建失败。'));
    });
  });
}

const watcherPaths = new Set([configPath, join(aerisRoot, 'crates')]);
function watch(paths) {
  paths.forEach((path) => watcherPaths.add(path));
  server?.watcher.add(paths);
}

async function build() {
  if (demo) return { title: 'DevBox Demo', scenes: [] };
  const settings = config();
  const manifest = resolve(project, settings.manifest);
  watch([manifest, resolve(manifest, '../src'), ...(settings.watch ?? []).map((path) => resolve(project, path))]);
  const { lvgl, emcc, emar, env, cargo, compiler } = resolveTools();
  const cargoEnvironment = {
    ...env,
    CARGO_TARGET_DIR: cargoTarget,
    CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS: '-C panic=abort',
  };
  const features = settings.features?.length ? ['--features', settings.features.join(',')] : [];
  const metadata = JSON.parse(await run(cargo,
    ['metadata', ...features, '--format-version', '1', '--filter-platform', target, '--manifest-path', manifest],
    { cwd: root, env: cargoEnvironment }, true));
  const packageInfo = metadata.packages.find(({ manifest_path }) => manifest_path === manifest);
  const library = packageInfo?.targets.find(({ crate_types }) => crate_types.includes('staticlib'));
  if (!library) throw new Error('预览 Cargo.toml 需要声明 [lib] crate-type = ["staticlib"]。');
  for (const dependency of metadata.packages.filter(({ source }) => source === null)) {
    const folder = resolve(dependency.manifest_path, '..');
    watch([dependency.manifest_path, join(folder, 'src'), ...dependency.targets.map(({ src_path }) => src_path)]);
  }

  const nativeBuild = join(native, compiler);
  mkdirSync(nativeBuild, { recursive: true });
  const link = join(nativeBuild, 'lvgl');
  if (!existsSync(link)) symlinkSync(lvgl, link, 'dir');
  for (const name of ['Makefile', 'browser.c']) {
    const from = join(root, 'native', name);
    const to = join(nativeBuild, name);
    if (!existsSync(to) || !readFileSync(from).equals(readFileSync(to))) cpSync(from, to);
  }
  await run(cargo, ['build', ...features, '--lib', '--release', '--target', target, '--manifest-path', manifest],
    { cwd: root, env: cargoEnvironment });
  await run('make', ['-s', '-j', process.env.AERIS_BUILD_JOBS ?? '8', 'EMCC=' + emcc, 'EMAR=' + emar],
    { cwd: nativeBuild, env });

  const staging = join(buildRoot, 'preview.tmp.html');
  try {
    await run(emcc, ['-Os', '-flto', '-sSINGLE_FILE=1', '-sEXPORTED_RUNTIME_METHODS=UTF8ToString', '-sALLOW_MEMORY_GROWTH=1', '-sSTACK_SIZE=131072',
      '--shell-file', join(root, 'native/shell.html'), 'browser.o',
      join(cargoEnvironment.CARGO_TARGET_DIR, target, 'release', 'lib' + library.name.replaceAll('-', '_') + '.a'),
      'liblvgl.a', '-o', staging],
    { cwd: nativeBuild, env });
    const license = [join(aerisRoot, 'LICENSE'), join(lvgl, 'LICENCE.txt'),
      join(aerisRoot, 'crates/aeris-ui/fonts/OFL.txt')].map((path) => readFileSync(path, 'utf8')).join('\n');
    const html = readFileSync(staging, 'utf8');
    writeFileSync(staging, html.replace('<head>', '<head><meta name="aeris-storage" content="' + storageId + '">'));
    appendFileSync(staging, '\n<!--\n' + license + '\n-->\n');
    renameSync(staging, preview);
  } finally {
    rmSync(staging, { force: true });
  }
  return projectInfo(settings);
}

async function runQueued() {
  if (stopped || active || queuedBuildId === null) return;
  active = true;
  buildId = queuedBuildId;
  const clean = queuedClean;
  const refresh = queuedRefresh;
  queuedBuildId = null;
  queuedClean = false;
  queuedRefresh = false;
  updateStatus('building');
  try {
    if (clean && !demo) {
      rmSync(cargoTarget, { recursive: true, force: true });
      rmSync(native, { recursive: true, force: true });
      log('info', '已清理 DevBox 构建缓存，开始重新构建。\n');
    }
    publishedProject = await build();
    if (!stopped) {
      publishedBuildId = buildId;
      updateStatus('ready', refresh);
    }
  } catch (error) {
    if (!stopped) {
      log('error', error.message + '\n');
      updateStatus('failed', refresh);
    }
  } finally {
    active = false;
    if (queuedBuildId !== null && !stopped && !debounce) void runQueued();
  }
}

function queue({ clean = false, refresh = false } = {}) {
  queuedBuildId ??= buildId + 1;
  queuedClean ||= clean;
  queuedRefresh ||= refresh;
  clearTimeout(debounce);
  debounce = setTimeout(() => {
    debounce = undefined;
    void runQueued();
  }, 180);
}

async function serve(req, res, next) {
  const url = new URL(req.url, 'http://localhost');
  if (url.pathname.startsWith('/devbox-')) res.setHeader('Cache-Control', 'no-store');
  try {
    if (['/devbox-agent', '/devbox-agent-preview', '/devbox-agent-settings', '/mcp'].includes(url.pathname)) {
      const port = (httpServer ?? server.httpServer).address()?.port;
      if (!agentReady || !['127.0.0.1:' + port, 'localhost:' + port].includes(req.headers.host)) {
        res.writeHead(403).end();
        return;
      }
      await agent.serve(url.pathname, req, res);
    } else if (url.pathname === '/devbox-build') {
      if (req.method !== 'POST') {
        res.writeHead(405, { Allow: 'POST' }).end();
        return;
      }
      const port = (httpServer ?? server.httpServer).address()?.port;
      const host = req.headers.host;
      if (agentMode || !['127.0.0.1:' + port, 'localhost:' + port].includes(host)
          || req.headers.origin !== 'http://' + host) {
        res.writeHead(403).end();
        return;
      }
      const mode = url.searchParams.get('mode');
      if (mode !== 'rebuild' && mode !== 'clean') {
        res.writeHead(400).end();
        return;
      }
      queue({ clean: mode === 'clean', refresh: true });
      res.writeHead(202).end();
    } else if (url.pathname === '/devbox-favicon.png') {
      const icon = await renderFavicon(demo ? undefined : config());
      res.writeHead(200, { 'Content-Type': 'image/png' }).end(icon);
    } else if (url.pathname === '/devbox-project.json') {
      res.setHeader('Content-Type', 'application/json; charset=utf-8');
      if (demo) {
        res.end(JSON.stringify({ title: 'DevBox Demo', scenes: [], status: buildStatus, logs: history, hasPreview: true, demo }));
        return;
      }
      const { title, scenes } = projectInfo(config());
      res.end(JSON.stringify({ title, scenes,
        status: buildStatus, logs: history, hasPreview: existsSync(preview) }));
    } else if (url.pathname === '/devbox-source') {
      if (!/^(localhost|127\.0\.0\.1)(:\d+)?$/i.test(req.headers.host ?? '')) {
        res.statusCode = 403;
        res.end();
        return;
      }
      serveSource(url, res);
    } else if (url.pathname === '/preview.html') {
      if (agent && url.searchParams.has('agentBuild') && (url.searchParams.get('agentBuild') !== String(publishedBuildId)
          || agentBuildState().state !== 'ready')) {
        res.writeHead(409).end();
        return;
      }
      if (demo) {
        const template = readFileSync(demoPreview, 'utf8');
        const html = agentMode || url.searchParams.has('agentBuild') ? template : await server.transformIndexHtml(req.url, template);
        res.setHeader('Content-Type', 'text/html; charset=utf-8');
        res.setHeader('Cache-Control', 'no-store');
        res.end(html);
        return;
      }
      if (!existsSync(preview)) {
        res.statusCode = 503;
        res.end();
        return;
      }
      res.setHeader('Content-Type', 'text/html; charset=utf-8');
      res.setHeader('Cache-Control', 'no-store');
      res.end(readFileSync(preview));
    } else next();
  } catch {
    res.statusCode = 500;
    res.end();
  }
}

async function shutdown() {
  if (stopped) return;
  stopped = true;
  clearTimeout(debounce);
  const current = child;
  const closed = Promise.allSettled([agent?.close(), server?.close(), httpServer && new Promise((done) => {
    httpServer.close(done);
    httpServer.closeAllConnections();
  })]);
  if (current?.pid !== undefined) {
    const exited = new Promise((done) => current.once('close', done));
    function kill(signal) {
      try {
        if (process.platform === 'win32') current.kill(signal);
        else process.kill(-current.pid, signal);
      } catch { /* 构建进程已退出。 */ }
    }
    kill('SIGTERM');
    const force = setTimeout(() => kill('SIGKILL'), 3000);
    await exited;
    clearTimeout(force);
  }
  await closed;
  rmSync(agentDirectory, { recursive: true, force: true });
}

try {
  publishedProject = demo ? { title: 'DevBox Demo', scenes: [] } : projectInfo(config());
  const { createAgent } = await import('./agent.mjs');
  agent = createAgent({ buildState: agentBuildState, directory: agentDirectory });
  if (agentMode) httpServer = createHttpServer();
  if (demo) {
    buildStatus = 'ready';
    buildId = publishedBuildId = 1;
  }
  const port = agentMode ? 0 : Number(process.env.AERIS_DEVBOX_PORT ?? 5173);
  if (!Number.isInteger(port) || port < 0 || port > 65535) throw new Error('AERIS_DEVBOX_PORT 需要有效的端口号。');
  server = await createServer({ root, cacheDir: demo && !agentMode ? undefined : join(buildRoot, 'vite'),
    server: { host: '127.0.0.1', port, ...(agentMode && { middlewareMode: true, hmr: false, ws: false }) }, plugins: [{
    name: 'aeris-devbox',
    hotUpdate({ type, file, modules }) {
      if (!demo || this.environment.name !== 'client' || type !== 'update' || file !== demoSource) return;
      for (const module of modules) this.environment.moduleGraph.invalidateModule(module);
      return [];
    },
    configureServer(vite) {
      vite.middlewares.use(serve);
      if (demo) watcherPaths.add(demoSource);
      vite.watcher.add([...watcherPaths]);
      vite.watcher.on('all', (event, path) => {
        if (!['add', 'change', 'unlink'].includes(event)) return;
        if (demo ? path === demoSource : [...watcherPaths].some((entry) => path === entry || path.startsWith(entry + sep))) queue();
      });
    },
  }] });
  process.once('SIGINT', shutdown);
  process.prependOnceListener('SIGTERM', shutdown);
  if (process.platform !== 'win32') process.once('SIGHUP', shutdown);
  if (agentMode) {
    httpServer.on('request', server.middlewares);
    httpServer.listen(0, '127.0.0.1');
    await once(httpServer, 'listening');
  } else {
    await server.listen();
    server.printUrls();
  }
  if (!demo) queue();
  agent.start((httpServer ?? server.httpServer).address().port, agentMode);
  agentReady = true;
} catch (error) {
  console.error(error.message);
  await shutdown();
  process.exitCode = 1;
}
