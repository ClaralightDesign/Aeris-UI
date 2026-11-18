import { Fragment, useLayoutEffect, useRef, useState } from 'react';
import { nodeClass, nodeName, type BoxEdges, type PreviewNode } from './preview';
import { Smooth } from './smooth';
import { hexColor, nodeBoxModel as boxModel, styleProperties } from './node-properties';

type Rect = { x: number; y: number; width: number; height: number };
const zeroEdges: BoxEdges = [0, 0, 0, 0];

function inset(rect: Rect, [top, right, bottom, left]: BoxEdges, direction = 1): Rect {
  const x = Math.min(rect.x + rect.width, rect.x + Math.max(0, left) * direction);
  const y = Math.min(rect.y + rect.height, rect.y + Math.max(0, top) * direction);
  return {
    x, y, width: Math.max(0, rect.x + rect.width - Math.max(0, right) * direction - x),
    height: Math.max(0, rect.y + rect.height - Math.max(0, bottom) * direction - y),
  };
}

function rectPath({ x, y, width, height }: Rect) {
  return `M ${x} ${y} h ${width} v ${height} h ${-width} Z`;
}

export function BoxHighlight({ node, width, height }: { node: PreviewNode; width: number; height: number }) {
  const { border, padding, margin } = boxModel(node);
  const outer = inset(node, margin ?? zeroEdges, -1);
  const inner = inset(node, border);
  const content = inset(inner, padding);
  return <svg className="debug-box-highlight" viewBox={`0 0 ${width} ${height}`} aria-hidden="true">
    <path data-region="margin" d={rectPath(outer) + rectPath(node)} />
    <path data-region="border" d={rectPath(node) + rectPath(inner)} />
    <path data-region="padding" d={rectPath(inner) + rectPath(content)} />
    <path data-region="content" d={rectPath(content)} />
    <rect className="debug-box-outline" x={node.x} y={node.y} width={node.width} height={node.height} />
  </svg>;
}

export function ElementCard({ node, detailed, canvasWidth, canvasHeight, scale, panX, panY }: {
  node: PreviewNode;
  detailed: boolean;
  canvasWidth: number;
  canvasHeight: number;
  scale: number;
  panX: number;
  panY: number;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ stageWidth: 0, stageHeight: 0, cardWidth: 0, cardHeight: 0 });
  useLayoutEffect(() => {
    const card = ref.current;
    const stage = card?.parentElement;
    if (!card || !stage) return;
    const update = () => setSize({
      stageWidth: stage.clientWidth, stageHeight: stage.clientHeight,
      cardWidth: card.offsetWidth, cardHeight: card.offsetHeight,
    });
    const observer = new ResizeObserver(update);
    observer.observe(card);
    observer.observe(stage);
    update();
    return () => observer.disconnect();
  }, []);

  const { stageWidth, stageHeight, cardWidth, cardHeight } = size;
  const { margin } = boxModel(node);
  const edge = 8;
  const left = stageWidth / 2 + panX + (node.x - canvasWidth / 2 - Math.max(0, margin?.[3] ?? 0)) * scale;
  const top = stageHeight / 2 + panY + (node.y - canvasHeight / 2 - Math.max(0, margin?.[0] ?? 0)) * scale;
  const bottom = stageHeight / 2 + panY + (node.y + node.height - canvasHeight / 2 + Math.max(0, margin?.[2] ?? 0)) * scale;
  const below = stageHeight - Math.max(0, bottom);
  const above = Math.min(stageHeight, top);
  const placeBelow = below >= cardHeight + edge * 2 || below >= above;
  const cardX = Math.max(edge, Math.min(left, stageWidth - cardWidth - edge));
  const cardY = Math.max(edge, Math.min(placeBelow ? bottom + edge : top - edge - cardHeight, stageHeight - cardHeight - edge));

  return <div className="debug-element-card-anchor" ref={ref}
    style={{ left: cardX, top: cardY, visibility: stageWidth ? undefined : 'hidden' }}>
    <span className="debug-element-card-pointer" data-side={placeBelow ? 'top' : 'bottom'}
      style={{ left: Math.max(12, Math.min(cardWidth - 12, left - cardX + 12)) }} aria-hidden="true" />
    <Smooth radius={8} stroke={{ width: 1, color: 'var(--debug-divider)', opacity: 1 }}>
      <section className="debug-element-card" aria-label="画布元素信息">
        <header><strong>{nodeName(node)}</strong><span>{node.width} × {node.height} px</span></header>
        {detailed && <><dl>
          {styleProperties(node).filter((row) => !row.zero).map((row) => <Fragment key={row.label}>
            <dt data-region={row.region}>{row.label}</dt><dd>{row.color !== undefined && <span className="debug-element-swatch"
              style={{ backgroundColor: hexColor(row.color) }} />}{row.value}</dd>
          </Fragment>)}
        </dl>
        <h3>对象信息</h3>
        <dl>
          <dt>文字</dt><dd>{node.value || '—'}</dd>
          <dt>对象类型</dt><dd>{nodeClass(node)}</dd>
          <dt>可点击</dt><dd>{node.clickable ? '是' : '否'}</dd>
          <dt>禁用</dt><dd>{node.disabled ? '是' : '否'}</dd>
        </dl></>}
      </section>
    </Smooth>
  </div>;
}
