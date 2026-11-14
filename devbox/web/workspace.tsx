import { Button, Popover, Switch } from '@heroui/react';
import { useEffect, useRef, useState, type PointerEvent } from 'react';
import { Inspector } from './inspector';
import { nodeName, pickNode, width, type PreviewWindow, type Snapshot } from './preview';
import logo from './assets/VerisDevBox.svg';
import caret from './assets/CaretDown.svg';
import ruler from './assets/Ruler.svg';
import zoomIcon from './assets/MagnifyingGlassPlus.svg';
import code from './assets/CodeBlock.svg';
import camera from './assets/CameraPlus.svg';
import controls from './assets/Faders.svg';
import picker from './assets/PaperPlaneTilt.svg';
import warningIcon from './assets/WarningDiamond.svg';
import errorIcon from './assets/ProhibitInset.svg';
import divider from './assets/Line12.svg';
import './workspace.css';

const views = ['元素', '控制台', '源代码', '资源', '性能', '图层'] as const;
type View = typeof views[number];
type Status = 'ready' | 'building' | 'failed';
type Log = { level: 'info' | 'error' | 'warning'; source: '构建' | '运行'; text: string };
type Project = {
  title: string;
  scenes: { id: string; title: string }[];
  status: Status;
  hasPreview: boolean;
  logs: Omit<Log, 'source'>[];
};
type Source = { name: string; text: string };
const EMPTY_SNAPSHOT: Snapshot = { height: 480, nodes: [], frameTime: 0, memoryBytes: 0 };

