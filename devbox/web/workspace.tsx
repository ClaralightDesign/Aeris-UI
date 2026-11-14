import { Button, Popover, Switch } from '@heroui/react';
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type PointerEvent, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Inspector } from './inspector';
import { Console, logLevel, type Log } from './console';
import { nodeName, pickNode, width, type PreviewWindow, type Snapshot } from './preview';
import logo from './assets/VerisDevBox.svg';
import caret from './assets/CaretDown.svg';
import rulerIcon from './assets/Ruler.svg';
import zoomIcon from './assets/MagnifyingGlassPlus.svg';
import code from './assets/CodeBlock.svg';
import camera from './assets/CameraPlus.svg';
import controls from './assets/Faders.svg';
import picker from './assets/PaperPlaneTilt.svg';
import prohibitRed from './assets/ProhibitInsetRed.svg';
import errorDot from './assets/ErrorDot.svg';
import commandKey from './assets/Command.svg';
import tooltipArrow from './assets/TooltipArrow.svg';
import captureFull from './assets/CaptureFull.svg';
import captureArea from './assets/CaptureArea.svg';
import recordFull from './assets/RecordFull.svg';
import recordArea from './assets/RecordArea.svg';
import checkIcon from './assets/Check.svg';
import panelLeft from './assets/PanelLeft.svg';
import panelRight from './assets/PanelRight.svg';
import panelBottom from './assets/PanelBottom.svg';
import errorIcon from './assets/ProhibitInset.svg';
import { downloadBlob, encodeAnimatedWebp, encodeGif, type RecordedFrame } from './export';
import './workspace.css';

const views = ['元素', '控制台', '源代码', '资源', '性能', '图层'] as const;
type View = typeof views[number];
type Status = 'ready' | 'building' | 'failed';
type Project = {
  title: string;
  scenes: { id: string; title: string }[];
  status: Status;
  hasPreview: boolean;
  logs: Omit<Log, 'source'>[];
};
type Source = { name: string; text: string };
const EMPTY_SNAPSHOT: Snapshot = { height: 480, nodes: [], frameTime: 0, memoryBytes: 0 };
const RULER_STEP = 10;
const RULER_LABEL = 50;
const RULER_EDGE = 40;
const INSPECTOR_MIN = 278;
const INSPECTOR_PULL = 14;
const RULER_TICKS = Array.from({ length: Math.floor(width / RULER_STEP) + 1 }, (_, index) => index * RULER_STEP);
const placements = ['left', 'right', 'bottom'] as const;
type Placement = typeof placements[number];
const placementInfo: Record<Placement, { title: string; icon: string }> = {
  left: { title: '左侧', icon: panelLeft },
  right: { title: '右侧', icon: panelRight },
  bottom: { title: '下侧', icon: panelBottom },
};

type CaptureMode = 'screen-shot' | 'area-shot' | 'screen-video' | 'area-video';
type CaptureFormat = 'mp4' | 'webp' | 'gif';
type CaptureRect = { x: number; y: number; width: number; height: number };
type RecordingSession = {
  mode: 'screen' | 'area';
  format: CaptureFormat;
  fps: number;
  cursor: boolean;
  region: CaptureRect;
  startedAt: number;
  frames: RecordedFrame[];
  pending: number;
  canvas: HTMLCanvasElement;
  recorder?: MediaRecorder;
  chunks: Blob[];
  sampleTimer?: number;
  drawFrame?: number;
  autoStop?: number;
  cancelled: boolean;
};
const captureModes: Record<CaptureMode, { title: string; action: string; icon: string }> = {
  'screen-shot': { title: '全屏截图', action: '开始截图', icon: captureFull },
  'area-shot': { title: '区域截图', action: '开始截图', icon: captureArea },
  'screen-video': { title: '全屏录屏', action: '开始录制', icon: recordFull },
  'area-video': { title: '区域录屏', action: '开始录制', icon: recordArea },
};
const captureOrder: CaptureMode[] = ['screen-shot', 'area-shot', 'screen-video', 'area-video'];
const captureFormats: CaptureFormat[] = ['mp4', 'webp', 'gif'];
const captureRates = [60, 30, 15];

function effectiveFps(format: CaptureFormat, fps: number) {
  return format === 'mp4' ? fps : Math.min(fps, format === 'gif' ? 12 : 24);
}

function inspectorLimit() {
  return Math.max(INSPECTOR_MIN, Math.min(window.innerWidth - 420, 1000));
}

function initialInspectorWidth() {
  const stored = Number(localStorage.getItem('aeris-debug-inspector-width'));
  const preferred = Number.isFinite(stored) && stored >= INSPECTOR_MIN ? stored : Math.round(window.innerWidth * 0.36);
  return Math.min(Math.max(preferred, INSPECTOR_MIN), inspectorLimit());
}

function Icon({ src }: { src: string }) {
  return <img className="debug-asset" src={src} alt="" />;
}

function formatDuration(milliseconds: number) {
  const seconds = Math.max(0, Math.floor(milliseconds / 1000));
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
}

function CaptureModeButton({ mode, selected, onSelect, onHover }: {
  mode: CaptureMode;
  selected: boolean;
  onSelect: () => void;
  onHover: (mode?: CaptureMode) => void;
}) {
  return <button type="button" className="debug-capture-mode" aria-pressed={selected} aria-label={captureModes[mode].title}
    onMouseEnter={() => onHover(mode)} onMouseLeave={() => onHover(undefined)}
    onFocus={() => onHover(mode)} onBlur={() => onHover(undefined)} onClick={onSelect}>
    <img className="debug-capture-icon" src={captureModes[mode].icon} alt="" />
  </button>;
}

function Hint({ label, keys, children, hidden }: { label: string; keys?: ReactNode; children: ReactNode; hidden?: boolean }) {
  const host = useRef<HTMLSpanElement>(null);
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  useEffect(() => {
    if (hidden) setOpen(false);
  }, [hidden]);
  function show() {
    if (hidden) return;
    const rect = host.current?.getBoundingClientRect();
    if (!rect) return;
    const center = rect.left + rect.width / 2;
    setPosition({ top: rect.bottom + 6, left: Math.min(window.innerWidth - 72, Math.max(72, center)) });
    setOpen(true);
  }
  function hide() {
    setOpen(false);
  }
  useEffect(() => {
    if (!open) return;
    window.addEventListener('resize', hide);
    window.addEventListener('scroll', hide, true);
    return () => {
      window.removeEventListener('resize', hide);
      window.removeEventListener('scroll', hide, true);
    };
  }, [open]);
  return <span className="debug-tooltip-host" ref={host}
    onMouseEnter={show} onMouseLeave={hide} onFocusCapture={show} onBlurCapture={hide}>
    {children}
    {open && !hidden && createPortal(
      <span className="debug-tooltip" style={{ top: position.top, left: position.left }} role="tooltip">
        <img className="debug-tooltip-arrow" src={tooltipArrow} alt="" />
        <span className="debug-tooltip-body">
          <span className="debug-tooltip-label">{label}</span>
          {keys && <span className="debug-tooltip-keys">{keys}</span>}
        </span>
      </span>,
      document.body,
    )}
  </span>;
}

