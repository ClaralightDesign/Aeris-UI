import { Button, Popover } from '@heroui/react';
import { NumberInput, ScrollArea } from '@claralight-design/react';
import { ArrowClockwiseIcon } from '@phosphor-icons/react/dist/csr/ArrowClockwise';
import { BroomIcon } from '@phosphor-icons/react/dist/csr/Broom';
import { CameraPlusIcon } from '@phosphor-icons/react/dist/csr/CameraPlus';
import { CaretDownIcon } from '@phosphor-icons/react/dist/csr/CaretDown';
import { CheckIcon } from '@phosphor-icons/react/dist/csr/Check';
import { CircleIcon } from '@phosphor-icons/react/dist/csr/Circle';
import { CodeBlockIcon } from '@phosphor-icons/react/dist/csr/CodeBlock';
import { CommandIcon } from '@phosphor-icons/react/dist/csr/Command';
import { FadersIcon } from '@phosphor-icons/react/dist/csr/Faders';
import { HammerIcon } from '@phosphor-icons/react/dist/csr/Hammer';
import { LinkIcon } from '@phosphor-icons/react/dist/csr/Link';
import { MagnifyingGlassPlusIcon } from '@phosphor-icons/react/dist/csr/MagnifyingGlassPlus';
import { ProhibitInsetIcon } from '@phosphor-icons/react/dist/csr/ProhibitInset';
import { RectangleIcon } from '@phosphor-icons/react/dist/csr/Rectangle';
import { RectangleDashedIcon } from '@phosphor-icons/react/dist/csr/RectangleDashed';
import { RulerIcon } from '@phosphor-icons/react/dist/csr/Ruler';
import { SidebarSimpleIcon } from '@phosphor-icons/react/dist/csr/SidebarSimple';
import { TerminalWindowIcon } from '@phosphor-icons/react/dist/csr/TerminalWindow';
import { Fragment, useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import Scritto from '@scritto/react';
import { Inspector } from './inspector';
import { DevToolsTabs } from './devtools-tabs';
import { BoxHighlight, ElementCard } from './element-overlay';
import { LayoutLens, LayoutSelection, layoutAreaForDevice, useLayoutModifier, type Point as LayoutPoint, type Rect as LayoutRect } from './layout-lens';
import { Console, logLevel, type Log } from './console';
import { Sources } from './sources';
import { Smooth, popoverStroke } from './smooth';
import { MAX_ZOOM, MIN_ZOOM, useCanvasViewport } from './canvas-viewport';
import { defaultDevice, nodeName, pickNode, type DeviceProfile, type PreviewWindow, type Snapshot } from './preview';
import logo from './assets/VerisDevBox.svg';
import picker from './assets/PaperPlaneTilt.svg';
import recordFull from './assets/RecordFull.svg';
import recordArea from './assets/RecordArea.svg';
import tooltipArrow from './assets/TooltipArrow.svg';
import { downloadBlob, encodeAnimatedWebp, encodeGif, encodeScreenshot, type RecordedFrame } from './export';
import './workspace.css';

const views = ['元素', '控制台', '源代码', '资源', '性能', '图层'] as const;
const copyPromptLabels = { idle: '复制提示词', copying: '复制中…', copied: '已复制', failed: '复制失败' };

function BuildActionLabel({ value, labels }: { value: string; labels: string[] }) {
  const current = useRef<HTMLSpanElement>(null);
  const [labelWidth, setLabelWidth] = useState(0);
  useLayoutEffect(() => {
    const label = current.current;
    if (!label) return;
    const measure = () => setLabelWidth(label.getBoundingClientRect().width);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(label);
    return () => observer.disconnect();
  }, [value]);
  return <span className="debug-update-action-label" style={{ width: labelWidth }}>
    {labels.map((label) => <span key={label} data-current={label === value}
      ref={label === value ? current : undefined}
      aria-hidden={label !== value}>{label}</span>)}
  </span>;
}
type View = typeof views[number];
type Status = 'ready' | 'building' | 'failed';
type Project = {
  title: string;
  scenes: { id: string; title: string }[];
  devices?: DeviceProfile[];
  status: Status;
  hasPreview: boolean;
  logs: Omit<Log, 'source'>[];
  demo?: boolean;
  app?: { id?: string; module?: string; version?: string; versionCode?: number; abi?: string; package?: string; targets?: string[] };
  tools?: {
    devbox?: string;
    lvgl?: string;
    emscripten?: string;
    dependencies?: { name: string; version: string; revision?: string; local: boolean }[];
  };
};
type Source = { name: string; text: string };
type AgentAccess = 'cli' | 'mcp';
type AgentSettings = Record<AgentAccess, boolean> & { cliSession?: string; mcpConfig?: object };

function PreviewDetails({ title, children }: { title: string; children: ReactNode }) {
  return <details className="debug-preview-info">
    <summary><span>{title}</span><CaretDownIcon size={12} aria-hidden="true" /></summary>
    <ScrollArea orientation="vertical" edge="mask" scrollbars="hidden" radius="none"
      className="debug-preview-info-scroll" viewportClassName="debug-preview-info-viewport"
      contentClassName="debug-preview-info-body">
      {children}
    </ScrollArea>
  </details>;
}

function PreviewInfoRows({ rows }: { rows: readonly (readonly [string, ReactNode])[] }) {
  return <dl>{rows.map(([label, value], index) =>
    <div key={`${label}-${index}`}><dt>{label}</dt><dd>{value}</dd></div>)}</dl>;
}

function PreviewInfo({ project, scene, device, height, zoom, status, agentSettings }: {
  project?: Project;
  scene?: string;
  device: DeviceProfile;
  height: number;
  zoom: number;
  status: Status;
  agentSettings?: AgentSettings;
}) {
  const { app, tools, demo } = project ?? {};
  const cliSession = agentSettings?.cli ? agentSettings.cliSession : undefined;
  const mcpConfig = agentSettings?.mcp ? agentSettings.mcpConfig : undefined;
  const displayedMcpConfig = mcpConfig && JSON.stringify(mcpConfig, (key, value) =>
    key === 'Authorization' ? 'Bearer ••••••••' : value, 2);
  const dependencies = Object.entries({
    'canopus-runtime': 'Canopus Runtime', 'canopus-abi': 'Canopus ABI crate', 'aeris-ui': 'AerisUI',
  }).flatMap<readonly [string, ReactNode]>(([name, label]) => {
    const matches = tools?.dependencies?.filter((item) => item.name === name) ?? [];
    return matches.length ? matches.map(({ version, revision, local }) => [
      label, <span title={revision}>{version}{revision ? ` · ${revision.slice(0, 8)}` : local ? ' · 本地' : ''}</span>,
    ] as const) : [[label, '—'] as const];
  });
  const groups: { title: string; rows: (readonly [string, ReactNode])[] }[] = [
    { title: '设备', rows: [
      ['机型', device.name],
      ['屏幕分辨率', `${device.width} × ${device.height} px`],
      ...(height === device.height ? [] : [['预览区域', `${device.width} × ${height} px`] as const]),
      ['画布缩放', `${zoom.toFixed(1)}%`],
    ] },
    { title: '应用', rows: [
      ['名称', project?.title ?? '等待项目加载'],
      ...(scene ? [['场景', scene] as const] : []),
      ...(demo ? [['运行模式', 'Gallery Demo'] as const] : []),
      ['应用 ID', app?.id ?? '—'],
      ...(app?.module && app.module !== app.id ? [['模块 ID', app.module] as const] : []),
      ['应用版本', app?.version ?? '—'],
      ...(app?.versionCode !== undefined ? [['构建编号', app.versionCode] as const] : []),
      ...(app?.package ? [['Cargo 包', app.package] as const] : []),
      ...(app?.targets?.length ? [['声明的设备目标', app.targets.join('、')] as const] : []),
      ['构建状态', { ready: '就绪', building: '构建中', failed: '构建失败' }[status]],
    ] },
  ];
  const toolRows: (readonly [string, ReactNode])[] = [
    ['DevBox', tools?.devbox ?? '—'],
    ...dependencies,
    ...(app?.abi ? [['应用 ABI 版本', app.abi] as const] : []),
    ['渲染引擎', `LVGL${tools?.lvgl ? ` ${tools.lvgl}` : ''}`],
    ['运行目标', 'wasm32-unknown-emscripten'],
    ['Emscripten', tools?.emscripten ?? '—'],
  ];
  return <>
    <PreviewDetails title="设备、应用与版本">
      {groups.map(({ title, rows }) => <section key={title}>
        <h2>{title}</h2>
        <PreviewInfoRows rows={rows} />
      </section>)}
    </PreviewDetails>
    <PreviewDetails title="工具链">
      <PreviewInfoRows rows={toolRows} />
    </PreviewDetails>
    {cliSession && <PreviewDetails title="Agent CLI 会话路径">
      <pre className="debug-preview-connection">{cliSession}</pre>
    </PreviewDetails>}
    {displayedMcpConfig && <PreviewDetails title="MCP 配置">
      <pre className="debug-preview-connection">{displayedMcpConfig}</pre>
    </PreviewDetails>}
  </>;
}

const agentOptions = [
  { kind: 'cli', label: 'Agent CLI', icon: TerminalWindowIcon, copy: '复制会话路径' },
  { kind: 'mcp', label: 'MCP', icon: LinkIcon, copy: '复制 MCP 配置' },
] as const;
const EMPTY_SNAPSHOT: Snapshot = { height: 480, nodes: [], frameTime: 0, memoryBytes: 0 };
const RULER_STEP = 10;
const RULER_LABEL = 50;
const RULER_EDGE = 40;
const INSPECTOR_MIN = 278;
const INSPECTOR_PULL = 14;
const placements = ['left', 'right', 'bottom'] as const;
const narrowInspectorQuery = '(max-width: 760px)';
type Placement = typeof placements[number];
const placementLabels: Record<Placement, string> = { left: '左侧', right: '右侧', bottom: '下侧' };

function PlacementIcon({ placement }: { placement: Placement }) {
  return <SidebarSimpleIcon className="debug-placement-icon" data-placement={placement} size={16} aria-hidden="true" />;
}

type CaptureMode = 'screen-shot' | 'area-shot' | 'screen-video' | 'area-video';
type CaptureFormat = 'mp4' | 'webp' | 'gif';
type ScreenshotFormat = 'png' | 'webp' | 'jpg';
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
const captureModes: Record<CaptureMode, { title: string; action: string }> = {
  'screen-shot': { title: '全屏截图', action: '开始截图' },
  'area-shot': { title: '区域截图', action: '开始截图' },
  'screen-video': { title: '全屏录屏', action: '开始录制' },
  'area-video': { title: '区域录屏', action: '开始录制' },
};
const captureOrder: CaptureMode[] = ['screen-shot', 'area-shot', 'screen-video', 'area-video'];
const captureFormats: CaptureFormat[] = ['mp4', 'webp', 'gif'];
const screenshotFormats: ScreenshotFormat[] = ['png', 'webp', 'jpg'];
const imageTypes: Record<ScreenshotFormat, string> = { png: 'image/png', webp: 'image/webp', jpg: 'image/jpeg' };
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
  return <Smooth radius={12}><button type="button" className="debug-capture-mode" aria-pressed={selected} aria-label={captureModes[mode].title}
    onMouseEnter={() => onHover(mode)} onMouseLeave={() => onHover(undefined)}
    onFocus={() => onHover(mode)} onBlur={() => onHover(undefined)} onClick={onSelect}>
    {mode === 'screen-shot' ? <RectangleIcon className="debug-capture-icon" size={24} aria-hidden="true" />
      : mode === 'area-shot' ? <RectangleDashedIcon className="debug-capture-icon" size={24} aria-hidden="true" />
      : <img className="debug-capture-icon" src={mode === 'screen-video' ? recordFull : recordArea} alt="" />}
  </button></Smooth>;
}

