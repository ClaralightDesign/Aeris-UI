// Embedded in the Cargo launcher so commands can connect from any directory.
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const usage = `用法：cargo aeris-devbox agent install-browser [devbox.json]
      cargo aeris-devbox agent serve [devbox.json]
      cargo aeris-devbox agent <命令> --session <文件>

status
wait [--after <buildId>] [--timeout <毫秒>]
snapshot
screenshot --out <file.png>
tap <x> <y> --preview <id>
swipe <x1> <y1> <x2> <y2> --preview <id>
back --preview <id>
scene <id>
reload
logs [--source build|runtime] [--since <cursor>]`;

function fail(code, message, details) {
  throw Object.assign(new Error(message), { code, details });
}

function parse([command, ...args]) {
  const coordinates = { tap: 2, swipe: 4 };
  const positional = coordinates[command] ?? (command === 'scene' ? 1 : 0);
  const options = {
    status: [], wait: ['after', 'timeout'], snapshot: [], screenshot: ['out'],
    tap: ['preview'], swipe: ['preview'], back: ['preview'], scene: [], reload: [], logs: ['source', 'since'],
  };
  if (!Object.hasOwn(options, command)) fail('INVALID_ARGUMENT', '未知命令。运行 agent --help 查看用法。');
  const params = {};
  const values = [];
  for (let index = 0; index < args.length; index++) {
    const arg = args[index];
    if (!arg.startsWith('--')) {
      values.push(arg);
      continue;
    }
    const name = arg.slice(2);
    const value = args[++index];
    if (!['session', ...options[command]].includes(name) || name in params || !value || value.startsWith('--')) {
      fail('INVALID_ARGUMENT', '无效参数：' + arg);
    }
    params[name] = value;
  }
  const { session, out, timeout: timeoutArg, ...request } = params;
  if (values.length !== positional || !session || (command === 'screenshot' && !out)
      || (['tap', 'swipe', 'back'].includes(command) && !request.preview)) {
    fail('INVALID_ARGUMENT', '缺少参数或位置参数数量不正确。运行 agent --help 查看用法。');
  }
  const integer = (value) => {
    if (!/^\d+$/.test(value) || !Number.isSafeInteger(Number(value))) fail('INVALID_ARGUMENT', '需要非负整数：' + value);
    return Number(value);
  };
  if (coordinates[command]) request.points = values.map(integer);
  if (command === 'scene') request.scene = values[0];
  if ('after' in request) request.after = integer(request.after);
  if ('since' in request) request.since = integer(request.since);
  if ('source' in request && !['build', 'runtime'].includes(request.source)) fail('INVALID_ARGUMENT', '日志来源应为 build 或 runtime。');
  const timeout = timeoutArg === undefined ? (command === 'wait' ? 300_000 : 30_000) : integer(timeoutArg);
  if (timeout < 1 || timeout > 3_600_000) fail('INVALID_ARGUMENT', '超时需在 1 到 3600000 毫秒之间。');
  return { command, request, session, out, timeout };
}

async function run(args) {
  const { command, request, session, out, timeout } = parse(args);
  let descriptor;
  try {
    descriptor = JSON.parse(await readFile(session, 'utf8'));
  } catch {
    fail('SESSION_UNAVAILABLE', '无法读取 DevBox 会话文件。');
  }
  if (descriptor?.version !== 1 || !/^http:\/\/127\.0\.0\.1:[1-9]\d{0,4}$/.test(descriptor.url)
      || !/^[a-f0-9]{64}$/.test(descriptor.token)) {
    fail('SESSION_UNAVAILABLE', 'DevBox 会话描述无效。');
  }
  const signal = AbortSignal.timeout(timeout);
  let result;
  do {
    try {
      const response = await fetch(descriptor.url + '/devbox-agent', {
        method: 'POST',
        redirect: 'error',
        headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ' + descriptor.token },
        body: JSON.stringify({ command, ...request }),
        signal,
      });
      result = await response.json();
    } catch {
      if (signal.aborted) fail('TIMEOUT', '等待 DevBox 响应超时。');
      fail('SESSION_UNAVAILABLE', '无法连接 DevBox 会话。');
    }
    if (result.version !== 1) fail('SESSION_UNAVAILABLE', 'DevBox 会话协议版本不兼容。');
    if (!result.ok) fail(result.error.code, result.error.message, result.error.details);
    if (command !== 'wait' || !result.pending) break;
    await delay(100, undefined, { signal }).catch(() => fail('TIMEOUT', '等待构建和预览首帧超时。', result));
  } while (true);

  if (out) {
    const { png, ...metadata } = result;
    const path = resolve(out);
    try {
      await writeFile(path, Buffer.from(png, 'base64'), { flag: 'wx' });
    } catch (error) {
      fail(error.code === 'EEXIST' ? 'OUTPUT_EXISTS' : 'OUTPUT_FAILED', '无法写入截图：' + error.message);
    }
    return { ...metadata, path };
  }
  return result;
}

const args = process.argv.slice(1);
if (!args.length || args.includes('--help') || args.includes('-h')) {
  console.log(usage);
} else {
  try {
    console.log(JSON.stringify(await run(args)));
  } catch (error) {
    console.log(JSON.stringify({
      version: 1, ok: false,
      error: { code: error.code ?? 'CLI_ERROR', message: error.message, details: error.details },
    }));
    process.exitCode = 1;
  }
}