function Ruler({ axis, size, cursor }: { axis: 'x' | 'y'; size: number; cursor?: number }) {
  const ticks = axis === 'x' ? RULER_TICKS : Array.from({ length: Math.floor(size / RULER_STEP) + 1 }, (_, index) => index * RULER_STEP);
  const place = (value: number) => axis === 'x' ? { left: value / size * 100 + '%' } : { top: value / size * 100 + '%' };
  const edge = cursor === undefined ? undefined : cursor < RULER_EDGE ? 'start' : cursor > size - RULER_EDGE ? 'end' : 'center';
  return <div className="debug-ruler" data-axis={axis} aria-hidden="true">
    {ticks.map((value) => <span key={value} className="debug-ruler-tick" data-major={value % RULER_LABEL === 0} style={place(value)}>
      {value > 0 && value % RULER_LABEL === 0 && value < size - RULER_EDGE
        && (cursor === undefined || Math.abs(value - cursor) >= RULER_EDGE / 2) && <em>{value}</em>}
    </span>)}
    {cursor !== undefined && <>
      <span className="debug-ruler-cursor" style={place(cursor)} />
      <span className="debug-ruler-value" data-edge={edge} style={place(cursor)}>{Math.round(cursor)}</span>
    </>}
    {(cursor === undefined || cursor < size - RULER_EDGE) && <span className="debug-ruler-size">{Math.round(size)}</span>}
  </div>;
}

