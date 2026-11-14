import { Button, Input } from '@heroui/react';
import { useState } from 'react';
import { nodeClass, nodeName, type PreviewNode } from './preview';
import arrow from './assets/CaretDown2.svg';
import search from './assets/MagnifyingGlass.svg';
import eye from './assets/Eye.svg';
import brackets from './assets/BracketsAngle.svg';
import bug from './assets/BugBeetle.svg';
import at from './assets/At.svg';

const tabs = ['样式', '已计算样式', '布局', '事件监听器', '属性', '无障碍'];

function color(value: number) {
  const hex = '#' + value.toString(16).padStart(6, '0');
  return <><span className="debug-swatch" style={{ backgroundColor: hex }} />{hex}</>;
}

function NodeDetails({ node, tab }: { node: PreviewNode; tab: string }) {
  const styles = [
    ['背景颜色', color(node.background)], ['文字颜色', color(node.foreground)],
    ['背景透明度', Math.round(node.backgroundOpacity / 2.55) + '%'],
    ['整体透明度', Math.round(node.opacity / 2.55) + '%'],
    ['圆角', node.radius + ' px'], ['边框宽度', node.border + ' px'],
  ] as const;
  const rows = tab === '样式' ? styles : tab === '已计算样式'
    ? [...styles, ['文字行高', node.fontHeight + ' px'], ['文字对齐', ['自动', '左对齐', '居中', '右对齐'][node.alignment] ?? '自动']]
    : tab === '布局' ? [
      ['位置', node.x + ', ' + node.y], ['尺寸', node.width + ' × ' + node.height + ' px'],
      ['内容区位置', node.contentX + ', ' + node.contentY],
      ['内容区尺寸', node.contentWidth + ' × ' + node.contentHeight + ' px'],
      ['溢出可见', node.overflow ? '是' : '否'],
    ] : tab === '事件监听器' ? [['已注册回调', node.events]]
    : tab === '无障碍' ? [['可点击', node.clickable ? '是' : '否'], ['文字', node.value || '—'], ['禁用', node.disabled ? '是' : '否']]
    : [['类型', nodeClass(node)], ...(node.component ? [['组件', node.component]] : []), ['文字', node.value || '—'], ['隐藏', node.hidden ? '是' : '否'],
      ['可滚动', node.scrollable ? '是' : '否'], ['选中', node.checked ? '是' : '否']];
  return <dl className="debug-properties">{rows.map(([key, value]) => <div key={key}><dt>{key}</dt><dd>{value}</dd></div>)}</dl>;
}

type TreeProps = {
  nodes: PreviewNode[];
  selected: PreviewNode | undefined;
  select: (id: number) => void;
  hover: (id?: number) => void;
};

export function Inspector({ nodes, selected, select, hover }: TreeProps) {
  const [query, setQuery] = useState('');
  const [tab, setTab] = useState('样式');
  const [collapsed, setCollapsed] = useState<Set<number>>(() => new Set());
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const parents = new Set(nodes.map((node) => node.parent));
  const selectedPath = new Set<number>();
  const matching = new Set<number>();
  const term = query.trim().toLocaleLowerCase();
  function collect(node: PreviewNode | undefined, path: Set<number>) {
    while (node && !path.has(node.id)) {
      path.add(node.id);
      node = byId.get(node.parent);
    }
  }
  collect(selected, selectedPath);
  if (term) {
    nodes.filter((node) => [nodeName(node), nodeClass(node), node.value].join(' ').toLocaleLowerCase().includes(term))
      .forEach((node) => collect(node, matching));
  }

  return <>
    <section className="debug-tree">
      <div className="debug-search"><img src={search} alt="" /><Input aria-label="搜索对象" placeholder="搜索" value={query} onChange={(event) => setQuery(event.target.value)} /></div>
      <div className="debug-tree-scroll" onMouseLeave={() => hover()} aria-label="LVGL 对象树">
        {nodes.map((node) => {
          if (term && !matching.has(node.id)) return null;
          let depth = 0;
          for (let parent = byId.get(node.parent); parent; parent = byId.get(parent.parent)) {
            if (!term && collapsed.has(parent.id) && !selectedPath.has(parent.id)) return null;
            depth++;
          }
          const open = term || !collapsed.has(node.id) || (selectedPath.has(node.id) && node.id !== selected?.id);
          return <div key={node.id} className="debug-tree-row" data-selected={selected?.id === node.id} data-hidden={node.hidden}
            style={{ paddingInlineStart: 12 + depth * 14 }} onMouseEnter={() => hover(node.id)}>
            {parents.has(node.id) ? <Button isIconOnly variant="ghost" className="debug-twisty"
              aria-label={open ? '折叠' : '展开'} aria-expanded={Boolean(open)}
              onPress={() => {
                select(node.id);
                setCollapsed((previous) => {
                  const next = new Set(previous);
                  if (open) next.add(node.id); else next.delete(node.id);
                  return next;
                });
              }}><img src={arrow} alt="" data-closed={!open} /></Button> : <span className="debug-twisty-spacer" />}
            <Button variant="ghost" className="debug-node-label" onPress={() => select(node.id)}>{nodeName(node)}</Button>
          </div>;
        })}
        {!nodes.length && <p className="debug-empty">等待预览加载</p>}
        {term && nodes.length > 0 && !matching.size && <p className="debug-empty">未找到对象</p>}
      </div>
    </section>
    <section className="debug-details">
      <div className="debug-detail-tabs" aria-label="对象详情">
        {tabs.map((name) => <Button key={name} variant="ghost" className="debug-pill" aria-pressed={tab === name} onPress={() => setTab(name)}>{name}</Button>)}
      </div>
      {selected && <>
        <div className="debug-detail-header">
          <img src={eye} alt="" /> <span>{nodeName(selected)}</span>
          <Button isIconOnly variant="ghost" className="debug-icon" aria-label="查看布局" onPress={() => setTab('布局')}><img src={bug} alt="" /></Button>
          <Button isIconOnly variant="ghost" className="debug-icon" aria-label="查看属性" onPress={() => setTab('属性')}><img src={brackets} alt="" /></Button>
        </div>
        <div className="debug-detail-body"><NodeDetails node={selected} tab={tab} /></div>
        <div className="debug-detail-footer"><span>渲染引擎</span><span><img src={at} alt="" /> LVGL 9.4</span></div>
      </>}
    </section>
  </>;
}
