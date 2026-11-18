import { BroomIcon } from '@phosphor-icons/react/dist/csr/Broom';
import { BugBeetleIcon } from '@phosphor-icons/react/dist/csr/BugBeetle';
import { CaretDownIcon } from '@phosphor-icons/react/dist/csr/CaretDown';
import { ChatCircleTextIcon } from '@phosphor-icons/react/dist/csr/ChatCircleText';
import { EyeIcon } from '@phosphor-icons/react/dist/csr/Eye';
import { FunnelIcon } from '@phosphor-icons/react/dist/csr/Funnel';
import { GearFineIcon } from '@phosphor-icons/react/dist/csr/GearFine';
import { InfoIcon } from '@phosphor-icons/react/dist/csr/Info';
import { TerminalWindowIcon } from '@phosphor-icons/react/dist/csr/TerminalWindow';
import { UserCircleIcon } from '@phosphor-icons/react/dist/csr/UserCircle';
import { WarningIcon } from '@phosphor-icons/react/dist/csr/Warning';
import { XCircleIcon } from '@phosphor-icons/react/dist/csr/XCircle';
import { Button, Input, Popover, Switch } from '@heroui/react';
import { useLayoutEffect, useRef, useState } from 'react';
import { Smooth, popoverStroke } from './smooth';

export type LogLevel = 'log' | 'info' | 'error' | 'warning' | 'debug';
export type Log = { level: LogLevel; source: '构建' | '运行'; text: string; device?: string };

const levels = {
  error: { name: '错误', label: 'ERROR', icon: XCircleIcon },
  warning: { name: '警告', label: 'WARN', icon: WarningIcon },
  info: { name: '信息', label: 'INFO', icon: InfoIcon },
  debug: { name: '详细信息', label: 'DEBUG', icon: BugBeetleIcon },
} as const;

const categories = [
  { id: 'all', name: '全部消息', icon: ChatCircleTextIcon },
  { id: '运行', name: '运行日志', icon: UserCircleIcon },
  { id: '构建', name: '构建日志', icon: TerminalWindowIcon },
  ...Object.entries(levels).map(([id, { name, icon }]) => ({ id, name, icon })),
];

export function logLevel(level: unknown, text: string): LogLevel {
  if (level === 'error' || level === 'warning' || level === 'debug') return level;
  const prefix = text.match(/^\s*(?:\[(ERROR|WARN(?:ING)?|INFO|DEBUG|TRACE)\]|(error|warn(?:ing)?|info|debug|trace)(?=\[|:))/im);
  switch ((prefix?.[1] ?? prefix?.[2])?.toLowerCase()) {
    case 'error': return 'error';
    case 'warn':
    case 'warning': return 'warning';
    case 'info': return 'info';
    case 'trace':
    case 'debug': return 'debug';
    default: return level === 'info' ? 'info' : 'log';
  }
}

export function Console({ logs, clearLogs, hidden }: { logs: Log[]; clearLogs: () => void; hidden: boolean }) {
  const [category, setCategory] = useState('all');
  const [query, setQuery] = useState('');
  const [showCategories, setShowCategories] = useState(true);
  const [follow, setFollow] = useState(true);
  const output = useRef<HTMLDivElement>(null);
  const atEnd = useRef(true);
  const term = query.trim().toLocaleLowerCase();
  const matches = (entry: Log, id: string) => id === 'all' || entry.level === id || entry.source === id;
  const visible = logs.filter((entry) => matches(entry, category)
    && [entry.text, entry.source, entry.level === 'log' ? 'LOG' : levels[entry.level].label].some((value) => value.toLocaleLowerCase().includes(term)));

  useLayoutEffect(() => {
    const panel = output.current;
    if (!hidden && follow && atEnd.current && panel) panel.scrollTop = panel.scrollHeight;
  }, [logs, query, category, follow, hidden]);

  return <section className="debug-console" aria-label="控制台" hidden={hidden}>
    <Smooth radius={8}><div className="debug-panel-toolbar">
      <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon" aria-label="清空日志"
        isDisabled={!logs.length} onPress={() => { atEnd.current = true; clearLogs(); }}>
        <BroomIcon size={16} aria-hidden="true" />
      </Button></Smooth>
      <span className="debug-panel-divider" aria-hidden="true" />
      <div className="debug-console-search">
        <FunnelIcon size={16} aria-hidden="true" />
        <Input aria-label="过滤日志" placeholder="过滤" type="search" value={query} onChange={(event) => { atEnd.current = true; setQuery(event.target.value); }} />
      </div>
      <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon" aria-label="显示日志类别"
        aria-pressed={showCategories} onPress={() => {
          setShowCategories(!showCategories);
          if (showCategories) setCategory('all');
        }}>
        <EyeIcon size={16} aria-hidden="true" />
      </Button></Smooth>
      <Popover>
        <Smooth radius={6}><Button isIconOnly variant="ghost" className="debug-icon" aria-label="控制台设置">
          <GearFineIcon size={16} aria-hidden="true" />
        </Button></Smooth>
        <Popover.Content placement="bottom end" className="debug-popover-wrap">
          <Smooth radius={20} stroke={popoverStroke}><Popover.Dialog className="debug-popover" aria-label="控制台设置">
            <Switch isSelected={follow} onChange={(value) => { atEnd.current = true; setFollow(value); }}>
              <Smooth radius="capsule"><Switch.Control><Smooth radius="capsule"><Switch.Thumb /></Smooth></Switch.Control></Smooth>
              <Switch.Content>自动滚动到最新日志</Switch.Content>
            </Switch>
          </Popover.Dialog></Smooth>
        </Popover.Content>
      </Popover>
    </div></Smooth>
    <div className="debug-console-body" data-categories={showCategories}>
      {showCategories && <Smooth radius={8}><nav className="debug-panel-surface debug-console-categories" aria-label="日志类别">
        {categories.map(({ id, name, icon: Icon }) => <Button key={id} variant="ghost" className="debug-sidebar-row debug-console-category"
          aria-pressed={category === id} data-active={category === id} onPress={() => { atEnd.current = true; setCategory(id); }}>
          <CaretDownIcon className="debug-console-caret" size={10} weight="bold" aria-hidden="true" />
          <Icon className="debug-console-icon" data-level={id} size={15} aria-hidden="true" />
          <span>{name}</span>
          <span className="debug-console-count">{logs.filter((entry) => matches(entry, id)).length}</span>
        </Button>)}
      </nav></Smooth>}
      <Smooth radius={8}><div className="debug-panel-surface debug-console-output" ref={output} role="region" aria-label="日志输出" tabIndex={0}
        onScroll={(event) => {
          const panel = event.currentTarget;
          atEnd.current = panel.scrollHeight - panel.clientHeight - panel.scrollTop < 4;
        }}>
        {visible.length ? <ol>
          {visible.map((entry, index) => {
            const level = entry.level === 'log' ? undefined : levels[entry.level];
            const Icon = level?.icon;
            return <li className="debug-console-log" key={index} data-level={entry.level}>
              {Icon && level ? <Icon className="debug-console-icon debug-console-level" size={13} role="img" aria-label={level.name} />
                : <span aria-hidden="true" />}
              <pre>{entry.text.trimEnd()}</pre>
              <span className="debug-console-source">{entry.source}</span>
            </li>;
          })}
        </ol> : <p className="debug-empty">{logs.length ? '未找到匹配的日志' : '暂无日志'}</p>}
      </div></Smooth>
    </div>
  </section>;
}