export function Component() {
  const [project, setProject] = useState<Project>();
  const [projectError, setProjectError] = useState(false);
  const [sceneId, setSceneId] = useState(() => new URLSearchParams(location.search).get('scene'));
  const scene = project?.scenes.find((item) => item.id === sceneId) ?? project?.scenes[0];
  const [hasPreview, setHasPreview] = useState(false);
  const [source, setSource] = useState<Source>();
  const [snapshot, setSnapshot] = useState<Snapshot>(EMPTY_SNAPSHOT);
  const [generation, setGeneration] = useState(0);
  const [status, setStatus] = useState<Status>('ready');
  const [logs, setLogs] = useState<Log[]>([]);
  const [notice, setNotice] = useState('');
  const [view, setView] = useState<View>('元素');
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [picking, setPicking] = useState(false);
  const [scenesOpen, setScenesOpen] = useState(false);
  const [ruler, setRuler] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [autoRefresh, setAutoRefresh] = useState(() => localStorage.getItem('aeris-debug-auto-refresh') !== 'off');
  const [selectedId, select] = useState<number>();
  const [hoverId, hover] = useState<number>();
  const [measure, setMeasure] = useState<{ x: number; y: number }>();
  const [frameReady, setFrameReady] = useState(0);
  const [placement, setPlacement] = useState<Placement>(() => {
    const saved = localStorage.getItem('aeris-debug-inspector-placement');
    return saved === 'left' || saved === 'bottom' ? saved : 'right';
  });
  const [inspectorWidth, setInspectorWidth] = useState(initialInspectorWidth);
  const [resizing, setResizing] = useState(false);
  const [dockOpen, setDockOpen] = useState(false);
  const [copyState, setCopyState] = useState<'idle' | 'copying' | 'copied' | 'failed'>('idle');
  const [compactStatus, setCompactStatus] = useState(false);
  const [captureOpen, setCaptureOpen] = useState(false);
  const [captureMode, setCaptureMode] = useState<CaptureMode>('screen-video');
  const [captureHover, setCaptureHover] = useState<CaptureMode>();
  const [captureFormat, setCaptureFormat] = useState<CaptureFormat>('mp4');
  const [captureRate, setCaptureRate] = useState(60);
  const [captureCursor, setCaptureCursor] = useState(true);
  const [captureSettings, setCaptureSettings] = useState(false);
  const [zoomOpen, setZoomOpen] = useState(false);
  const [controlsOpen, setControlsOpen] = useState(false);
  const [areaMode, setAreaMode] = useState<'shot' | 'video'>();
  const [area, setArea] = useState<CaptureRect>();
  const [recording, setRecording] = useState<{ mode: 'screen' | 'area'; format: CaptureFormat; startedAt: number }>();
  const [recordTime, setRecordTime] = useState(0);
  const [pill, setPill] = useState<'idle' | 'recording' | 'exporting' | 'saved'>('idle');
  const [savedLabel, setSavedLabel] = useState('');
  const [exportProgress, setExportProgress] = useState(0);
  const [captureNotice, setCaptureNotice] = useState('');
  const [indicator, setIndicator] = useState({ left: 0, width: 0 });
  const [indicatorReady, setIndicatorReady] = useState(false);
  const restored = useRef(true);
  const copyTimer = useRef<number>(undefined);
  const statusRef = useRef<HTMLDivElement>(null);
  const tabList = useRef<HTMLDivElement>(null);
  const session = useRef<RecordingSession | undefined>(undefined);
  const areaStart = useRef<{ x: number; y: number } | undefined>(undefined);
  const pointer = useRef<{ x: number; y: number } | undefined>(undefined);
  const noticeTimer = useRef<number>(undefined);
  const savedTimer = useRef<number>(undefined);
  const preview = useRef<HTMLIFrameElement>(null);
  const refreshEnabled = useRef(autoRefresh);

  const previewParams = new URLSearchParams({ scene: scene?.id ?? '', debug: '1', revision: String(generation) });
  const previewUrl = '/preview.html?' + previewParams;
  const renderer = () => (preview.current?.contentWindow as PreviewWindow | null)?.aerisDebugger;
  const selected = snapshot.nodes.find((node) => node.id === selectedId) ?? snapshot.nodes[0];
  const highlighted = snapshot.nodes.find((node) => node.id === hoverId) ?? (ruler || picking ? selected : undefined);
  const errorCount = logs.filter((entry) => entry.level === 'error').length;

  function append(log: Log) {
    setLogs((previous) => [...previous.slice(-499), log]);
  }

  useEffect(() => {
    const previous = document.title;
    document.title = 'Aeris DevBox';
    return () => { document.title = previous; };
  }, []);

  useEffect(() => {
    refreshEnabled.current = autoRefresh;
    localStorage.setItem('aeris-debug-auto-refresh', autoRefresh ? 'on' : 'off');
  }, [autoRefresh]);

  useEffect(() => {
    if (restored.current) {
      restored.current = false;
      return;
    }
    localStorage.setItem('aeris-debug-inspector-placement', placement);
    localStorage.setItem('aeris-debug-inspector-width', String(inspectorWidth));
  }, [placement, inspectorWidth]);

  useEffect(() => {
    setSnapshot(EMPTY_SNAPSHOT);
    select(undefined);
    hover(undefined);
    const timer = window.setInterval(() => {
      const next = renderer()?.snapshot();
      if (next) setSnapshot(next);
    }, 250);
    return () => window.clearInterval(timer);
  }, [previewUrl]);

  useEffect(() => {
    function onMessage(event: MessageEvent) {
      if (event.origin !== location.origin || event.source !== preview.current?.contentWindow) return;
      const data = event.data;
      if (data?.type === 'aeris-debug-log' && typeof data.text === 'string') {
        append({ level: logLevel(data.level, data.text), source: '运行', text: data.text });
      } else if (data?.type === 'aeris-debug-error') {
        append({ level: 'error', source: '运行', text: '无法加载组件预览。' });
      }
    }
    window.addEventListener('message', onMessage);
    const hot = import.meta.hot;
    const startup = new AbortController();
    async function loadProject(initial = false) {
      try {
        const response = await fetch('/devbox-project.json', { signal: startup.signal, cache: 'no-store' });
        if (!response.ok) throw new Error();
        const data: Project = await response.json();
        if (startup.signal.aborted) return;
        setProject(data);
        setProjectError(false);
        if (initial) {
          setStatus(data.status);
          setHasPreview(data.hasPreview);
          setLogs(data.logs.map((entry) => ({ ...entry, level: logLevel(entry.level, entry.text), source: '构建' })));
        }
      } catch {
        if (!startup.signal.aborted) setProjectError(true);
      }
    }
    function build({ status: next }: { status: Status }) {
      setStatus(next);
      if (next === 'ready') {
        setHasPreview(true);
        if (refreshEnabled.current) setGeneration((value) => value + 1);
        void loadProject();
      }
    }
    function buildLog({ level, text }: { level: Log['level']; text: string }) {
      append({ level: logLevel(level, text), source: '构建', text });
    }
    void loadProject(true);
    hot?.on('aeris:build', build);
    hot?.on('aeris:log', buildLog);
    return () => {
      startup.abort();
      window.removeEventListener('message', onMessage);
      hot?.off('aeris:build', build);
      hot?.off('aeris:log', buildLog);
    };
  }, []);

  useEffect(() => {
    if (!picking) return;
    function escape(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        setPicking(false);
        hover(undefined);
      }
    }
    window.addEventListener('keydown', escape);
    return () => window.removeEventListener('keydown', escape);
  }, [picking]);

  function findAt(event: PointerEvent<HTMLDivElement>) {
    const rect = event.currentTarget.getBoundingClientRect();
    const x = (event.clientX - rect.left) * width / rect.width;
    const y = (event.clientY - rect.top) * snapshot.height / rect.height;
    if (ruler) setMeasure({ x, y });
    return pickNode(snapshot.nodes, x, y);
  }

  // 直接监听预览页面的指针，标尺才能在不遮挡画面的情况下跟随鼠标。
  useEffect(() => {
    if (!ruler) {
      setMeasure(undefined);
      return;
    }
    const doc = preview.current?.contentDocument;
    const view = doc?.defaultView;
    if (!doc?.documentElement || !view) return;
    function track(event: globalThis.PointerEvent) {
      setMeasure({
        x: Math.min(width, Math.max(0, event.clientX / view!.innerWidth * width)),
        y: Math.min(snapshot.height, Math.max(0, event.clientY / view!.innerHeight * snapshot.height)),
      });
    }
    function clear() {
      setMeasure(undefined);
    }
    doc.addEventListener('pointermove', track);
    doc.documentElement.addEventListener('pointerleave', clear);
    return () => {
      doc.removeEventListener('pointermove', track);
      doc.documentElement.removeEventListener('pointerleave', clear);
    };
  }, [ruler, previewUrl, snapshot.height, frameReady]);

  // 拖动分隔条调整检查器宽度，越过最小宽度后直接收起。
  function resize(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    event.preventDefault();
    const handle = event.currentTarget;
    const startX = event.clientX;
    const startWidth = inspectorWidth;
    const direction = placement === 'left' ? 1 : -1;
    const limit = inspectorLimit();
    if (handle.isConnected) {
      try { handle.setPointerCapture(event.pointerId); } catch { /* 指针已释放 */ }
    }
    setResizing(true);
    function move(pointer: globalThis.PointerEvent) {
      const next = startWidth + (pointer.clientX - startX) * direction;
      if (next < INSPECTOR_MIN - INSPECTOR_PULL) {
        setInspectorOpen(false);
        finish();
        return;
      }
      setInspectorWidth(Math.min(limit, Math.max(INSPECTOR_MIN, next)));
    }
    function finish() {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', finish);
      window.removeEventListener('pointercancel', finish);
      if (handle.hasPointerCapture(event.pointerId)) handle.releasePointerCapture(event.pointerId);
      setResizing(false);
    }
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', finish);
    window.addEventListener('pointercancel', finish);
  }

  function saveScreenshot() {
    const url = renderer()?.screenshot();
    if (!url) {
      notifyCapture('截图失败');
      return;
    }
    const link = document.createElement('a');
    link.href = url;
    link.download = (scene?.id ?? project?.title ?? 'aeris') + '.png';
    link.click();
    markSaved('已保存为 PNG');
  }

  function changeScene(id: string) {
    const url = new URL(location.href);
    url.searchParams.set('scene', id);
    history.replaceState(null, '', url);
    setSceneId(id);
  }

  useEffect(() => {
    setSource(undefined);
    if (view !== '源代码' || !project) return;
    const request = new AbortController();
    void fetch('/devbox-source?' + new URLSearchParams({ scene: scene?.id ?? '' }), { signal: request.signal })
      .then(async (response) => {
        if (response.ok && !request.signal.aborted) setSource(await response.json());
      })
      .catch(() => {});
    return () => request.abort();
  }, [project, scene?.id, view, generation]);

  function openView(next: View) {
    setView(next);
    setInspectorOpen(true);
    if (next !== '元素') {
      setPicking(false);
      hover(undefined);
    }
  }

  function notifyCapture(text: string) {
    setCaptureNotice(text);
    window.clearTimeout(noticeTimer.current);
    noticeTimer.current = window.setTimeout(() => setCaptureNotice(''), 2600);
  }

  function markSaved(label: string) {
    setSavedLabel(label);
    setPill('saved');
    window.clearTimeout(savedTimer.current);
    savedTimer.current = window.setTimeout(() => {
      setPill('idle');
      setRecordTime(0);
    }, 5000);
  }

  function resetCapture() {
    window.clearTimeout(savedTimer.current);
    setPill('idle');
    setRecordTime(0);
  }

  function captureBaseName() {
    return scene?.id ?? project?.title ?? 'aeris';
  }

  function sourceCanvas() {
    return preview.current?.contentDocument?.querySelector('canvas') ?? null;
  }

  function drawPointer(context: CanvasRenderingContext2D, x: number, y: number) {
    context.save();
    context.beginPath();
    context.arc(x, y, 7, 0, Math.PI * 2);
    context.fillStyle = 'rgba(255,255,255,.9)';
    context.fill();
    context.lineWidth = 2;
    context.strokeStyle = 'rgba(20,20,20,.5)';
    context.stroke();
    context.restore();
  }

  async function saveAreaScreenshot(region: CaptureRect) {
    const url = renderer()?.screenshot();
    if (!url) return;
    const image = new Image();
    image.src = url;
    await image.decode();
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.round(region.width));
    canvas.height = Math.max(1, Math.round(region.height));
    canvas.getContext('2d')!.drawImage(
      image, region.x, region.y, region.width, region.height, 0, 0, canvas.width, canvas.height,
    );
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
    if (blob) {
      downloadBlob(blob, `${captureBaseName()}-area.png`);
      markSaved('已保存为 PNG');
    } else {
      notifyCapture('截图失败');
    }
  }

  function startRecording(mode: 'screen' | 'area', region?: CaptureRect) {
    const source = sourceCanvas();
    if (!source) {
      notifyCapture('预览尚未准备好');
      return;
    }
    const area = region ?? { x: 0, y: 0, width, height: snapshot.height };
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.round(area.width));
    canvas.height = Math.max(1, Math.round(area.height));
    const context = canvas.getContext('2d', { alpha: false })!;
    const format = captureFormat;
    const fps = captureRate;
    const sessionValue: RecordingSession = {
      mode, format, fps, cursor: captureCursor, region: area, startedAt: performance.now(),
      frames: [], pending: 0, canvas, chunks: [], cancelled: false,
    };
    session.current = sessionValue;
    setRecording({ mode, format, startedAt: sessionValue.startedAt });
    setRecordTime(0);
    setCaptureNotice('');
    window.clearTimeout(savedTimer.current);
    setSavedLabel('');
    setPill('recording');
    const draw = () => {
      if (session.current !== sessionValue) return;
      const current = sourceCanvas();
      if (current) {
        context.drawImage(current, area.x, area.y, area.width, area.height, 0, 0, canvas.width, canvas.height);
        const point = pointer.current;
        if (sessionValue.cursor && point && point.x >= area.x && point.y >= area.y
          && point.x <= area.x + area.width && point.y <= area.y + area.height) {
          drawPointer(context, point.x - area.x, point.y - area.y);
        }
      }
      sessionValue.drawFrame = requestAnimationFrame(draw);
    };
    draw();
    if (format === 'mp4') {
      try {
        const mime = MediaRecorder.isTypeSupported('video/mp4;codecs=avc1.42E01E')
          ? 'video/mp4;codecs=avc1.42E01E'
          : 'video/webm;codecs=vp9';
        const recorder = new MediaRecorder(canvas.captureStream(fps), { mimeType: mime, videoBitsPerSecond: 8_000_000 });
        recorder.ondataavailable = (event) => {
          if (event.data.size) sessionValue.chunks.push(event.data);
        };
        recorder.start(500);
        sessionValue.recorder = recorder;
      } catch {
        session.current = undefined;
        if (sessionValue.drawFrame) cancelAnimationFrame(sessionValue.drawFrame);
        setRecording(undefined);
        setPill('idle');
        notifyCapture('当前浏览器无法开始录制');
        return;
      }
    } else {
      const interval = Math.max(40, 1000 / Math.max(1, effectiveFps(format, fps)));
      sessionValue.sampleTimer = window.setInterval(() => {
        if (session.current !== sessionValue) return;
        const frameCanvas = document.createElement('canvas');
        frameCanvas.width = canvas.width;
        frameCanvas.height = canvas.height;
        frameCanvas.getContext('2d')!.drawImage(canvas, 0, 0);
        const at = performance.now();
        sessionValue.pending++;
        frameCanvas.toBlob((blob) => {
          if (blob) sessionValue.frames.push({ blob, at });
          sessionValue.pending--;
        }, 'image/webp', 0.92);
      }, interval);
    }
    sessionValue.autoStop = window.setTimeout(() => { void stopRecording(true); }, 120000);
  }

  async function stopRecording(exportFile: boolean) {
    const value = session.current;
    if (!value) return;
    session.current = undefined;
    value.cancelled = !exportFile;
    if (value.drawFrame) cancelAnimationFrame(value.drawFrame);
    if (value.sampleTimer) window.clearInterval(value.sampleTimer);
    if (value.autoStop) window.clearTimeout(value.autoStop);
    setRecording(undefined);
    if (value.recorder && value.recorder.state !== 'inactive') {
      await new Promise<void>((resolve) => {
        value.recorder!.addEventListener('stop', () => resolve(), { once: true });
        value.recorder!.stop();
      });
    }
    const deadline = performance.now() + 900;
    while (value.pending > 0 && performance.now() < deadline) {
      await new Promise((resolve) => window.setTimeout(resolve, 30));
    }
    if (!exportFile) {
      setPill('idle');
      notifyCapture('已取消录制');
      return;
    }
    const name = captureBaseName();
    setPill('exporting');
    setExportProgress(0);
    try {
      if (value.format === 'mp4') {
        const type = value.recorder?.mimeType ?? 'video/mp4';
        const extension = type.includes('mp4') ? 'mp4' : 'webm';
        downloadBlob(new Blob(value.chunks, { type }), `${name}-recording.${extension}`);
        markSaved(extension === 'mp4' ? '已保存为 MP4' : '已保存为 WebM');
      } else {
        const frames = value.frames.slice().sort((a, b) => a.at - b.at);
        if (!frames.length) {
          setPill('idle');
          notifyCapture('没有捕获到画面');
          return;
        }
        const size = { width: value.canvas.width, height: value.canvas.height };
        const blob = value.format === 'gif'
          ? await encodeGif(frames, size, setExportProgress)
          : await encodeAnimatedWebp(frames, size, setExportProgress);
        downloadBlob(blob, `${name}-recording.${value.format === 'gif' ? 'gif' : 'webp'}`);
        markSaved(value.format === 'gif' ? '已保存为 GIF' : '已保存为 WebP');
      }
    } catch {
      setPill('idle');
      notifyCapture('导出失败');
    }
  }

  function beginCapture() {
    setCaptureOpen(false);
    setCaptureSettings(false);
    if (captureMode === 'screen-shot') {
      saveScreenshot();
    } else if (captureMode === 'area-shot') {
      setArea(undefined);
      setAreaMode('shot');
    } else if (captureMode === 'area-video') {
      setArea(undefined);
      setAreaMode('video');
    } else {
      startRecording('screen');
    }
  }

  function areaPoint(event: PointerEvent<HTMLDivElement>) {
    const rect = event.currentTarget.getBoundingClientRect();
    return {
      x: Math.min(width, Math.max(0, (event.clientX - rect.left) * width / rect.width)),
      y: Math.min(snapshot.height, Math.max(0, (event.clientY - rect.top) * snapshot.height / rect.height)),
    };
  }

  function startArea(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    const point = areaPoint(event);
    areaStart.current = point;
    setArea({ x: point.x, y: point.y, width: 0, height: 0 });
  }

  function moveArea(event: PointerEvent<HTMLDivElement>) {
    const start = areaStart.current;
    if (!start) return;
    const point = areaPoint(event);
    setArea({
      x: Math.min(start.x, point.x),
      y: Math.min(start.y, point.y),
      width: Math.abs(point.x - start.x),
      height: Math.abs(point.y - start.y),
    });
  }

  function endArea(event: PointerEvent<HTMLDivElement>) {
    const start = areaStart.current;
    const mode = areaMode;
    areaStart.current = undefined;
    if (!start || !mode) {
      setArea(undefined);
      return;
    }
    const point = areaPoint(event);
    const region = {
      x: Math.min(start.x, point.x),
      y: Math.min(start.y, point.y),
      width: Math.abs(point.x - start.x),
      height: Math.abs(point.y - start.y),
    };
    setAreaMode(undefined);
    setArea(undefined);
    if (region.width < 4 || region.height < 4) return;
    if (mode === 'shot') void saveAreaScreenshot(region);
    else startRecording('area', region);
  }

  useEffect(() => {
    if (!areaMode) return;
    function escape(event: KeyboardEvent) {
      if (event.key !== 'Escape') return;
      setAreaMode(undefined);
      setArea(undefined);
      areaStart.current = undefined;
    }
    window.addEventListener('keydown', escape);
    return () => window.removeEventListener('keydown', escape);
  }, [areaMode]);

  useEffect(() => {
    if (!recording) {
      setRecordTime(0);
      return;
    }
    const timer = window.setInterval(() => setRecordTime(performance.now() - recording.startedAt), 200);
    return () => window.clearInterval(timer);
  }, [recording]);

  useEffect(() => {
    const doc = preview.current?.contentDocument;
    const view = doc?.defaultView;
    if (!doc?.documentElement || !view) return;
    function move(event: globalThis.PointerEvent) {
      pointer.current = {
        x: Math.min(width, Math.max(0, event.clientX / view!.innerWidth * width)),
        y: Math.min(snapshot.height, Math.max(0, event.clientY / view!.innerHeight * snapshot.height)),
      };
    }
    function leave() {
      pointer.current = undefined;
    }
    doc.addEventListener('pointermove', move);
    doc.documentElement.addEventListener('pointerleave', leave);
    return () => {
      doc.removeEventListener('pointermove', move);
      doc.documentElement.removeEventListener('pointerleave', leave);
    };
  }, [previewUrl, snapshot.height, frameReady]);

  useEffect(() => () => {
    window.clearTimeout(noticeTimer.current);
    window.clearTimeout(savedTimer.current);
    const value = session.current;
    if (!value) return;
    session.current = undefined;
    if (value.drawFrame) cancelAnimationFrame(value.drawFrame);
    if (value.sampleTimer) window.clearInterval(value.sampleTimer);
    if (value.autoStop) window.clearTimeout(value.autoStop);
    if (value.recorder && value.recorder.state !== 'inactive') value.recorder.stop();
  }, []);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (!(event.metaKey || event.ctrlKey) || event.shiftKey || event.altKey) return;
      const element = event.target instanceof Element ? event.target : null;
      if (element?.closest('input, textarea, select, [contenteditable="true"]')) return;
      const key = event.key.toLowerCase();
      if (key === 's') {
        event.preventDefault();
        if (pill === 'idle') {
          setControlsOpen(false);
          setCaptureOpen((open) => !open);
        }
        return;
      }
      if (key === 'r') {
        event.preventDefault();
        setRuler((value) => !value);
        return;
      }
      if (key === 'e') {
        event.preventDefault();
        if (pill === 'idle') {
          setCaptureOpen(false);
          setControlsOpen((open) => !open);
        }
        return;
      }
      if (key === 'i') {
        event.preventDefault();
        setInspectorOpen((open) => !open);
        setPicking(false);
        hover(undefined);
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [pill]);

  function promptText(source?: Source) {
    const errors = logs.filter((entry) => entry.level === 'error');
    const lines = [
      'Aeris DevBox 构建失败，请帮我定位并修复。',
      '',
      `项目：${project?.title ?? 'Aeris UI'}${scene ? ' / ' + scene.title : ''}`,
      `错误数：${errorCount}`,
      '',
      '错误日志：',
      ...(errors.length ? errors.map((entry) => `${entry.source}：${entry.text.trim()}`) : ['（没有捕获到错误输出）']),
      '',
      '最近日志：',
      ...(logs.length ? logs.slice(-12).map((entry) => `${entry.source}/${entry.level}：${entry.text.trim()}`) : ['（无）']),
    ];
    if (source?.text) lines.push('', `当前场景源码（${source.name}）：`, '```', source.text, '```');
    return lines.join('\n');
  }

  function flashCopy(state: 'copied' | 'failed') {
    setCopyState(state);
    window.clearTimeout(copyTimer.current);
    copyTimer.current = window.setTimeout(() => setCopyState('idle'), 2000);
  }

  function copyWithSelection(text: string) {
    try {
      const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      const area = document.createElement('textarea');
      area.value = text;
      area.readOnly = true;
      area.style.cssText = 'position:fixed;top:0;left:0;opacity:0;pointer-events:none';
      document.body.append(area);
      area.focus();
      area.select();
      const copied = document.execCommand('copy');
      area.remove();
      active?.focus();
      return copied;
    } catch {
      return false;
    }
  }

  async function writeClipboard(text: string) {
    if (copyWithSelection(text)) return true;
    try {
      // 未授权剪贴板时浏览器可能一直等待，超时后按失败处理。
      return await Promise.race([
        navigator.clipboard.writeText(text).then(() => true),
        new Promise<boolean>((resolve) => window.setTimeout(() => resolve(false), 1200)),
      ]);
    } catch {
      return false;
    }
  }

  async function copyPrompt() {
    setCopyState('copying');
    let source: Source | undefined;
    try {
      const response = await fetch('/devbox-source?' + new URLSearchParams({ scene: scene?.id ?? '' }), { cache: 'no-store' });
      if (response.ok) {
        const data: Source = await response.json();
        if (data.text && data.text.length <= 30000) source = data;
      }
    } catch { /* 没有源码接口时只复制日志。 */ }
    const copied = await writeClipboard(promptText(source));
    flashCopy(copied ? 'copied' : 'failed');
  }

  // 失败叠层可见时启用设计稿上的快捷键：Enter 打开详情，⌘/Ctrl+C 复制提示词。
  useEffect(() => {
    if (status !== 'failed') return;
    function onKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.isComposing) return;
      const element = event.target instanceof Element ? event.target : null;
      if (element?.closest('input, textarea, select, [contenteditable="true"]')) return;
      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'c') {
        if (window.getSelection()?.toString()) return;
        event.preventDefault();
        void copyPrompt();
        return;
      }
      if (event.key === 'Enter' && !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey) {
        if (element?.closest('button, a, [role="button"]')) return;
        event.preventDefault();
        openView('控制台');
      }
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [status, logs, project, scene?.id, scene?.title, errorCount]);

  useEffect(() => () => window.clearTimeout(copyTimer.current), []);

  useEffect(() => {
    const parent = statusRef.current?.parentElement;
    if (!parent) return;
    const update = () => setCompactStatus(parent.clientWidth < 685);
    const observer = new ResizeObserver(update);
    observer.observe(parent);
    update();
    return () => observer.disconnect();
  }, [status]);

  useLayoutEffect(() => {
    const list = tabList.current;
    if (!list) return;
    const update = () => {
      const button = list.querySelector<HTMLButtonElement>('.debug-pill[aria-pressed="true"]');
      if (!button) return;
      const listRect = list.getBoundingClientRect();
      const buttonRect = button.getBoundingClientRect();
      if (!buttonRect.width) return;
      const left = buttonRect.left - listRect.left;
      setIndicator({ left, width: buttonRect.width });
      const scroller = list.parentElement;
      if (!scroller || scroller.scrollWidth <= scroller.clientWidth) return;
      const right = left + buttonRect.width;
      if (left < scroller.scrollLeft) scroller.scrollTo({ left: Math.max(0, left - 12), behavior: 'smooth' });
      else if (right > scroller.scrollLeft + scroller.clientWidth) {
        scroller.scrollTo({ left: right - scroller.clientWidth + 12, behavior: 'smooth' });
      }
    };
    update();
    const frame = window.requestAnimationFrame(update);
    const observer = new ResizeObserver(update);
    observer.observe(list);
    return () => {
      window.cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [view, errorCount]);

  useEffect(() => {
    if (indicatorReady) return;
    const timer = window.setTimeout(() => setIndicatorReady(true), 80);
    return () => window.clearTimeout(timer);
  }, [indicatorReady]);

  const apple = /Mac|iPhone|iPad|iPod/.test(navigator.userAgent);
  const shownCapture = captureHover ?? captureMode;
  const captureTitle = captureModes[shownCapture].title;
  const captureAction = captureModes[captureMode].action;
  const activeFps = effectiveFps(captureFormat, captureRate);
  const shortcut = (letter: string) => <>
    <span className="debug-tooltip-key" data-icon={apple ? 'true' : undefined}>
      {apple ? <img src={commandKey} alt="" /> : 'Ctrl'}
    </span>
    <span className="debug-tooltip-key" data-dim="true">{letter}</span>
  </>;

  return <main className="debug-workspace" data-inspector={inspectorOpen} data-placement={placement} data-resizing={resizing}
    style={{ '--debug-inspector-width': (inspectorOpen ? inspectorWidth : 0) + 'px' } as CSSProperties}>
    <section className="debug-preview-panel" aria-label="手表预览">
      <header className="debug-toolbar">
        <div className="debug-logo"><img src={logo} alt="Aeris DevBox" /></div>
        {project && project.scenes.length === 0 ? <span className="debug-app-title">{project.title}</span> :
        <Popover isOpen={scenesOpen} onOpenChange={setScenesOpen}>
          <Button variant="ghost" className="debug-target" aria-label="选择场景">
            <span>{project?.title ?? "Aeris DevBox"}</span>{scene && <span className="debug-target-device"> · {scene.title}</span>}<Icon src={caret} />
          </Button>
          <Popover.Content placement="bottom" className="debug-popover-wrap"><Popover.Dialog aria-label="预览场景" className="debug-popover">
            <span className="debug-section-title">预览场景</span>
            {project?.scenes.map((item) => <Button key={item.id} variant="ghost" className="debug-menu-item"
              aria-pressed={scene?.id === item.id} onPress={() => { changeScene(item.id); setScenesOpen(false); }}>
              {item.title}
            </Button>)}
          </Popover.Dialog></Popover.Content>
        </Popover>}
        <div className="debug-tools">
          <Hint label="标尺" keys={shortcut('R')}>
            <Button isIconOnly variant="ghost" className="debug-icon" aria-label="显示标尺" aria-pressed={ruler} onPress={() => setRuler(!ruler)}><Icon src={rulerIcon} /></Button>
          </Hint>
          <Hint label="缩放" hidden={zoomOpen}>
            <Popover isOpen={zoomOpen} onOpenChange={setZoomOpen}>
              <Button isIconOnly variant="ghost" className="debug-icon" aria-label="缩放"><Icon src={zoomIcon} /></Button>
              <Popover.Content className="debug-popover-wrap"><Popover.Dialog aria-label="缩放" className="debug-popover">
                <span className="debug-section-title">缩放</span>
                {[0.75, 1, 1.5, 2].map((scale) => <Button key={scale} variant="ghost" className="debug-menu-item" aria-pressed={zoom === scale} onPress={() => setZoom(scale)}>{scale * 100}%</Button>)}
              </Popover.Dialog></Popover.Content>
            </Popover>
          </Hint>
          <Hint label="检查面板" keys={shortcut('I')}>
            <Button isIconOnly variant="ghost" className="debug-icon" aria-label="显示检查面板"
              aria-pressed={inspectorOpen} onPress={() => { setInspectorOpen(!inspectorOpen); setPicking(false); hover(undefined); }}><Icon src={code} /></Button>
          </Hint>
          <div className="debug-capture-slot" data-state={pill}>
            <span className="debug-capture-idle" inert={pill !== 'idle'} aria-hidden={pill !== 'idle'}>
              <Hint label="录屏" keys={shortcut('S')} hidden={captureOpen || pill !== 'idle'}>
                <Popover isOpen={captureOpen} onOpenChange={(open) => {
                  setCaptureOpen(open);
                  if (!open) {
                    setCaptureSettings(false);
                    setCaptureHover(undefined);
                  }
                }}>
                  <Button isIconOnly variant="ghost" className="debug-icon" aria-label="截图与录屏"
                    isDisabled={!snapshot.nodes.length || pill !== 'idle'}><Icon src={camera} /></Button>
                  <Popover.Content placement="bottom end" className="debug-capture-popover"><Popover.Dialog className="debug-capture" aria-label="截图与录屏">
                    <span className="debug-capture-title">{captureTitle}</span>
                    <div className="debug-capture-modes">
                      {captureOrder.slice(0, 2).map((mode) => <CaptureModeButton key={mode} mode={mode}
                        selected={captureMode === mode} onSelect={() => setCaptureMode(mode)} onHover={setCaptureHover} />)}
                      <span className="debug-capture-separator" aria-hidden="true" />
                      {captureOrder.slice(2).map((mode) => <CaptureModeButton key={mode} mode={mode}
                        selected={captureMode === mode} onSelect={() => setCaptureMode(mode)} onHover={setCaptureHover} />)}
                    </div>
                    <button type="button" className="debug-capture-settings-row" aria-expanded={captureSettings}
                      onClick={() => setCaptureSettings(!captureSettings)}>
                      <span className="debug-capture-value">{captureFormat.toUpperCase()}</span>
                      <span className="debug-capture-dot">·</span>
                      <span className="debug-capture-value">{activeFps}HZ</span>
                      <span className="debug-capture-dot">·</span>
                      <span className="debug-capture-cursor-label">{captureCursor ? '录制鼠标指针' : '不录制指针'}</span>
                      <Icon src={caret} />
                    </button>
                    {captureSettings && <div className="debug-capture-settings">
                      <div className="debug-capture-setting">
                        <span>格式</span>
                        <div className="debug-capture-segments">
                          {captureFormats.map((format) => <button key={format} type="button" aria-pressed={captureFormat === format}
                            onClick={() => setCaptureFormat(format)}>{format.toUpperCase()}</button>)}
                        </div>
                      </div>
                      <div className="debug-capture-setting">
                        <span>帧率</span>
                        <div className="debug-capture-segments">
                          {captureRates.map((rate) => <button key={rate} type="button" aria-pressed={captureRate === rate}
                            onClick={() => setCaptureRate(rate)}>{rate}HZ</button>)}
                        </div>
                      </div>
                      <label className="debug-capture-cursor">
                        <input type="checkbox" checked={captureCursor} onChange={(event) => setCaptureCursor(event.target.checked)} />
                        录制鼠标指针
                      </label>
                    </div>}
                    <button type="button" className="debug-capture-start" onClick={beginCapture}>{captureAction}</button>
                  </Popover.Dialog></Popover.Content>
                </Popover>
              </Hint>
            </span>
            {pill !== 'idle' && <button type="button" className="debug-capture-pill"
              aria-label={pill === 'recording' ? '停止录制' : pill === 'exporting' ? '正在导出' : '已保存'}
              disabled={pill === 'exporting'}
              onClick={() => {
                if (pill === 'recording') void stopRecording(true);
                else if (pill === 'saved') resetCapture();
              }}>
              <span className="debug-capture-pill-layer" data-content="record">
                <img className="debug-capture-pill-icon" src={recordFull} alt="" />
                <span className="debug-capture-pill-time">
                  {pill === 'exporting' ? `导出 ${Math.round(exportProgress * 100)}%` : formatDuration(recordTime)}
                </span>
              </span>
              <span className="debug-capture-pill-layer" data-content="saved">
                <img className="debug-capture-pill-icon" src={checkIcon} alt="" />
                <span className="debug-capture-pill-saved">{savedLabel}</span>
              </span>
            </button>}
          </div>
          <Hint label="设置" keys={shortcut('E')} hidden={controlsOpen}>
          <Popover isOpen={controlsOpen} onOpenChange={setControlsOpen}>
            <Button isIconOnly variant="ghost" className="debug-icon" aria-label="预览控制"><Icon src={controls} /></Button>
            <Popover.Content placement="bottom end" className="debug-popover-wrap"><Popover.Dialog className="debug-popover debug-preview-menu" aria-label="预览控制">
              <div className="debug-menu-actions">
                <Button size="sm" variant="secondary" onPress={() => setGeneration((value) => value + 1)}>重载</Button>
                <Button size="sm" variant="secondary" onPress={async () => {
                  try { await navigator.clipboard.writeText(location.href); setNotice('已复制预览链接'); }
                  catch { setNotice('无法复制预览链接'); }
                }}>复制链接</Button>
                <Button size="sm" variant="secondary" isDisabled={!snapshot.nodes.length} onPress={saveScreenshot}>截图</Button>
              </div>
              <span className="debug-section-title">{scene ? "当前场景" : "应用"}</span>
              <span>{scene?.title ?? project?.title ?? '等待项目加载'}</span>
              <Switch isSelected={autoRefresh} onChange={setAutoRefresh}>
                <Switch.Control><Switch.Thumb /></Switch.Control><Switch.Content>保存后自动刷新</Switch.Content>
              </Switch>
              <footer>小米手环 10 Pro · 336 × 480</footer>
              <span role="status">{notice}</span>
            </Popover.Dialog></Popover.Content>
          </Popover>
          </Hint>
        </div>
      </header>

      <div className="debug-stage">
        {projectError && <p role="alert" className="debug-empty">无法读取项目配置。</p>}
        {hasPreview && project && <div className="debug-device-canvas" data-ruler={ruler}>
          <div className="debug-device-frame" style={{ width: width * zoom, aspectRatio: width + ' / ' + snapshot.height }}>
            <iframe key={previewUrl} ref={preview} width={width} height={480} title={(scene?.title ?? project.title) + '预览'} src={previewUrl}
              onLoad={() => setFrameReady((value) => value + 1)} />
            {highlighted && !highlighted.hidden && <div className="debug-highlight" style={{
              left: highlighted.x / width * 100 + '%', top: highlighted.y / snapshot.height * 100 + '%',
              width: highlighted.width / width * 100 + '%', height: highlighted.height / snapshot.height * 100 + '%',
            }} />}
            {picking && <div className="debug-pick-surface" onPointerMove={(event) => hover(findAt(event)?.id)}
              onPointerLeave={() => {
                hover(undefined);
                setMeasure(undefined);
              }} onPointerDown={(event) => {
                const node = findAt(event);
                if (node) select(node.id);
                hover(undefined);
                setPicking(false);
              }} />}
          </div>
          {ruler && <div className="debug-rulers">
            <span className="debug-ruler-corner">px</span>
            <Ruler axis="y" size={snapshot.height} cursor={measure?.y} />
            <Ruler axis="x" size={width} cursor={measure?.x} />
          </div>}
          {areaMode && <div className="debug-area-surface" onPointerDown={startArea} onPointerMove={moveArea}
            onPointerUp={endArea} onPointerCancel={() => { areaStart.current = undefined; setArea(undefined); }}>
            {area && <div className="debug-area-box" style={{
              left: area.x / width * 100 + '%', top: area.y / snapshot.height * 100 + '%',
              width: area.width / width * 100 + '%', height: area.height / snapshot.height * 100 + '%',
            }} />}
          </div>}
        </div>}
      </div>
      {areaMode && <div className="debug-area-hint" role="status">{areaMode === 'shot' ? '拖拽选择截图区域' : '拖拽选择录屏区域'} · Esc 取消</div>}
      {captureNotice && <div className="debug-capture-notice" role="status">{captureNotice}</div>}
      {(status === 'building' || status === 'failed') && <div className="debug-update" data-state={status} data-compact={compactStatus}
        ref={statusRef} role={status === 'failed' ? 'alert' : 'status'} aria-label={status === 'building' ? '正在更新预览' : undefined}>
        <div className="debug-update-pane" data-pane="loading" aria-hidden={status !== 'building'}>
          <span className="debug-update-track">
            <span className="debug-update-bar" />
            <span className="debug-update-bar" />
          </span>
        </div>
        <div className="debug-update-pane" data-pane="failed" aria-hidden={status !== 'failed'} inert={status !== 'failed'}>
          <div className="debug-update-message">
            <Icon src={prohibitRed} />
            <span>{hasPreview ? '由于构建失败，当前预览未刷新' : '构建失败，暂时没有可预览的版本'}</span>
          </div>
          <div className="debug-update-actions">
            <span className="debug-update-errors">
              {errorCount > 0 ? <><Icon src={errorDot} />{errorCount}个错误</> : '构建失败'}
            </span>
            <button type="button" className="debug-update-action" onClick={() => openView('控制台')}>
              查看详情<span className="debug-update-key">enter</span>
            </button>
            <button type="button" className="debug-update-action debug-update-copy" onClick={() => void copyPrompt()}>
              <span>{copyState === 'copying' ? '复制中…' : copyState === 'copied' ? '已复制' : copyState === 'failed' ? '复制失败' : '复制提示词'}</span>
              <span className="debug-update-keys">
                {apple
                  ? <span className="debug-update-key debug-update-key-icon"><Icon src={commandKey} /></span>
                  : <span className="debug-update-key">Ctrl</span>}
                <span className="debug-update-key" data-dim="true">C</span>
              </span>
            </button>
          </div>
        </div>
      </div>}
      <span className="debug-sr-only" role="status">{copyState === 'copied' ? '提示词已复制' : copyState === 'failed' ? '无法复制提示词' : copyState === 'copying' ? '正在复制提示词' : ''}</span>
    </section>

    <aside className="debug-inspector" aria-label="检查器" data-console={view === '控制台'} data-placement={placement}
      aria-hidden={!inspectorOpen} inert={!inspectorOpen}>
      <div className="debug-inspector-toolbar">
        <div className="debug-toolbar-slot">
          <Button isIconOnly variant="ghost" className="debug-icon debug-picker" aria-label="从画布选取元素" aria-pressed={picking}
            onPress={() => {
              setPicking(!picking);
              hover(undefined);
              setView('元素');
            }}><Icon src={picker} /></Button>
        </div>
        <nav className="debug-main-tabs" aria-label="调试面板">
          <div className="debug-main-tabs-inner" ref={tabList}>
            <span className="debug-tab-indicator" aria-hidden="true" data-ready={indicatorReady}
              style={{ transform: `translate(${indicator.left}px, -50%)`, width: indicator.width }} />
            {views.map((name, index) => <span className="debug-main-tab" key={name}>
              {index > 0 && <span className="debug-tab-divider" aria-hidden="true" />}
              <Button variant="ghost" className="debug-pill" aria-pressed={view === name} onPress={() => openView(name)}>
                {name}
                {name === '控制台' && errorCount > 0 && <span className="debug-errors"><Icon src={errorIcon} />{errorCount}</span>}
              </Button>
            </span>)}
          </div>
        </nav>
        <div className="debug-toolbar-slot" data-end="true">
          <Popover isOpen={dockOpen} onOpenChange={setDockOpen}>
            <Button isIconOnly variant="ghost" className="debug-icon debug-picker" aria-label="检查器位置"><Icon src={placementInfo[placement].icon} /></Button>
            <Popover.Content placement="bottom end" className="debug-popover-wrap"><Popover.Dialog aria-label="检查器位置" className="debug-popover">
              <span className="debug-section-title">检查器位置</span>
              {placements.map((option) => <Button key={option} variant="ghost" className="debug-menu-item debug-placement"
                aria-pressed={placement === option} onPress={() => { setPlacement(option); setDockOpen(false); }}>
                <Icon src={placementInfo[option].icon} />{placementInfo[option].title}
              </Button>)}
            </Popover.Dialog></Popover.Content>
          </Popover>
        </div>
      </div>
      {view === '元素' && <Inspector key={previewUrl} nodes={snapshot.nodes} selected={selected} select={select} hover={hover} />}
      <Console logs={logs} clearLogs={() => setLogs([])} hidden={view !== '控制台'} />
      {view === '源代码' && <div className="debug-panel">{source ? <><h2>{source.name}</h2><pre>{source.text}</pre></> : <p>暂无源码。</p>}</div>}
      {view === '资源' && <div className="debug-panel"><h2>当前画面</h2><dl className="debug-properties">
        <div><dt>图片对象</dt><dd>{snapshot.nodes.filter((node) => node.kind === 2 && !node.hidden).length}</dd></div>
      </dl></div>}
      {view === '性能' && <div className="debug-panel"><h2>浏览器预览</h2><dl className="debug-properties">
        <div><dt>对象数</dt><dd>{snapshot.nodes.length}</dd></div>
        <div><dt>最近绘制间隔</dt><dd>{snapshot.frameTime ? snapshot.frameTime.toFixed(1) + ' ms' : '—'}</dd></div>
        <div><dt>WASM 内存</dt><dd>{snapshot.memoryBytes ? (snapshot.memoryBytes / 1048576).toFixed(1) + ' MiB' : '—'}</dd></div>
      </dl></div>}
      {view === '图层' && <div className="debug-panel">
        {snapshot.nodes.filter((node) => !node.hidden).reverse().map((node) =>
          <Button key={node.id} variant="ghost" className="debug-layer" onHoverStart={() => hover(node.id)} onHoverEnd={() => hover(undefined)}
            onPress={() => { select(node.id); setView('元素'); }}>{nodeName(node)} <span>{node.width} × {node.height}</span></Button>)}
      </div>}
    </aside>
    {inspectorOpen && placement !== 'bottom' && <div className="debug-resizer" data-placement={placement} role="separator"
      aria-orientation="vertical" aria-label="调整检查器宽度" onPointerDown={resize} />}
  </main>;
}