function Icon({ src }: { src: string }) {
  return <img className="debug-asset" src={src} alt="" />;
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
  const [bounds, setBounds] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [autoRefresh, setAutoRefresh] = useState(() => localStorage.getItem('aeris-debug-auto-refresh') !== 'off');
  const [selectedId, select] = useState<number>();
  const [hoverId, hover] = useState<number>();
  const [sourceFilter, setSourceFilter] = useState<'全部' | Log['source']>('全部');
  const preview = useRef<HTMLIFrameElement>(null);
  const refreshEnabled = useRef(autoRefresh);

  const previewParams = new URLSearchParams({ scene: scene?.id ?? '', debug: '1', revision: String(generation) });
  const previewUrl = '/preview.html?' + previewParams;
  const renderer = () => (preview.current?.contentWindow as PreviewWindow | null)?.aerisDebugger;
  const selected = snapshot.nodes.find((node) => node.id === selectedId) ?? snapshot.nodes[0];
  const highlighted = snapshot.nodes.find((node) => node.id === hoverId) ?? (bounds || picking ? selected : undefined);
  const errorCount = logs.filter((entry) => entry.level === 'error').length;
  const warningCount = logs.filter((entry) => entry.level === 'warning').length;

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
        append({ level: data.level === 'error' ? 'error' : 'info', source: '运行', text: data.text });
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
          setLogs(data.logs.map((entry) => ({ ...entry, source: '构建' })));
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
      const severity = /\berror[\[:]/i.test(text) ? 'error' : /\bwarning:/i.test(text) ? 'warning' : level;
      append({ level: severity, source: '构建', text });
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
    return pickNode(snapshot.nodes, (event.clientX - rect.left) * width / rect.width,
      (event.clientY - rect.top) * snapshot.height / rect.height);
  }

  function saveScreenshot() {
    const url = renderer()?.screenshot();
    if (!url) return;
    const link = document.createElement('a');
    link.href = url;
    link.download = (scene?.id ?? project?.title ?? 'aeris') + '.png';
    link.click();
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
  }

  const buildLabel = status === 'building' ? '构建中' : status === 'failed'
    ? hasPreview ? '正在预览上一成功版本' : '构建失败'
    : '';

  return <main className="debug-workspace" data-inspector={inspectorOpen}>
    <section className="debug-preview-panel" aria-label="手表预览">
      <header className="debug-toolbar">
        <div className="debug-logo"><img src={logo} alt="Aeris DevBox" /></div>
        {project && project.scenes.length === 0 ? <span className="debug-app-title">{project.title}</span> :
        <Popover isOpen={scenesOpen} onOpenChange={setScenesOpen}>
          <Button variant="ghost" className="debug-target" aria-label="选择场景">
            <span>{project?.title ?? "Aeris DevBox"}</span>{scene && <span className="debug-target-device"> · {scene.title}</span>}<Icon src={caret} />
          </Button>
          <Popover.Content placement="bottom"><Popover.Dialog aria-label="预览场景" className="debug-popover">
            <span className="debug-section-title">预览场景</span>
            {project?.scenes.map((item) => <Button key={item.id} variant="ghost" className="debug-menu-item"
              aria-pressed={scene?.id === item.id} onPress={() => { changeScene(item.id); setScenesOpen(false); }}>
              {item.title}
            </Button>)}
          </Popover.Dialog></Popover.Content>
        </Popover>}
        <div className="debug-tools">
          <Button isIconOnly variant="ghost" className="debug-icon" aria-label="显示布局边界" aria-pressed={bounds} onPress={() => setBounds(!bounds)}><Icon src={ruler} /></Button>
          <Popover>
            <Button isIconOnly variant="ghost" className="debug-icon" aria-label="缩放"><Icon src={zoomIcon} /></Button>
            <Popover.Content><Popover.Dialog aria-label="缩放" className="debug-popover">
              {[0.75, 1, 1.5, 2].map((scale) => <Button key={scale} variant="ghost" className="debug-menu-item" aria-pressed={zoom === scale} onPress={() => setZoom(scale)}>{scale * 100}%</Button>)}
            </Popover.Dialog></Popover.Content>
          </Popover>
          <Button isIconOnly variant="ghost" className="debug-icon" aria-label="显示检查面板" aria-pressed={inspectorOpen} onPress={() => { setInspectorOpen(!inspectorOpen); setPicking(false); hover(undefined); }}><Icon src={code} /></Button>
          <Button isIconOnly variant="ghost" className="debug-icon" aria-label="保存截图" isDisabled={!snapshot.nodes.length} onPress={saveScreenshot}><Icon src={camera} /></Button>
          <Popover>
            <Button isIconOnly variant="ghost" className="debug-icon" aria-label="预览控制"><Icon src={controls} /></Button>
            <Popover.Content placement="bottom end"><Popover.Dialog className="debug-popover" aria-label="预览控制">
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
        </div>
      </header>

      <div className="debug-stage">
        {projectError && <p role="alert" className="debug-empty">无法读取项目配置。</p>}
        {hasPreview && project && <div className="debug-device-frame" style={{ width: width * zoom, aspectRatio: width + ' / ' + snapshot.height }}>
          <iframe key={previewUrl} ref={preview} width={width} height={480} title={(scene?.title ?? project.title) + '预览'} src={previewUrl} />
          {highlighted && !highlighted.hidden && <div className="debug-highlight" style={{
            left: highlighted.x / width * 100 + '%', top: highlighted.y / snapshot.height * 100 + '%',
            width: highlighted.width / width * 100 + '%', height: highlighted.height / snapshot.height * 100 + '%',
          }} />}
          {picking && <div className="debug-pick-surface" onPointerMove={(event) => hover(findAt(event)?.id)}
            onPointerLeave={() => hover(undefined)} onPointerDown={(event) => {
              const node = findAt(event);
              if (node) select(node.id);
              hover(undefined);
              setPicking(false);
            }} />}
        </div>}
      </div>
      {buildLabel && <Button variant="secondary" size="sm" className="debug-build-status" onPress={() => openView('控制台')}>{buildLabel}</Button>}
    </section>

    {inspectorOpen && <aside className="debug-inspector" aria-label="检查器">
      <div className="debug-inspector-toolbar">
        <Button isIconOnly variant="ghost" className="debug-icon debug-picker" aria-label="从画布选取元素" aria-pressed={picking}
          onPress={() => {
            setPicking(!picking);
            hover(undefined);
            setView('元素');
          }}><Icon src={picker} /></Button>
        <nav className="debug-main-tabs" aria-label="调试面板">
          {views.map((name) => <span className="debug-main-tab" key={name}>
            <Button variant="ghost" className="debug-pill" aria-pressed={view === name} onPress={() => setView(name)}>{name}
              {name === '控制台' && warningCount > 0 && <span className="debug-warnings"><Icon src={warningIcon} />{warningCount}</span>}
              {name === '控制台' && errorCount > 0 && <span className="debug-errors"><Icon src={errorIcon} />{errorCount}</span>}
            </Button>
            {name !== '元素' && name !== '图层' && <Icon src={divider} />}
          </span>)}
        </nav>
      </div>
      {view === '元素' && <Inspector key={previewUrl} nodes={snapshot.nodes} selected={selected} select={select} hover={hover} />}
      {view === '控制台' && <div className="debug-panel">
        <div className="debug-panel-actions">
          {(['全部', '构建', '运行'] as const).map((name) => <Button size="sm" variant="ghost" key={name} aria-pressed={sourceFilter === name} onPress={() => setSourceFilter(name)}>{name}</Button>)}
          <Button size="sm" variant="ghost" onPress={() => setLogs([])}>清空</Button>
        </div>
        {logs.filter((entry) => sourceFilter === '全部' || entry.source === sourceFilter).map((entry, index) =>
          <pre className="debug-log" key={index} data-level={entry.level}>[{entry.source}] {entry.text}</pre>)}
        {!logs.length && <p className="debug-empty">暂无日志</p>}
      </div>}
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
    </aside>}
  </main>;
}
