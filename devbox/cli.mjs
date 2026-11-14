// 管理项目场景、WASM 构建及本地预览服务。

import { spawn, spawnSync } from 'node:child_process';
import { appendFileSync, cpSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';

const root = fileURLToPath(new URL('.', import.meta.url));
if (process.argv.includes('--help')) {
  console.log('用法：cargo aeris-devbox [devbox.json]');
  process.exit(0);
}
const configPath = resolve(process.argv[2] ?? 'devbox.json');
const project = resolve(configPath, '..');
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
const buildRoot = resolve(project, 'target/aeris-devbox');
const native = join(buildRoot, 'native');
const preview = join(buildRoot, 'preview.html');
const storageId = createHash('sha256').update(configPath).digest('hex').slice(0, 16);

function config() {
  const data = JSON.parse(readFileSync(configPath, 'utf8'));
  if (typeof data.title !== 'string' || !data.title.trim()
      || typeof data.manifest !== 'string' || !data.manifest.trim()
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
let buildStatus = existsSync(preview) ? 'ready' : 'building';
let history = [];
let queued = false;
let active = false;
let debounce;

function log(level, text) {
  const entry = { level, text };
  history = [...history.slice(-499), entry];
  server?.ws.send('aeris:log', entry);
  process.stderr.write(text);
}

function updateStatus(status) {
  buildStatus = status;
  server?.ws.send('aeris:build', { status });
}

async function run(command, args, options, capture = false) {
  if (stopped) throw new Error('DevBox 已退出。');
  return new Promise((done, fail) => {
    let stdout = '';
    const process = spawn(command, args, {
      ...options, detached: globalThis.process.platform !== 'win32', stdio: ['ignore', 'pipe', 'pipe'],
    });
    child = process;
    process.stdout.on('data', (chunk) => {
      if (capture) stdout += chunk;
      else log('info', chunk.toString());
    });
    process.stderr.on('data', (chunk) => log('info', chunk.toString()));
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
  const settings = config();
  const manifest = resolve(project, settings.manifest);
  watch([manifest, resolve(manifest, '../src'), ...(settings.watch ?? []).map((path) => resolve(project, path))]);
  const { lvgl, emcc, emar, env, cargo, compiler } = resolveTools();
  const cargoEnvironment = {
    ...env,
    CARGO_TARGET_DIR: join(buildRoot, 'cargo'),
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
}

async function runQueued() {
  if (stopped || active || !queued) return;
  active = true;
  queued = false;
  updateStatus('building');
  try {
    await build();
    if (!stopped) updateStatus('ready');
  } catch (error) {
    if (!stopped) {
      log('error', error.message + '\n');
      updateStatus('failed');
    }
  } finally {
    active = false;
    if (queued && !stopped) void runQueued();
  }
}

function queue() {
  queued = true;
  clearTimeout(debounce);
  debounce = setTimeout(runQueued, 180);
}

function serve(req, res, next) {
  const url = new URL(req.url, 'http://localhost');
  if (url.pathname.startsWith('/devbox-')) res.setHeader('Cache-Control', 'no-store');
  try {
    if (url.pathname === '/devbox-project.json') {
      const { title, scenes = [] } = config();
      res.setHeader('Content-Type', 'application/json; charset=utf-8');
      res.end(JSON.stringify({ title, scenes: scenes.map(({ id, title }) => ({ id, title })),
        status: buildStatus, logs: history, hasPreview: existsSync(preview) }));
    } else if (url.pathname === '/devbox-source') {
      const settings = config();
      const entry = settings.scenes?.find(({ id }) => id === url.searchParams.get('scene')) ?? settings;
      if (!entry?.source) {
        res.statusCode = 404;
        res.end();
        return;
      }
      res.setHeader('Content-Type', 'application/json; charset=utf-8');
      res.end(JSON.stringify({ name: entry.source, text: readFileSync(resolve(project, entry.source), 'utf8') }));
    } else if (url.pathname === '/preview.html') {
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
  if (current?.pid !== undefined) {
    const exited = new Promise((done) => current.once('close', done));
    try {
      if (process.platform === 'win32') current.kill();
      else process.kill(-current.pid, 'SIGTERM');
    } catch { /* 构建进程已退出。 */ }
    await exited;
  }
  await server?.close();
}

try {
  config();
  const port = Number(process.env.AERIS_DEVBOX_PORT ?? 5173);
  if (!Number.isInteger(port) || port < 0 || port > 65535) throw new Error('AERIS_DEVBOX_PORT 需要有效的端口号。');
  server = await createServer({ root, cacheDir: join(buildRoot, 'vite'), server: { host: '127.0.0.1', port }, plugins: [{
    name: 'aeris-devbox',
    configureServer(vite) {
      vite.middlewares.use(serve);
      vite.watcher.add([...watcherPaths]);
      vite.watcher.on('all', (event, path) => {
        if (!['add', 'change', 'unlink'].includes(event)) return;
        if ([...watcherPaths].some((entry) => path === entry || path.startsWith(entry + sep))) queue();
      });
    },
  }] });
  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
  await server.listen();
  server.printUrls();
  queue();
} catch (error) {
  console.error(error.message);
  await shutdown();
  process.exitCode = 1;
}