function ZoomPercentInput({ percent, onChange }: { percent: number; onChange: (percent: number) => void }) {
  const id = useId();
  function apply(value: number | null) {
    if (value !== null) onChange(Math.round(value * 10) / 10);
  }
  return <div className="debug-zoom-field" onKeyDown={(event) => {
    if (event.key === 'Enter' && event.target instanceof HTMLInputElement) {
      event.preventDefault();
      event.target.blur();
    }
  }}>
    <label className="debug-sr-only" htmlFor={id}>缩放百分比</label>
    <NumberInput id={id} value={Math.round(percent * 10) / 10} min={MIN_ZOOM} max={MAX_ZOOM} step={1}
      prefix={<span title="按住并左右拖动调整缩放" className="debug-zoom-scrub-label">缩放</span>}
      suffix="%" format={{ maximumFractionDigits: 1 }}
      onValueChange={(value, reason) => { if (reason === 'scrub') apply(value); }}
      onValueCommitted={apply} onCancel={apply} />
  </div>;
}

function Hint({ label, keys, children, hidden }: { label: string; keys?: ReactNode; children: ReactNode; hidden?: boolean }) {
  const host = useRef<HTMLSpanElement>(null);
  const tooltip = useRef<HTMLSpanElement>(null);
  const shadowId = useId();
  const [open, setOpen] = useState(false);
  const [anchor, setAnchor] = useState({ top: 0, x: 0 });
  const [placement, setPlacement] = useState({ center: 0, arrowOffset: 0 });
  useEffect(() => {
    if (hidden) setOpen(false);
  }, [hidden]);
  function show() {
    if (hidden) return;
    const rect = host.current?.getBoundingClientRect();
    if (!rect) return;
    setAnchor({ top: rect.bottom + 6, x: rect.left + rect.width / 2 });
    setOpen(true);
  }
  useLayoutEffect(() => {
    const element = tooltip.current;
    if (!element) return;
    function place(width: number) {
      const inset = 8;
      const half = width / 2;
      const center = Math.max(half + inset, Math.min(document.documentElement.clientWidth - half - inset, anchor.x));
      const limit = Math.max(0, half - 11 - inset);
      setPlacement({ center, arrowOffset: Math.max(-limit, Math.min(limit, anchor.x - center)) });
    }
    place(element.getBoundingClientRect().width);
    const observer = new ResizeObserver(() => place(element.getBoundingClientRect().width));
    observer.observe(element);
    return () => observer.disconnect();
  }, [anchor.x, open, hidden]);
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
      <span ref={tooltip} className="debug-tooltip" style={{ top: anchor.top, left: placement.center }} role="tooltip">
        <svg className="debug-tooltip-filter" width="0" height="0" aria-hidden="true">
          <defs>
            <filter id={shadowId} colorInterpolationFilters="sRGB">
              {/* The exported arrow and the body both have a 25% white fill. */}
              <feComponentTransfer in="SourceAlpha" result="silhouette">
                <feFuncA type="linear" slope={4} />
              </feComponentTransfer>
              {([{ dy: -1, blur: 1, opacity: .1 }, { dy: 1, blur: 1, opacity: .3 }, { dy: 1, blur: 3, opacity: .1 }]).map(
                ({ dy, blur, opacity }, index) => <Fragment key={index}>
                  <feOffset in="silhouette" dy={dy} result="offset" />
                  <feGaussianBlur in="offset" stdDeviation={blur / 2} result="blurred" />
                  <feComposite in="silhouette" in2="blurred" operator="out" result="edge" />
                  <feFlood floodColor="white" floodOpacity={opacity} result="highlight" />
                  <feComposite in="highlight" in2="edge" operator="in" result={`shadow${index}`} />
                </Fragment>,
              )}
              <feMerge>
                <feMergeNode in="SourceGraphic" />
                <feMergeNode in="shadow2" />
                <feMergeNode in="shadow1" />
                <feMergeNode in="shadow0" />
              </feMerge>
            </filter>
          </defs>
        </svg>
        <span className="debug-tooltip-surface" aria-hidden="true" style={{ filter: `url("#${shadowId}")` }}>
          <img src={tooltipArrow} width={22} height={8} alt="" style={{ transform: `translateX(${placement.arrowOffset}px)` }} />
          <Smooth radius={8}><span className="debug-tooltip-fill" /></Smooth>
        </span>
        <Smooth radius={8}><span className="debug-tooltip-body" data-shortcut={!!keys}>
          <span className="debug-tooltip-label">{label}</span>
          {keys && <span className="debug-tooltip-keys">{keys}</span>}
        </span></Smooth>
      </span>,
      document.body,
    )}
  </span>;
}

