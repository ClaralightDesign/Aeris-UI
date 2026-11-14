import { pickNode, type PreviewNode, type PreviewWindow } from './preview';

const canvas = document.querySelector<HTMLCanvasElement>('#demo')!;
const context = canvas.getContext('2d', { alpha: false })!;
const duration = 60_000;
const palette = { background: 0x08090b, surface: 0x1b2228, text: 0xf3f7f5, muted: 0x9daab1, accent: 0xb5f477, track: 0x24352e };

type Bounds = [x: number, y: number, width: number, height: number];

function node(id: number, parent: number, component: string, bounds: Bounds, properties: Partial<PreviewNode> = {}): PreviewNode {
  const [x, y, width, height] = bounds;
  const border = properties.border ?? 0;
  return {
    id, parent, kind: 3, className: 'CanvasObject', component, x, y, width, height,
    contentX: x + border, contentY: y + border, contentWidth: width - 2 * border, contentHeight: height - 2 * border,
    margin: [0, 0, 0, 0], borderWidths: [border, border, border, border],
    value: '', hidden: false, clickable: false, scrollable: false, overflow: false,
    events: 0, background: 0, foreground: palette.text, backgroundOpacity: 0, opacity: 255,
    radius: 0, border: 0, fontHeight: 0, alignment: 2, checked: false, disabled: false,
    ...properties,
  };
}

const screen = node(1, 0, 'DemoScreen', [0, 0, canvas.width, canvas.height], {
  kind: 0, className: 'CanvasScreen', background: palette.background, backgroundOpacity: 255,
});
const title = node(2, screen.id, 'Title', [24, 33, 288, 32], {
  kind: 1, className: 'CanvasLabel', value: '倒计时', fontHeight: 24,
});
const ring = node(3, screen.id, 'ProgressRing', [48, 90, 240, 240], {
  background: palette.track, foreground: palette.accent, backgroundOpacity: 255, radius: 120, border: 8,
});
const clock = node(4, ring.id, 'TimerDisplay', [65, 176, 206, 68], {
  kind: 1, className: 'CanvasLabel', fontHeight: 48,
});
const timerState = node(5, ring.id, 'TimerState', [80, 246, 176, 22], {
  kind: 1, className: 'CanvasLabel', foreground: palette.muted, fontHeight: 15,
});
const toggle = node(6, screen.id, 'TimerButton', [28, 355, 280, 50], {
  clickable: true, events: 1, background: palette.accent, backgroundOpacity: 255,
  foreground: palette.background, radius: 25, fontHeight: 18,
});
const reset = node(7, screen.id, 'ResetButton', [28, 414, 280, 40], {
  clickable: true, events: 1, value: '重置', background: palette.surface, backgroundOpacity: 255,
  foreground: palette.text, radius: 20, fontHeight: 16,
});
const nodes = [screen, title, ring, clock, timerState, toggle, reset];

let remaining = duration;
let deadline: number | undefined;
let frameId = 0;
let lastFrame: number | undefined;
let frameTime = 0;

function color(value: number) {
  return '#' + value.toString(16).padStart(6, '0');
}

function fillBox(item: PreviewNode) {
  context.fillStyle = color(item.background);
  context.beginPath();
  context.roundRect(item.x, item.y, item.width, item.height, item.radius);
  context.fill();
}

function drawLabel(item: PreviewNode, family = 'system-ui', weight = 600) {
  context.fillStyle = color(item.foreground);
  context.font = `${weight} ${item.fontHeight}px ${family}`;
  context.textAlign = 'center';
  context.textBaseline = 'middle';
  context.fillText(item.value, item.x + item.width / 2, item.y + item.height / 2);
}

function paint(now: number) {
  if (deadline !== undefined) {
    remaining = Math.max(0, deadline - now);
    if (lastFrame !== undefined) frameTime = now - lastFrame;
    lastFrame = now;
    if (remaining === 0) {
      deadline = undefined;
      log('倒计时结束');
    } else frameId = requestAnimationFrame(paint);
  }

  const seconds = Math.ceil(remaining / 1000);
  clock.value = `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
  timerState.value = deadline !== undefined ? '进行中' : remaining === 0 ? '已完成' : remaining === duration ? '准备就绪' : '已暂停';
  toggle.value = deadline !== undefined ? '暂停' : remaining === 0 ? '重新开始' : remaining === duration ? '开始' : '继续';

  fillBox(screen);
  drawLabel(title);
  const centerX = ring.x + ring.radius;
  const centerY = ring.y + ring.radius;
  const radius = ring.radius - ring.border / 2;
  const start = -Math.PI / 2;
  context.lineCap = 'round';
  context.lineWidth = ring.border;
  context.strokeStyle = color(ring.background);
  context.beginPath();
  context.arc(centerX, centerY, radius, start, start + Math.PI * 2);
  context.stroke();
  context.strokeStyle = color(ring.foreground);
  context.beginPath();
  context.arc(centerX, centerY, radius, start, start + Math.PI * 2 * remaining / duration);
  context.stroke();
  drawLabel(clock, 'ui-monospace, monospace', 650);
  drawLabel(timerState);
  fillBox(toggle);
  drawLabel(toggle);
  fillBox(reset);
  drawLabel(reset);
}

function log(text: string) {
  window.parent.postMessage({ type: 'aeris-debug-log', level: 'info', text }, location.origin);
}

function stop() {
  deadline = undefined;
  lastFrame = undefined;
  cancelAnimationFrame(frameId);
}

function toggleTimer() {
  const now = performance.now();
  if (deadline !== undefined) {
    remaining = Math.max(0, deadline - now);
    stop();
    log('倒计时已暂停');
  } else {
    if (remaining === 0) remaining = duration;
    log(remaining === duration ? '倒计时已开始' : '倒计时已继续');
    deadline = now + remaining;
  }
  paint(now);
}

function resetTimer() {
  stop();
  remaining = duration;
  paint(performance.now());
  log('倒计时已重置');
}

canvas.addEventListener('click', (event) => {
  const bounds = canvas.getBoundingClientRect();
  const target = pickNode(nodes, (event.clientX - bounds.left) * canvas.width / bounds.width,
    (event.clientY - bounds.top) * canvas.height / bounds.height);
  if (target === toggle) toggleTimer();
  if (target === reset) resetTimer();
});

canvas.addEventListener('keydown', (event) => {
  if (event.repeat || event.metaKey || event.ctrlKey || event.altKey) return;
  if (event.key === 'Enter') {
    event.preventDefault();
    toggleTimer();
  }
  if (event.key === 'Escape') resetTimer();
});

paint(performance.now());
(window as PreviewWindow).aerisDebugger = {
  snapshot: () => ({ height: canvas.height, nodes, frameTime, memoryBytes: 0 }),
  screenshot: () => canvas.toDataURL('image/png'),
};
log('演示画面已加载');
