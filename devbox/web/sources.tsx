import { Button } from '@heroui/react';
import { CaretDownIcon } from '@phosphor-icons/react/dist/csr/CaretDown';
import { FileCodeIcon } from '@phosphor-icons/react/dist/csr/FileCode';
import { FolderIcon } from '@phosphor-icons/react/dist/csr/Folder';
import { FolderOpenIcon } from '@phosphor-icons/react/dist/csr/FolderOpen';
import { SidebarSimpleIcon } from '@phosphor-icons/react/dist/csr/SidebarSimple';
import { XIcon } from '@phosphor-icons/react/dist/csr/X';
import { useEffect, useRef, useState } from 'react';
import { Smooth } from './smooth';
import './sources.css';

type Entry = { name: string; directory: boolean };
type Directory = { entries: Entry[]; initial?: string };
type Source = { name: string; text: string };
type FileTab = { path: string; scrollLeft: number; scrollTop: number };
type Listing = (Directory | { error: string }) & { scene: string };

async function request<T>(params: Record<string, string>, signal: AbortSignal): Promise<T> {
  const result = await fetch('/devbox-source?' + new URLSearchParams(params), { signal, cache: 'no-store' });
  if (result.status === 413) throw new Error('文件超过 1 MiB，无法预览。');
  if (result.status === 415) throw new Error('此文件不是文本。');
  if (!result.ok) throw new Error('无法读取此路径。');
  return result.json();
}

function useDirectory(directory: string, scene: string, revision: number, enabled = true) {
  const [listing, setListing] = useState<Listing>();
  useEffect(() => {
    if (!enabled) return;
    const controller = new AbortController();
    void request<Directory>({ directory, scene }, controller.signal)
      .then((data) => { if (!controller.signal.aborted) setListing({ ...data, scene }); })
      .catch((error: Error) => { if (!controller.signal.aborted) setListing({ error: error.message, scene }); });
    return () => controller.abort();
  }, [directory, scene, revision, enabled]);
  return listing;
}

type TreeProps = {
  scene: string;
  revision: number;
  active?: string;
  open: (path: string) => void;
};

function DirectoryRows({ listing, path, depth, ...tree }: TreeProps & { listing?: Listing; path: string; depth: number }) {
  if (!listing) return <p className="debug-source-message">正在读取文件…</p>;
  if ('error' in listing) return <p role="alert" className="debug-source-message">{listing.error}</p>;
  if (!listing.entries.length) return <p className="debug-source-message">没有可浏览的文件</p>;
  return <ul className="debug-source-list">
    {listing.entries.map((entry) => {
      const child = path ? `${path}/${entry.name}` : entry.name;
      return <li key={child}>{entry.directory
        ? <Folder {...tree} path={child} name={entry.name} depth={depth} />
        : <Button variant="ghost" className="debug-sidebar-row debug-source-row" data-path={child}
          data-active={tree.active === child} aria-current={tree.active === child ? 'true' : undefined}
          onPress={() => tree.open(child)} style={{ paddingInlineStart: depth * 12 + 20 }}>
          <FileCodeIcon size={15} aria-hidden="true" /><span title={child}>{entry.name}</span>
        </Button>}
      </li>;
    })}
  </ul>;
}

function Folder({ path, name, depth, ...tree }: TreeProps & { path: string; name: string; depth: number }) {
  const [expanded, setExpanded] = useState<boolean>();
  const open = expanded ?? Boolean(tree.active?.startsWith(path + '/'));
  const Icon = open ? FolderOpenIcon : FolderIcon;
  return <>
    <Button variant="ghost" className="debug-sidebar-row debug-source-row" data-path={path} aria-expanded={open}
      data-expanded={open} onPress={() => setExpanded(!open)} style={{ paddingInlineStart: depth * 12 + 6 }}>
      <CaretDownIcon className="debug-source-caret" size={10} weight="bold" aria-hidden="true" />
      <Icon size={15} aria-hidden="true" /><span title={path}>{name}</span>
    </Button>
    {open && <FolderContents {...tree} path={path} depth={depth + 1} />}
  </>;
}

function FolderContents({ path, depth, ...tree }: TreeProps & { path: string; depth: number }) {
  const listing = useDirectory(path, tree.scene, tree.revision);
  return <DirectoryRows {...tree} listing={listing} path={path} depth={depth} />;
}

function SourceCode({ source, tab }: { source: Source; tab: FileTab }) {
  const [markup, setMarkup] = useState<string>();
  const view = useRef<HTMLDivElement>(null);
  const lines = source.text.split('\n').length;
  useEffect(() => {
    let current = true;
    setMarkup(undefined);
    void import('./source-highlight')
      .then(({ highlight }) => { if (current) setMarkup(highlight(source.name, source.text)); })
      .catch(() => {});
    return () => { current = false; };
  }, [source]);
  useEffect(() => {
    if (view.current) view.current.scrollTo(tab.scrollLeft, tab.scrollTop);
  }, [tab]);
  return <div className="debug-source-scroll" tabIndex={0} aria-label={source.name} ref={view}
    onScroll={(event) => {
      tab.scrollLeft = event.currentTarget.scrollLeft;
      tab.scrollTop = event.currentTarget.scrollTop;
    }}>
    <div className="debug-source-code">
      <pre className="debug-source-lines" aria-hidden="true">{Array.from({ length: lines }, (_, index) => index + 1).join('\n')}</pre>
      <pre><code>{markup === undefined ? source.text : <span dangerouslySetInnerHTML={{ __html: markup }} />}{'\n'}</code></pre>
    </div>
  </div>;
}