function Ruler({ axis, size, cursor }: { axis: 'x' | 'y'; size: number; cursor?: number }) {
  const ticks = Array.from({ length: Math.floor(size / RULER_STEP) + 1 }, (_, index) => index * RULER_STEP);
  const place = (value: number) => axis === 'x' ? { left: value / size * 100 + '%' } : { top: value / size * 100 + '%' };
  const edge = cursor === undefined ? undefined : cursor < RULER_EDGE ? 'start' : cursor > size - RULER_EDGE ? 'end' : 'center';
  return <div className="debug-ruler" data-axis={axis} aria-hidden="true">
    {ticks.map((value) => <span key={value} className="debug-ruler-tick" data-major={value % RULER_LABEL === 0} style={place(value)}>
      {value > 0 && value % RULER_LABEL === 0 && value < size - RULER_EDGE
        && (cursor === undefined || Math.abs(value - cursor) >= RULER_EDGE / 2) && <em>{value}</em>}
    </span>)}
    {cursor !== undefined && <>
      <span className="debug-ruler-cursor" style={place(cursor)} />
      <Smooth radius={2}><span className="debug-ruler-value" data-edge={edge} style={place(cursor)}>{Math.round(cursor)}</span></Smooth>
    </>}
    {(cursor === undefined || cursor < size - RULER_EDGE) && <Smooth radius={2}><span className="debug-ruler-size">{Math.round(size)}</span></Smooth>}
  </div>;
}

