import { Button, Input, Popover, Switch } from '@heroui/react';
import { useLayoutEffect, useRef, useState } from 'react';
import broom from './assets/console/Broom.svg';
import funnel from './assets/console/Funnel.svg';
import gear from './assets/console/GearFine.svg';
import divider from './assets/console/Divider.svg';
import message from './assets/console/ChatCircleText.svg';
import user from './assets/console/UserCircle.svg';
import terminal from './assets/console/TerminalWindow.svg';
import error from './assets/console/ErrorFilter.svg';
import warning from './assets/console/WarningFilter.svg';
import info from './assets/console/Info.svg';
import errorMessage from './assets/console/ErrorMessage.svg';
import warningMessage from './assets/console/WarningMessage.svg';
import infoMessage from './assets/console/InfoMessage.svg';
import caret from './assets/CaretDown2.svg';
import eye from './assets/Eye.svg';
import bug from './assets/BugBeetle.svg';

export type LogLevel = 'log' | 'info' | 'error' | 'warning' | 'debug';
export type Log = { level: LogLevel; source: '构建' | '运行'; text: string };

const levels = {
  error: { name: '错误', label: 'ERROR', icon: error, messageIcon: errorMessage },
  warning: { name: '警告', label: 'WARN', icon: warning, messageIcon: warningMessage },
  info: { name: '信息', label: 'INFO', icon: info, messageIcon: infoMessage },
  debug: { name: '详细信息', label: 'DEBUG', icon: bug, messageIcon: bug },
} as const;

const categories = [
  { id: 'all', name: '全部消息', icon: message },
  { id: '运行', name: '运行日志', icon: user },
  { id: '构建', name: '构建日志', icon: terminal },
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
    <div className="debug-console-toolbar">
      <Button isIconOnly variant="ghost" className="debug-icon" aria-label="清空日志"
        isDisabled={!logs.length} onPress={() => { atEnd.current = true; clearLogs(); }}>
        <img src={broom} alt="" />
      </Button>
      <img src={divider} alt="" />
      <div className="debug-console-search">
        <img src={funnel} alt="" />
        <Input aria-label="过滤日志" placeholder="过滤" type="search" value={query} onChange={(event) => { atEnd.current = true; setQuery(event.target.value); }} />
      </div>
      <Button isIconOnly variant="ghost" className="debug-icon" aria-label="显示日志类别"
        aria-pressed={showCategories} onPress={() => {
          setShowCategories(!showCategories);
          if (showCategories) setCategory('all');
        }}>
        <img src={eye} alt="" />
      </Button>
      <Popover>
        <Button isIconOnly variant="ghost" className="debug-icon" aria-label="控制台设置">
          <img src={gear} alt="" />
        </Button>
        <Popover.Content placement="bottom end">
          <Popover.Dialog className="debug-popover" aria-label="控制台设置">
            <Switch isSelected={follow} onChange={(value) => { atEnd.current = true; setFollow(value); }}>
              <Switch.Control><Switch.Thumb /></Switch.Control>
              <Switch.Content>自动滚动到最新日志</Switch.Content>
            </Switch>
          </Popover.Dialog>
        </Popover.Content>
      </Popover>
    </div>
    <div className="debug-console-body" data-categories={showCategories}>
      {showCategories && <nav className="debug-console-categories" aria-label="日志类别">
        {categories.map(({ id, name, icon }) => <Button key={id} variant="ghost" className="debug-console-category"
          aria-pressed={category === id} onPress={() => { atEnd.current = true; setCategory(id); }}>
          <img className="debug-console-caret" src={caret} alt="" />
          <img className={id === 'error' || id === 'warning' || id === 'info' ? undefined : 'debug-console-icon'} src={icon} alt="" />
          <span>{name}</span>
          <span className="debug-console-count">{logs.filter((entry) => matches(entry, id)).length}</span>
        </Button>)}
      </nav>}
      <div className="debug-console-output" ref={output} role="region" aria-label="日志输出" tabIndex={0}
        onScroll={(event) => {
          const panel = event.currentTarget;
          atEnd.current = panel.scrollHeight - panel.clientHeight - panel.scrollTop < 4;
        }}>
        {visible.length ? <ol>
          {visible.map((entry, index) => <li className="debug-console-log" key={index} data-level={entry.level}>
            {entry.level === 'log' ? <span aria-hidden="true" />
              : <img className="debug-console-level" src={levels[entry.level].messageIcon} alt={levels[entry.level].name} />}
            <pre>{entry.text.trimEnd()}</pre>
            <span className="debug-console-source">{entry.source}</span>
          </li>)}
        </ol> : <p className="debug-empty">{logs.length ? '未找到匹配的日志' : '暂无日志'}</p>}
      </div>
    </div>
  </section>;
}
