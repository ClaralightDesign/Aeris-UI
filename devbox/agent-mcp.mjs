import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { setTimeout as delay } from 'node:timers/promises';
import { z } from 'zod';

const pixel = z.int().nonnegative().describe('Device pixel coordinate, measured from the top-left of the canvas.');
const preview = z.string().min(1).describe('previewId from the latest snapshot or screenshot.');
const device = z.string().min(1).optional().describe('Device ID from status. Defaults to the first configured device.');
const readOnly = { readOnlyHint: true, openWorldHint: false };
const mutatesPreview = { destructiveHint: false, openWorldHint: false };

const tools = [
  ['status', 'Read project, scene, build and preview status.', {}, readOnly],
  ['wait', 'Wait for the latest queued build and the preview first frame. Pass after to require a newer build.', {
    after: z.int().nonnegative().optional().describe('Last observed buildId.'),
    timeoutMs: z.int().min(1).max(300_000).optional().describe('Maximum wait in milliseconds, defaults to 45 seconds.'),
  }, readOnly],
  ['snapshot', 'Read the LVGL object tree and its bounds in device pixels. Node IDs are valid only for this observation.', {}, readOnly],
  ['screenshot', 'Capture the visible device canvas as a PNG image.', {}, readOnly],
  ['tap', 'Tap a device pixel using coordinates from the current preview.', { x: pixel, y: pixel, preview }, mutatesPreview],
  ['swipe', 'Swipe between two device pixels in the current preview.', {
    x1: pixel, y1: pixel, x2: pixel, y2: pixel, preview,
  }, mutatesPreview],
  ['back', 'Send Escape to the current device preview.', { preview }, mutatesPreview],
  ['scene', 'Open a configured scene using its ID from status.', { scene: z.string().min(1) }, mutatesPreview],
  ['reload', 'Restart the current scene and return its new preview ID.', {}, mutatesPreview],
  ['logs', 'Read incremental build or runtime logs. Pass the last cursor to read new entries.', {
    source: z.enum(['build', 'runtime']).optional(), since: z.int().nonnegative().optional(),
  }, readOnly],
];

export async function serveMcp(req, res, body, dispatch, accessSignal) {
  const server = new McpServer({ name: 'aeris-devbox', version: '1.0.0' });
  for (const [command, description, inputSchema, annotations] of tools) {
    server.registerTool('devbox_' + command, { description, inputSchema: { device, ...inputSchema }, annotations }, async (args, { signal }) => {
      const { timeoutMs, x, y, x1, y1, x2, y2, ...params } = args;
      const deadline = AbortSignal.timeout(timeoutMs ?? 45_000);
      const cancelled = AbortSignal.any([accessSignal, signal, deadline]);
      const request = { command, ...params };
      if (command === 'tap') request.points = [x, y];
      if (command === 'swipe') request.points = [x1, y1, x2, y2];
      let result;
      try {
        do {
          cancelled.throwIfAborted();
          result = await dispatch(request, cancelled);
          if (command !== 'wait' || !result.pending) break;
          await delay(100, undefined, { signal: cancelled });
        } while (true);
        cancelled.throwIfAborted();
      } catch (cause) {
        const code = accessSignal.aborted ? 'FORBIDDEN' : deadline.aborted ? 'TIMEOUT'
          : typeof cause.code === 'string' ? cause.code : cause.name === 'TimeoutError' ? 'TIMEOUT' : 'PREVIEW_FAILED';
        const message = accessSignal.aborted ? 'MCP 已关闭。' : deadline.aborted ? '等待 DevBox 响应超时。' : cause.message;
        const output = { version: 1, ok: false, error: { code, message, details: cause.details } };
        return { isError: true, structuredContent: output, content: [{ type: 'text', text: JSON.stringify(output) }] };
      }
      const { png, ...data } = result;
      const output = { version: 1, ok: true, ...data };
      const content = [{ type: 'text', text: JSON.stringify(output) }];
      if (png) content.unshift({ type: 'image', data: png, mimeType: 'image/png' });
      return { structuredContent: output, content };
    });
  }

  const transport = new StreamableHTTPServerTransport({ enableJsonResponse: true, sessionIdGenerator: undefined });
  res.once('close', () => { void server.close(); });
  await server.connect(transport);
  await transport.handleRequest(req, res, body);
}