export function Component() {
  const [project, setProject] = useState<Project>();
  const [projectError, setProjectError] = useState(false);
  const [initialScene] = useState(() => new URLSearchParams(location.search).get('scene'));
  const [deviceId, setDeviceId] = useState<string>();
  const [scenes, setScenes] = useState<Record<string, string>>({});
  const devices = project?.devices ?? [defaultDevice];
  const device = devices.find(({ id }) => id === deviceId) ?? devices[0];
  const { width } = device;
  const sceneFor = (id: string) => project?.scenes.find((item) => item.id === (scenes[id] ?? initialScene)) ?? project?.scenes[0];
  const scene = sceneFor(device.id);
  const [hasPreview, setHasPreview] = useState(false);
  const [sourceRevision, setSourceRevision] = useState(0);
  const [deviceSnapshots, setDeviceSnapshots] = useState<Record<string, { url: string; snapshot: Snapshot }>>({});
  const [deviceCrops, setDeviceCrops] = useState<Record<string, { url: string; height: number }>>({});
  const [generation, setGeneration] = useState(0);
  const [reloads, setReloads] = useState<Record<string, number>>({});
  const [status, setStatus] = useState<Status>('ready');
  const [allLogs, setLogs] = useState<Log[]>([]);
  const logs = useMemo(() => allLogs.filter((entry) => !entry.device || entry.device === device.id), [allLogs, device.id]);
  const [notice, setNotice] = useState('');
  const [view, setView] = useState<View>('元素');
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [picking, setPicking] = useState(false);
  const [scenesOpen, setScenesOpen] = useState(false);
  const [ruler, setRuler] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(() => localStorage.getItem('aeris-debug-auto-refresh') !== 'off');
  const [selectedId, select] = useState<number>();
  const [canvasSelection, setCanvasSelection] = useState<number>();
  const [hoverId, hover] = useState<number>();
  const [measure, setMeasure] = useState<{ x: number; y: number }>();
  const [frameReady, setFrameReady] = useState(0);
  const [preferredPlacement, setPlacement] = useState<Placement>(() => {
    const saved = localStorage.getItem('aeris-debug-inspector-placement');
    return saved === 'left' || saved === 'bottom' ? saved : 'right';
  });
  const [narrowInspector, setNarrowInspector] = useState(() => matchMedia(narrowInspectorQuery).matches);
  const placement = narrowInspector ? 'bottom' : preferredPlacement;
  const [inspectorWidth, setInspectorWidth] = useState(initialInspectorWidth);
  const [resizing, setResizing] = useState(false);
  const [dockOpen, setDockOpen] = useState(false);
  const [copyState, setCopyState] = useState<'idle' | 'copying' | 'copied' | 'failed'>('idle');
  const [compactStatus, setCompactStatus] = useState(false);
  const [captureOpen, setCaptureOpen] = useState(false);
  const [captureMode, setCaptureMode] = useState<CaptureMode>('screen-video');
  const [captureHover, setCaptureHover] = useState<CaptureMode>();
  const [captureFormat, setCaptureFormat] = useState<CaptureFormat>('mp4');
  const [screenshotFormat, setScreenshotFormat] = useState<ScreenshotFormat>('png');
  const [captureRate, setCaptureRate] = useState(60);
  const [captureCursor, setCaptureCursor] = useState(true);
  const [captureSettings, setCaptureSettings] = useState(false);
  const [zoomOpen, setZoomOpen] = useState(false);
  const [controlsOpen, setControlsOpen] = useState(false);
  const [agentSettings, setAgentSettings] = useState<AgentSettings>();
  const [agentUpdating, setAgentUpdating] = useState(false);
  const [buildRequest, setBuildRequest] = useState(false);
  const [areaMode, setAreaMode] = useState<'shot' | 'video'>();
  const [area, setArea] = useState<CaptureRect>();
  const [recording, setRecording] = useState<{ mode: 'screen' | 'area'; format: CaptureFormat; startedAt: number }>();
  const [recordTime, setRecordTime] = useState(0);
  const [pill, setPill] = useState<'idle' | 'recording' | 'exporting' | 'saved'>('idle');
  const [savedLabel, setSavedLabel] = useState('');
  const [exportProgress, setExportProgress] = useState(0);
  const [captureNotice, setCaptureNotice] = useState('');
  const restored = useRef(true);
  const copyTimer = useRef<number>(undefined);
  const statusRef = useRef<HTMLDivElement>(null);
  const session = useRef<RecordingSession | undefined>(undefined);
  const areaStart = useRef<{ x: number; y: number } | undefined>(undefined);
  const pointer = useRef<{ x: number; y: number } | undefined>(undefined);
  const noticeTimer = useRef<number>(undefined);
  const savedTimer = useRef<number>(undefined);
  const stage = useRef<HTMLDivElement>(null);
  const preview = useRef<HTMLIFrameElement>(null);
  const frames = useRef(new Map<string, HTMLIFrameElement>());
  const activateDevice = useRef<(id: string) => void>(() => {});
  activateDevice.current = changeDevice;
  const refreshEnabled = useRef(autoRefresh);

  useEffect(() => {
    if (!controlsOpen) return;
    const controller = new AbortController();
    const { signal } = controller;
    fetch('/devbox-agent-settings', { signal, cache: 'no-store' })
      .then((response) => {
        if (!response.ok) throw new Error();
        return response.json() as Promise<AgentSettings>;
      })
      .then((settings) => { if (!signal.aborted) setAgentSettings(settings); })
      .catch(() => { if (!signal.aborted) setNotice('无法读取 Agent 设置'); });
    return () => controller.abort();
  }, [controlsOpen]);

  async function toggleAgent(kind: AgentAccess) {
    if (!agentSettings || agentUpdating) return;
    setAgentUpdating(true);
    setNotice('');
    try {
      const response = await fetch('/devbox-agent-settings', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ kind, enabled: !agentSettings[kind] }),
      });
      if (!response.ok) throw new Error();
      setAgentSettings(await response.json());
    } catch {
      setNotice('无法更新 Agent 设置');
    } finally {
      setAgentUpdating(false);
    }
  }

  async function requestBuild(mode: 'rebuild' | 'clean') {
    setBuildRequest(true);
    setNotice(mode === 'clean' ? '等待清理缓存并重新构建' : '等待重新构建');
    try {
      const response = await fetch('/devbox-build?' + new URLSearchParams({ mode }), { method: 'POST' });
      if (!response.ok) throw new Error();
    } catch {
      setNotice('无法请求重新构建');
    } finally {
      setBuildRequest(false);
    }
  }

  function urlFor(id: string) {
    const params = new URLSearchParams({ scene: sceneFor(id)?.id ?? '', device: id, debug: '1',
      revision: generation + '-' + (reloads[id] ?? 0) });
    return '/preview.html?' + params;
  }
  const previewUrl = urlFor(device.id);
  const savedSnapshot = deviceSnapshots[device.id];
  const snapshot = savedSnapshot?.url === previewUrl ? savedSnapshot.snapshot
    : { ...EMPTY_SNAPSHOT, height: previewHeight(device, previewUrl) };
  function previewHeight(profile: DeviceProfile, url: string) {
    const crop = deviceCrops[profile.id];
    return crop?.url === url ? Math.min(crop.height, profile.height) : profile.height;
  }
  useLayoutEffect(() => {
    preview.current = frames.current.get(device.id) ?? null;
  }, [device.id, previewUrl, hasPreview, project]);
  const { viewport, scale, space, dragging, reset, fit, zoomIn, zoomOut, setPercent } = useCanvasViewport(stage, preview, frameReady, previewUrl);
  const altHeld = useLayoutModifier(frames, frameReady);
  const layoutMode = altHeld && hasPreview && !!project && !areaMode && !space && !dragging && !resizing;
  const [layoutArea, setLayoutArea] = useState<LayoutRect>();
  useEffect(() => { if (!layoutMode) setLayoutArea(undefined); }, [layoutMode]);
  const gap = 36;
  const comparison = devices.length > 1;
  const totalWidth = devices.reduce((sum, profile) => sum + profile.width, 0) + gap * (devices.length - 1);
  const maxHeight = Math.max(...devices.map((profile) => profile.height));
  const deviceList = devices.map(({ id }) => id).join(',');
  function fitDevices() {
    if (comparison) fit(totalWidth + 36, maxHeight + 66);
    else fit(width, snapshot.height);
  }
  function deviceOffset(id: string) {
    let offset = -totalWidth / 2;
    for (const profile of devices) {
      if (profile.id === id) return (offset + profile.width / 2) * scale;
      offset += profile.width + gap;
    }
    return 0;
  }
  function trackLayoutPoint(point?: LayoutPoint) {
    if (!point) {
      setMeasure(undefined);
      return;
    }
    const x = point.x - deviceOffset(device.id) / scale + width / 2;
    const y = point.y + snapshot.height / 2;
    setMeasure(x >= 0 && x <= width && y >= 0 && y <= snapshot.height ? { x, y } : undefined);
  }
  useLayoutEffect(() => {
    if (hasPreview && comparison) fitDevices();
  }, [hasPreview, deviceList]);
  const zoomLabel = `${viewport.percent.toFixed(1)}%`;
  const previousZoom = useRef(viewport.percent);
  const [zoomExpanded, setZoomExpanded] = useState(false);
  useEffect(() => {
    const changed = viewport.percent !== previousZoom.current;
    previousZoom.current = viewport.percent;
    if (zoomOpen || !viewport.showZoomFeedback) {
      setZoomExpanded(false);
      return;
    }
    if (!changed) return;
    setZoomExpanded(true);
    const timer = window.setTimeout(() => setZoomExpanded(false), 1500);
    return () => window.clearTimeout(timer);
  }, [viewport.percent, viewport.showZoomFeedback, zoomOpen]);
  const renderer = () => (preview.current?.contentWindow as PreviewWindow | null)?.aerisDebugger;
  const selected = snapshot.nodes.find((node) => node.id === selectedId) ?? snapshot.nodes[0];
  const inspected = inspectorOpen && view === '元素' && !areaMode && !layoutMode
    ? snapshot.nodes.find((node) => node.id === hoverId && !node.hidden) : undefined;
  const highlighted = inspected ?? (ruler ? selected : undefined);
  const errorCount = logs.filter((entry) => entry.level === 'error').length;

  function append(log: Log) {
    setLogs((previous) => [...previous.slice(-499), log]);
  }

  useEffect(() => {
    const previous = document.title;
    document.title = project?.title ? `${project.title} / Aeris DevBox` : 'Aeris DevBox';
    return () => { document.title = previous; };
  }, [project?.title]);

  useEffect(() => {
    refreshEnabled.current = autoRefresh;
    localStorage.setItem('aeris-debug-auto-refresh', autoRefresh ? 'on' : 'off');
  }, [autoRefresh]);

  useEffect(() => {
    const media = matchMedia(narrowInspectorQuery);
    const update = () => setNarrowInspector(media.matches);
    media.addEventListener('change', update);
    update();
    return () => media.removeEventListener('change', update);
  }, []);

  useEffect(() => {
    if (restored.current) {
      restored.current = false;
      return;
    }
    localStorage.setItem('aeris-debug-inspector-placement', preferredPlacement);
    localStorage.setItem('aeris-debug-inspector-width', String(inspectorWidth));
  }, [preferredPlacement, inspectorWidth]);

  useEffect(() => {
    select(undefined);
    setCanvasSelection(undefined);
    hover(undefined);
    function inspect() {
      const next = renderer()?.snapshot();
      if (next) setDeviceSnapshots((previous) => ({ ...previous, [device.id]: { url: previewUrl, snapshot: next } }));
    }
    inspect();
    const timer = window.setInterval(inspect, 250);
    return () => window.clearInterval(timer);
  }, [previewUrl]);

  useEffect(() => {
    function onMessage(event: MessageEvent) {
      if (event.origin !== location.origin) return;
      const source = [...frames.current].find(([, frame]) => frame.contentWindow === event.source);
      if (!source) return;
      const [device, frame] = source;
      const data = event.data;
      if (data?.type === 'aeris-debug-log' && typeof data.text === 'string') {
        append({ level: logLevel(data.level, data.text), source: '运行', text: data.text, device });
      } else if (data?.type === 'aeris-debug-error') {
        append({ level: 'error', source: '运行', text: '无法加载组件预览。', device });
      } else if (data?.type === 'aeris-preview-size' && Number.isInteger(data.height) && data.height > 0) {
        const url = frame.getAttribute('src');
        if (url) setDeviceCrops((previous) => ({ ...previous, [device]: { url, height: data.height } }));
      }
    }
    window.addEventListener('message', onMessage);
    const hot = import.meta.hot;
    const startup = new AbortController();
    async function loadProject(initial = false, refresh = false) {
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
        } else if (refresh) {
          setGeneration((value) => value + 1);
        }
      } catch {
        if (!startup.signal.aborted) setProjectError(true);
      }
    }
    function build({ status: next, refresh }: { status: Status; refresh?: boolean }) {
      setStatus(next);
      if (refresh) setNotice((current) => current.startsWith('等待')
        ? next === 'ready' ? '构建完成，已刷新预览' : '构建失败，请查看控制台'
        : current);
      if (next === 'ready') {
        setSourceRevision((value) => value + 1);
        setHasPreview(true);
        void loadProject(false, refresh || refreshEnabled.current);
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
    if (renderer()?.contains?.(Math.floor(x), Math.floor(y)) === false) return;
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

  function changeScene(id: string) {
    const url = new URL(location.href);
    url.searchParams.set('scene', id);
    history.replaceState(null, '', url);
    setScenes((previous) => ({ ...previous, [device.id]: id }));
  }

  function changeDevice(id: string) {
    if (id === device.id) return;
    if (session.current) void stopRecording(true);
    setPicking(false);
    setAreaMode(undefined);
    setArea(undefined);
    setDeviceId(id);
  }

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

  async function saveScreenshot(format: ScreenshotFormat, region?: CaptureRect) {
    const mime = imageTypes[format];
    try {
      const url = renderer()?.screenshot();
      if (!url) throw new Error();
      const name = captureBaseName() + (region ? '-area' : '') + '.' + format;
      if (format === 'png' && !region) {
        const link = document.createElement('a');
        link.href = url;
        link.download = name;
        link.click();
      } else {
        const image = new Image();
        image.src = url;
        await image.decode();
        const crop = region ?? { x: 0, y: 0, width: image.naturalWidth, height: image.naturalHeight };
        const canvas = document.createElement('canvas');
        canvas.width = Math.max(1, Math.round(crop.width));
        canvas.height = Math.max(1, Math.round(crop.height));
        const context = canvas.getContext('2d');
        if (!context) throw new Error();
        context.drawImage(image, crop.x, crop.y, crop.width, crop.height, 0, 0, canvas.width, canvas.height);
        downloadBlob(await encodeScreenshot(canvas, mime), name);
      }
      markSaved(`已保存为 ${format.toUpperCase()}`);
    } catch {
      notifyCapture(`无法导出 ${format.toUpperCase()} 截图`);
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
        context.save();
        if (device.shape !== 'rectangle') {
          context.beginPath();
          context.roundRect(-area.x, -area.y, device.width, device.height, Math.min(device.width, device.height) / 2);
          context.clip();
        }
        context.drawImage(current, area.x, area.y, area.width, area.height, 0, 0, canvas.width, canvas.height);
        const point = pointer.current;
        if (sessionValue.cursor && point && point.x >= area.x && point.y >= area.y
          && point.x <= area.x + area.width && point.y <= area.y + area.height) {
          drawPointer(context, point.x - area.x, point.y - area.y);
        }
        context.restore();
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
      void saveScreenshot(screenshotFormat);
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
    if (mode === 'shot') void saveScreenshot(screenshotFormat, region);
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

  const apple = /Mac|iPhone|iPad|iPod/.test(navigator.userAgent);
  const shownCapture = captureHover ?? captureMode;
  const captureTitle = captureModes[shownCapture].title;
  const captureAction = captureModes[captureMode].action;
  const recordingMode = captureMode === 'screen-video' || captureMode === 'area-video';
  const selectedFormat = recordingMode ? captureFormat : screenshotFormat;
  const formats = recordingMode
    ? captureFormats.map((format) => ({ format, select: () => setCaptureFormat(format) }))
    : screenshotFormats.map((format) => ({ format, select: () => setScreenshotFormat(format) }));
  const activeFps = effectiveFps(captureFormat, captureRate);
  const shortcut = (letter: string) => <>
    <Smooth radius={4}><span className="debug-tooltip-key" data-icon={apple ? 'true' : undefined}>
      {apple ? <CommandIcon size={14} aria-hidden="true" /> : 'Ctrl'}
    </span></Smooth>
    <Smooth radius={4}><span className="debug-tooltip-key" data-dim="true">{letter}</span></Smooth>
  </>;

  return <main className="debug-workspace" data-inspector={inspectorOpen} data-placement={placement} data-resizing={resizing}
    style={{ '--debug-inspector-width': (inspectorOpen ? inspectorWidth : 0) + 'px' } as CSSProperties}>
    <Smooth radius={8} stroke={{ width: 1, color: "#ffffff", opacity: 0.14 }}><section className="debug-preview-panel" aria-label="手表预览">
      <header className="debug-toolbar">
        <div className="debug-logo"><img src={logo} alt="Aeris DevBox" /></div>
        {project && project.scenes.length === 0 ? <span className="debug-app-title">{project.title}</span> :
        <Popover isOpen={scenesOpen} onOpenChange={setScenesOpen}>
          <Smooth radius={6}><Button variant="ghost" className="debug-target" aria-label="选择场景">
            <span>{project?.title ?? "Aeris DevBox"}</span>{scene && <span className="debug-target-device"> · {scene.title}</span>}<CaretDownIcon size={12} aria-hidden="true" />
          </Button></Smooth>
          <Popover.Content placement="bottom" className="debug-popover-wrap"><Smooth radius={20} stroke={popoverStroke}><Popover.Dialog aria-label="预览场景" className="debug-popover">
            <span className="debug-section-title">预览场景</span>
            {project?.scenes.map((item) => <Button key={item.id} variant="ghost" className="debug-menu-item"
              aria-pressed={scene?.id === item.id} onPress={() => { changeScene(item.id); setScenesOpen(false); }}>
              {item.title}
            </Button>)}
          </Popover.Dialog></Smooth></Popover.Content>
        </Popover>}
        <div className="debug-tools">
          <Hint label="标尺" keys={shortcut('R')}>
            <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon" aria-label="显示标尺" aria-pressed={ruler} onPress={() => setRuler(!ruler)}><RulerIcon size={18} aria-hidden="true" /></Button></Smooth>
          </Hint>
          <Hint label="缩放" hidden={zoomOpen || zoomExpanded}>
            <Popover isOpen={zoomOpen} onOpenChange={setZoomOpen}>
              <Smooth radius={6}><Button variant="ghost" className="debug-icon debug-zoom-trigger" aria-label="缩放"
                data-expanded={zoomExpanded} style={{ '--debug-zoom-digits': zoomLabel.length - 2 } as CSSProperties}>
                <MagnifyingGlassPlusIcon size={18} aria-hidden="true" />
                <span className="debug-zoom-trigger-value" aria-hidden="true"><Scritto value={zoomLabel} /></span>
              </Button></Smooth>
              <Popover.Content className="debug-popover-wrap"><Smooth radius={20} stroke={popoverStroke}><Popover.Dialog aria-label="缩放" className="debug-popover debug-zoom-menu">
                <ZoomPercentInput percent={viewport.percent} onChange={setPercent} />
                <div className="debug-zoom-actions">
                  {[
                    { label: '放大', key: '+', action: zoomIn },
                    { label: '缩小', key: '−', action: zoomOut },
                    { label: '适应画布', action: fitDevices },
                    { label: '缩放至 50%', action: () => setPercent(50) },
                    { label: '缩放至 100%', key: '0', action: reset },
                    { label: '缩放至 200%', action: () => setPercent(200) },
                  ].map(({ label, key, action }) => <Smooth key={label} radius={8}>
                    <Button variant="ghost" className="debug-menu-item" onPress={() => { action(); setZoomOpen(false); }}>
                      <span>{label}</span>{key && <span className="debug-zoom-shortcut" aria-hidden="true">{apple ? '⌘' : 'Ctrl '}{key}</span>}
                    </Button>
                  </Smooth>)}
                </div>
                <div className="debug-zoom-actions">
                  <Smooth radius={8}><Button variant="ghost" className="debug-menu-item" aria-pressed={ruler}
                    onPress={() => setRuler(!ruler)}>
                    <span>标尺</span><span className="debug-zoom-shortcut" aria-hidden="true">{apple ? '⌘' : 'Ctrl '}R</span>
                  </Button></Smooth>
                </div>
              </Popover.Dialog></Smooth></Popover.Content>
            </Popover>
          </Hint>
          <Hint label="检查面板" keys={shortcut('I')}>
            <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon" aria-label="显示检查面板"
              aria-pressed={inspectorOpen} onPress={() => { setInspectorOpen(!inspectorOpen); setPicking(false); hover(undefined); }}><CodeBlockIcon size={18} aria-hidden="true" /></Button></Smooth>
          </Hint>
          <Smooth radius={6}><div className="debug-capture-slot" data-status={pill}>
            <span className="debug-capture-idle" inert={pill !== 'idle'} aria-hidden={pill !== 'idle'}>
              <Hint label="录屏" keys={shortcut('S')} hidden={captureOpen || pill !== 'idle'}>
                <Popover isOpen={captureOpen} onOpenChange={(open) => {
                  setCaptureOpen(open);
                  if (!open) {
                    setCaptureSettings(false);
                    setCaptureHover(undefined);
                  }
                }}>
                  <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon" aria-label="截图与录屏"
                    isDisabled={!snapshot.nodes.length || pill !== 'idle'}><CameraPlusIcon size={18} aria-hidden="true" /></Button></Smooth>
                  <Popover.Content placement="bottom end" className="debug-capture-popover"><Smooth radius={20} stroke={popoverStroke}><Popover.Dialog className="debug-capture" aria-label="截图与录屏">
                    <span className="debug-capture-title"><Scritto value={captureTitle} /></span>
                    <div className="debug-capture-modes">
                      {captureOrder.slice(0, 2).map((mode) => <CaptureModeButton key={mode} mode={mode}
                        selected={captureMode === mode} onSelect={() => setCaptureMode(mode)} onHover={setCaptureHover} />)}
                      <span className="debug-capture-separator" aria-hidden="true" />
                      {captureOrder.slice(2).map((mode) => <CaptureModeButton key={mode} mode={mode}
                        selected={captureMode === mode} onSelect={() => setCaptureMode(mode)} onHover={setCaptureHover} />)}
                    </div>
                    <Smooth radius={8}><button type="button" className="debug-capture-settings-row" aria-expanded={captureSettings}
                      onClick={() => setCaptureSettings(!captureSettings)}>
                      <Scritto className="debug-capture-value" value={selectedFormat.toUpperCase()} />
                      <span className="debug-capture-video-summary" data-visible={recordingMode} aria-hidden={!recordingMode}>
                        <span>
                          <span className="debug-capture-dot">·</span>
                          <Scritto className="debug-capture-value" value={`${activeFps}HZ`} />
                          <span className="debug-capture-dot">·</span>
                          <Scritto className="debug-capture-cursor-label" value={captureCursor ? '录制鼠标指针' : '不录制指针'} />
                        </span>
                      </span>
                      <CaretDownIcon size={12} aria-hidden="true" />
                    </button></Smooth>
                    {captureSettings && <Smooth radius={10}><div className="debug-capture-settings">
                      <div className="debug-capture-setting">
                        <span>格式</span>
                        <div className="debug-capture-segments">
                          {formats.map(({ format, select }, index) => <Smooth key={index} radius={6}>
                            <button type="button" aria-pressed={selectedFormat === format} aria-label={format.toUpperCase()} onClick={select}>
                              <Scritto value={format.toUpperCase()} aria-hidden="true" />
                            </button>
                          </Smooth>)}
                        </div>
                      </div>
                      <div className="debug-capture-video-settings" data-visible={recordingMode} inert={!recordingMode}>
                        <div>
                          <div className="debug-capture-setting">
                            <span>帧率</span>
                            <div className="debug-capture-segments">
                              {captureRates.map((rate) => <Smooth key={rate} radius={6}>
                                <button type="button" aria-pressed={captureRate === rate} onClick={() => setCaptureRate(rate)}>{rate}HZ</button>
                              </Smooth>)}
                            </div>
                          </div>
                          <label className="debug-capture-cursor">
                            <input className="debug-sr-only" type="checkbox" checked={captureCursor}
                              onChange={(event) => setCaptureCursor(event.target.checked)} />
                            <span className="debug-capture-checkbox">
                              <Smooth radius={4} stroke={{ width: 1, color: '#ffffff', opacity: 0.3 }}>
                                <span className="debug-capture-checkbox-mark"><CheckIcon size={11} weight="bold" aria-hidden="true" /></span>
                              </Smooth>
                            </span>
                            录制鼠标指针
                          </label>
                        </div>
                      </div>
                    </div></Smooth>}
                    <Smooth radius={12}><button type="button" className="debug-capture-start" aria-label={captureAction} onClick={beginCapture}>
                      <Scritto value={captureAction} aria-hidden="true" />
                    </button></Smooth>
                  </Popover.Dialog></Smooth></Popover.Content>
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
                  <Scritto value={pill === 'exporting' ? `导出 ${Math.round(exportProgress * 100)}%` : formatDuration(recordTime)}
                    trend={1} aria-hidden="true" />
                </span>
              </span>
              <span className="debug-capture-pill-layer" data-content="saved">
                <CheckIcon size={18} aria-hidden="true" />
                <span className="debug-capture-pill-saved">{savedLabel}</span>
              </span>
            </button>}
          </div></Smooth>
          <Hint label="设置" keys={shortcut('E')} hidden={controlsOpen}>
            <Popover isOpen={controlsOpen} onOpenChange={setControlsOpen}>
              <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon" aria-label="设置"><FadersIcon size={18} aria-hidden="true" /></Button></Smooth>
              <Popover.Content placement="bottom end" className="debug-popover-wrap">
                <Smooth radius={20} stroke={popoverStroke}><Popover.Dialog className="debug-popover debug-preview-menu" aria-label="设置">
                  <div className="debug-preview-actions">
                    <Hint label="重载"><Smooth radius={12}><Button isIconOnly variant="ghost" aria-label="重载"
                      onPress={() => setReloads((previous) => ({ ...previous, [device.id]: (previous[device.id] ?? 0) + 1 }))}>
                      <ArrowClockwiseIcon size={18} aria-hidden="true" />
                    </Button></Smooth></Hint>
                    <Hint label="截图"><Smooth radius={12}><Button isIconOnly variant="ghost" aria-label="截图"
                      isDisabled={!snapshot.nodes.length} onPress={() => void saveScreenshot('png')}>
                      <CameraPlusIcon size={18} aria-hidden="true" />
                    </Button></Smooth></Hint>
                    <Hint label="重新构建"><Smooth radius={12}><Button isIconOnly variant="ghost" aria-label="重新构建"
                      isDisabled={!project || buildRequest} onPress={() => void requestBuild('rebuild')}>
                      <HammerIcon size={18} aria-hidden="true" />
                    </Button></Smooth></Hint>
                    <Hint label="清理构建缓存"><Smooth radius={12}><Button isIconOnly variant="ghost" aria-label="清理构建缓存"
                      isDisabled={!project || buildRequest} onPress={() => void requestBuild('clean')}>
                      <BroomIcon size={18} aria-hidden="true" />
                    </Button></Smooth></Hint>
                  </div>
                  <div className="debug-preview-section">
                    <div className="debug-preview-context">
                      <h2>{scene ? '当前场景' : '应用'}</h2>
                      <span title={scene?.title ?? project?.title}>{scene?.title ?? project?.title ?? '等待项目加载'}</span>
                    </div>
                    <Smooth radius={12}><button type="button" className="debug-preview-setting" role="switch"
                      aria-checked={autoRefresh} aria-label="保存后自动刷新" onClick={() => setAutoRefresh((enabled) => !enabled)}>
                      <Smooth radius="capsule"><span className="debug-preview-setting-icon"><ArrowClockwiseIcon size={16} aria-hidden="true" /></span></Smooth>
                      <span className="debug-preview-setting-label">
                        <span>保存后自动刷新</span>
                        <span>{autoRefresh ? '已开启' : '已关闭'}</span>
                      </span>
                    </button></Smooth>
                  </div>
                  <div className="debug-preview-section">
                    {agentOptions.map(({ kind, label, icon: Icon, copy }) => {
                      const enabled = agentSettings?.[kind] ?? false;
                      const connection = kind === 'cli' ? agentSettings?.cliSession : agentSettings?.mcpConfig;
                      return <div className="debug-agent-setting" key={kind}>
                        <Smooth radius={12}><button type="button" className="debug-preview-setting"
                          role="switch" aria-label={label} aria-checked={enabled}
                          disabled={!agentSettings || agentUpdating} onClick={() => void toggleAgent(kind)}>
                          <Smooth radius="capsule"><span className="debug-preview-setting-icon"><Icon size={16} aria-hidden="true" /></span></Smooth>
                          <span className="debug-preview-setting-label">
                            <span>{label}</span>
                            <span>{agentSettings ? enabled ? '已开启' : '已关闭' : '读取设置中'}</span>
                          </span>
                        </button></Smooth>
                        {enabled && connection && <Button variant="ghost" className="debug-agent-copy" onPress={async () => {
                          const value = typeof connection === 'string' ? connection : JSON.stringify(connection, null, 2);
                          try {
                            await navigator.clipboard.writeText(value);
                            setNotice('已' + copy);
                          } catch {
                            setNotice('无法' + copy);
                          }
                        }}>{copy}</Button>}
                      </div>;
                    })}
                  </div>
                  <footer className="debug-preview-footer">
                    <PreviewInfo project={project} scene={scene?.title} device={device} height={snapshot.height} zoom={viewport.percent}
                      status={status} agentSettings={agentSettings} />
                    <span className="debug-preview-notice" role="status">{notice}</span>
                  </footer>
                </Popover.Dialog></Smooth>
              </Popover.Content>
            </Popover>
          </Hint>
        </div>
      </header>

      <div className="debug-stage" ref={stage} data-ready={hasPreview && !!project}
        data-pan={dragging ? 'dragging' : space ? 'ready' : undefined}>
        {projectError && <p role="alert" className="debug-empty">无法读取项目配置。</p>}
        {hasPreview && project && devices.map((profile) => {
          const url = urlFor(profile.id);
          const active = profile.id === device.id;
          const cached = deviceSnapshots[profile.id];
          const height = cached?.url === url ? cached.snapshot.height : previewHeight(profile, url);
          const offset = deviceOffset(profile.id);
          const selectedArea = layoutMode && layoutArea && layoutAreaForDevice(
            layoutArea, offset / scale - profile.width / 2, -height / 2, profile.width, height,
          );
          const shaped = profile.shape !== 'rectangle';
          const radius = shaped ? 0 : 40;
          return <div key={profile.id} className="debug-device-canvas" data-ruler={active && ruler} style={{
            width: profile.width * scale, height: height * scale,
            transform: `translate(-50%, -50%) translate(${viewport.x + offset}px, ${viewport.y}px)`,
          }}>
          {comparison && <button type="button" className="debug-device-label" aria-pressed={active}
            onClick={() => changeDevice(profile.id)}>{profile.name}</button>}
          <div className="debug-device-scale" style={{ width: profile.width, height, transform: `scale(${scale})` }}>
          <Smooth radius={radius} stroke={shaped ? undefined : { width: 1, color: "#ffffff", opacity: 0.5 }}>
          <div className="debug-device-frame" style={{ borderRadius: radius, background: shaped ? 'transparent' : undefined }}>
            <iframe key={url} width={profile.width} height={profile.height}
              title={comparison ? profile.name + '预览' : (scene?.title ?? project.title) + '预览'} src={url}
              ref={(frame) => {
                if (frame) frames.current.set(profile.id, frame);
                else frames.current.delete(profile.id);
              }}
              onLoad={(event) => {
                event.currentTarget.contentWindow?.addEventListener('pointerdown', () => activateDevice.current(profile.id), true);
                setFrameReady((value) => value + 1);
              }} />
            {selectedArea && <LayoutLens key={url} profile={profile} height={height} scale={scale} area={selectedArea}
              frame={frames.current.get(profile.id)}
              activeSnapshot={active ? snapshot : undefined} />}
            {active && !layoutMode && highlighted && !highlighted.hidden && <BoxHighlight node={highlighted} width={width} height={snapshot.height} />}
            {active && !layoutMode && picking && <div className="debug-pick-surface" onPointerMove={(event) => hover(findAt(event)?.id)}
              onPointerLeave={() => {
                hover(undefined);
                setMeasure(undefined);
              }} onPointerDown={(event) => {
                if (event.button !== 0) return;
                event.preventDefault();
                const node = findAt(event);
                if (node) {
                  select(node.id);
                  setCanvasSelection(node.id);
                  openView('元素');
                }
                hover(undefined);
                setPicking(false);
              }} />}
            {profile.shape === 'capsule' && <div className="debug-device-capsule-outline"
              style={{ height: profile.height }} aria-hidden="true" />}
          </div></Smooth>
          </div>
          {active && ruler && <div className="debug-rulers">
            <span className="debug-ruler-corner">px</span>
            <Ruler axis="y" size={snapshot.height} cursor={measure?.y} />
            <Ruler axis="x" size={width} cursor={measure?.x} />
          </div>}
          {active && areaMode && <div className="debug-area-surface" onPointerDown={startArea} onPointerMove={moveArea}
            onPointerUp={endArea} onPointerCancel={() => { areaStart.current = undefined; setArea(undefined); }}>
            {area && <div className="debug-area-box" style={{
              left: area.x / width * 100 + '%', top: area.y / snapshot.height * 100 + '%',
              width: area.width / width * 100 + '%', height: area.height / snapshot.height * 100 + '%',
            }}><Smooth radius={0} stroke={{ width: 1, color: '#69b6ff', opacity: 1 }}>
              <div className="debug-area-outline" aria-hidden="true" />
            </Smooth></div>}
          </div>}
        </div>;
        })}
        {layoutMode && <LayoutSelection area={layoutArea} onArea={setLayoutArea}
          scale={scale} panX={viewport.x} panY={viewport.y} onPoint={ruler ? trackLayoutPoint : undefined} />}
        {inspected && <ElementCard node={inspected} detailed={picking} canvasWidth={width} canvasHeight={snapshot.height}
          scale={scale} panX={viewport.x + deviceOffset(device.id)} panY={viewport.y} />}
      </div>
      {areaMode && <div className="debug-area-hint">
        <Smooth radius="capsule" stroke={{ ...popoverStroke, opacity: 0.14 }}>
          <div className="debug-capture-hint-body" role="status">
            {areaMode === 'shot' ? '拖拽选择截图区域' : '拖拽选择录屏区域'} · Esc 取消
          </div>
        </Smooth>
      </div>}
      {captureNotice && <div className="debug-capture-notice">
        <Smooth radius={10} stroke={{ ...popoverStroke, opacity: 0.14 }}>
          <div className="debug-capture-hint-body" role="status">{captureNotice}</div>
        </Smooth>
      </div>}
      {layoutMode && <Smooth radius={14}><div className="debug-update debug-layout-hint" role="status"
        data-build={status} data-compact={compactStatus} data-capture-notice={!!captureNotice}>
        <RectangleDashedIcon size={18} aria-hidden="true" />
        <span>拖动框选，检查尺寸和圆角</span>
        <kbd className="debug-update-key">{apple ? 'Option' : 'Alt'}</kbd>
      </div></Smooth>}
      {(status === 'building' || status === 'failed') && <Smooth radius={status === 'building' ? 'capsule' : 14}><div className="debug-update" data-status={status} data-compact={compactStatus}
        ref={statusRef} role={status === 'failed' ? 'alert' : 'status'} aria-label={status === 'building' ? '正在更新预览' : undefined}>
        <div className="debug-update-pane" data-pane="loading" aria-hidden={status !== 'building'}>
          <Smooth radius="capsule"><span className="debug-update-track">
            <Smooth radius="capsule"><span className="debug-update-bar" /></Smooth>
            <Smooth radius="capsule"><span className="debug-update-bar" /></Smooth>
          </span></Smooth>
        </div>
        <div className="debug-update-pane" data-pane="failed" aria-hidden={status !== 'failed'} inert={status !== 'failed'}>
          <div className="debug-update-message">
            <ProhibitInsetIcon size={18} className="debug-update-error-icon" aria-hidden="true" />
            <span>{hasPreview ? '由于构建失败，当前预览未刷新' : '构建失败，暂时没有可预览的版本'}</span>
          </div>
          <div className="debug-update-actions">
            <span className="debug-update-errors">
              {errorCount > 0 ? <><span className="debug-update-error-dot" aria-hidden="true">
                <CircleIcon size={7} weight="fill" />
              </span>{errorCount}个错误</> : '构建失败'}
            </span>
            <Smooth radius={8}><button type="button" className="debug-update-action" onClick={() => openView('控制台')}>
              <BuildActionLabel value={inspectorOpen && view === '控制台' ? '已打开控制台' : '查看详情'}
                labels={['查看详情', '已打开控制台']} />
              <Smooth radius={4}><span className="debug-update-key">enter</span></Smooth>
            </button></Smooth>
            <Smooth radius={8}><button type="button" className="debug-update-action debug-update-copy" onClick={() => void copyPrompt()}>
              <BuildActionLabel value={copyPromptLabels[copyState]} labels={Object.values(copyPromptLabels)} />
              <span className="debug-update-keys">
                {apple
                  ? <Smooth radius={4}><span className="debug-update-key debug-update-key-icon"><CommandIcon size={14} aria-hidden="true" /></span></Smooth>
                  : <Smooth radius={4}><span className="debug-update-key">Ctrl</span></Smooth>}
                <Smooth radius={4}><span className="debug-update-key" data-dim="true">C</span></Smooth>
              </span>
            </button></Smooth>
          </div>
        </div>
      </div></Smooth>}
      <span className="debug-sr-only" role="status">{copyState === 'copied' ? '提示词已复制' : copyState === 'failed' ? '无法复制提示词' : copyState === 'copying' ? '正在复制提示词' : ''}</span>
    </section></Smooth>

    <Smooth radius={8}><aside className="debug-inspector" aria-label="检查器" data-console={view === '控制台'} data-placement={placement}
      aria-hidden={!inspectorOpen} inert={!inspectorOpen}>
      <div className="debug-inspector-toolbar">
        <div className="debug-toolbar-slot">
          <Hint label={`按住 ${apple ? 'Option' : 'Alt'} 拖动框选布局`}>
            <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon debug-picker" aria-label="从画布选取元素" aria-pressed={picking}
              onPress={() => {
                setPicking(!picking);
                hover(undefined);
                setView('元素');
              }}><img className="debug-picker-asset" src={picker} alt="" /></Button></Smooth>
          </Hint>
        </div>
        <DevToolsTabs label="调试面板" tabs={views} value={view} onChange={openView}
          extra={(name) => name === '控制台' && errorCount > 0 &&
            <span className="debug-errors"><ProhibitInsetIcon size={14} aria-hidden="true" />{errorCount}</span>} />
        <div className="debug-toolbar-slot" data-end="true">
          <Popover isOpen={dockOpen} onOpenChange={setDockOpen}>
            <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon debug-picker" aria-label="检查器位置"><PlacementIcon placement={placement} /></Button></Smooth>
            <Popover.Content placement="bottom end" className="debug-popover-wrap"><Smooth radius={20} stroke={popoverStroke}><Popover.Dialog aria-label="检查器位置" className="debug-popover">
              <span className="debug-section-title">检查器位置</span>
              {narrowInspector && <p className="debug-placement-notice">窄屏下仅支持下侧</p>}
              {placements.map((option) => <Button key={option} variant="ghost" className="debug-menu-item debug-placement"
                isDisabled={narrowInspector && option !== 'bottom'}
                aria-pressed={placement === option} onPress={() => { setPlacement(option); setDockOpen(false); }}>
                <PlacementIcon placement={option} />{placementLabels[option]}
              </Button>)}
            </Popover.Dialog></Smooth></Popover.Content>
          </Popover>
        </div>
      </div>
      {view === '元素' && <Inspector key={previewUrl} nodes={snapshot.nodes} selected={selected} canvasSelection={canvasSelection}
        onCanvasSelectionRevealed={() => setCanvasSelection(undefined)} select={select} hover={hover} />}
      <Console logs={logs} clearLogs={() => setLogs((previous) => previous.filter((entry) => entry.device && entry.device !== device.id))}
        hidden={view !== '控制台'} />
      {project && <Sources title={project.title} scene={scene?.id ?? ''} revision={sourceRevision + generation}
        hidden={view !== '源代码' || !inspectorOpen} />}
      {view === '资源' && <Smooth radius={7}><div className="debug-panel"><h2>当前画面</h2><dl className="debug-properties">
        <div><dt>图片对象</dt><dd>{snapshot.nodes.filter((node) => node.kind === 2 && !node.hidden).length}</dd></div>
      </dl></div></Smooth>}
      {view === '性能' && <Smooth radius={7}><div className="debug-panel"><h2>浏览器预览</h2><dl className="debug-properties">
        <div><dt>对象数</dt><dd>{snapshot.nodes.length}</dd></div>
        <div><dt>最近绘制间隔</dt><dd>{snapshot.frameTime ? snapshot.frameTime.toFixed(1) + ' ms' : '—'}</dd></div>
        <div><dt>WASM 内存</dt><dd>{snapshot.memoryBytes ? (snapshot.memoryBytes / 1048576).toFixed(1) + ' MiB' : '—'}</dd></div>
      </dl></div></Smooth>}
      {view === '图层' && <Smooth radius={7}><div className="debug-panel">
        {snapshot.nodes.filter((node) => !node.hidden).reverse().map((node) =>
          <Button key={node.id} variant="ghost" className="debug-layer" onHoverStart={() => hover(node.id)} onHoverEnd={() => hover(undefined)}
            onPress={() => { select(node.id); setView('元素'); }}>{nodeName(node)} <span>{node.width} × {node.height}</span></Button>)}
      </div></Smooth>}
    </aside></Smooth>
    {inspectorOpen && placement !== 'bottom' && <div className="debug-resizer" data-placement={placement} role="separator"
      aria-orientation="vertical" aria-label="调整检查器宽度" onPointerDown={resize} />}
  </main>;
}