export function Sources({ title, scene, revision, hidden }: { title: string; scene: string; revision: number; hidden: boolean }) {
  const root = useDirectory('', scene, revision, !hidden);
  const [tabs, setTabs] = useState<FileTab[]>([]);
  const [active, setActive] = useState<string>();
  const [treeVisible, setTreeVisible] = useState(true);
  const [document, setDocument] = useState<{ path: string; source?: Source; error?: string }>();
  const previousScene = useRef<string>(undefined);
  const activeTab = useRef<HTMLDivElement>(null);
  const tab = tabs.find(({ path }) => path === active);
  const initial = root && !('error' in root) ? root.initial : undefined;

  useEffect(() => {
    if (hidden || !root || 'error' in root || root.scene !== scene || previousScene.current === scene) return;
    previousScene.current = scene;
    if (initial) {
      setTabs((current) => current.some(({ path }) => path === initial) ? current : [...current, { path: initial, scrollLeft: 0, scrollTop: 0 }]);
      setActive(initial);
    }
  }, [hidden, root, initial, scene]);

  useEffect(() => {
    if (hidden || !active) return;
    const controller = new AbortController();
    void request<Source>({ file: active }, controller.signal)
      .then((source) => { if (!controller.signal.aborted) setDocument({ path: active, source }); })
      .catch((error: Error) => { if (!controller.signal.aborted) setDocument({ path: active, error: error.message }); });
    return () => controller.abort();
  }, [active, hidden, revision]);

  useEffect(() => {
    if (!hidden) activeTab.current?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }, [active, hidden]);

  function open(path: string) {
    setTabs((current) => current.some((item) => item.path === path) ? current : [...current, { path, scrollLeft: 0, scrollTop: 0 }]);
    setActive(path);
  }

  function close(path: string) {
    const index = tabs.findIndex((item) => item.path === path);
    setTabs((current) => current.filter((item) => item.path !== path));
    if (active === path) setActive((tabs[index + 1] ?? tabs[index - 1])?.path);
  }

  const shown = document?.path === active ? document : undefined;
  return <section className="debug-sources" aria-label="项目源代码" hidden={hidden} data-tree={treeVisible}>
    <Smooth radius={8}><div className="debug-panel-toolbar debug-source-toolbar">
      <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon" aria-label={treeVisible ? '隐藏文件树' : '显示文件树'}
        aria-pressed={treeVisible} onPress={() => setTreeVisible(!treeVisible)}><SidebarSimpleIcon size={16} aria-hidden="true" /></Button></Smooth>
      {tabs.length > 0 && <span className="debug-panel-divider" aria-hidden="true" />}
      <div className="debug-source-tabs" role="group" aria-label="已打开的文件">
        {tabs.map(({ path }) => {
          const name = path.split('/').at(-1);
          const selected = path === active;
          return <Smooth radius="capsule" key={path}><div className="debug-source-tab" title={path} data-active={selected} ref={selected ? activeTab : undefined}>
            <Smooth radius="capsule"><Button variant="ghost" className="debug-pill" data-path={path} aria-pressed={selected} onPress={() => setActive(path)}>
              <span>{name}</span>
            </Button></Smooth>
            <Smooth radius="capsule"><Button isIconOnly variant="ghost" className="debug-source-close" aria-label={`关闭 ${name}`} onPress={() => close(path)}>
              <XIcon size={12} weight="bold" aria-hidden="true" />
            </Button></Smooth>
          </div></Smooth>;
        })}
      </div>
    </div></Smooth>
    {treeVisible && !hidden && <Smooth radius={8}><nav className="debug-panel-surface debug-source-tree" aria-label="项目文件">
      <div className="debug-source-project" title={title}>{title}</div>
      <DirectoryRows listing={root} path="" depth={0} scene={scene} revision={revision} active={active} open={open} />
    </nav></Smooth>}
    <Smooth radius={8}><div className="debug-panel-surface debug-source-view">
      {tab && shown?.source ? <SourceCode key={tab.path} source={shown.source} tab={tab} /> :
        <p className="debug-source-message" role={shown?.error ? 'alert' : 'status'}>
          {shown?.error ?? (tab ? '正在读取源代码…' : '从文件树选择文件')}
        </p>}
      {tab && <div className="debug-source-footer" title={tab.path}>{tab.path}</div>}
    </div></Smooth>
  </section>;
}
