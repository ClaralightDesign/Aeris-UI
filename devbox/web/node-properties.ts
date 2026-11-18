import type { BoxEdges, PreviewNode } from './preview';

export type NodeProperty = { label: string; value: string; color?: number; zero?: boolean;
  region?: 'margin' | 'border' | 'padding' | 'content' };
export const formatNumber = (value: number) => String(Math.round(value * 100) / 100);
export const hexColor = (value: number) => '#' + value.toString(16).padStart(6, '0');

export function nodeBoxModel(node: PreviewNode) {
  const border = node.borderWidths ?? [node.border, node.border, node.border, node.border] as BoxEdges;
  const padding: BoxEdges = [node.contentY - node.y - border[0],
    node.x + node.width - node.contentX - node.contentWidth - border[1],
    node.y + node.height - node.contentY - node.contentHeight - border[2],
    node.contentX - node.x - border[3]];
  return { border, padding, margin: node.margin };
}

function formatEdges([top, right, bottom, left]: BoxEdges) {
  const values = left !== right ? [top, right, bottom, left]
    : top !== bottom ? [top, right, bottom] : top !== right ? [top, right] : [top];
  return values.map((value) => `${value}px`).join(' ');
}

export function boxProperties(node: PreviewNode): NodeProperty[] {
  const { margin, border, padding } = nodeBoxModel(node);
  return [
    { label: '外边距', value: margin ? formatEdges(margin) : '—', region: 'margin', zero: !margin?.some((v) => v !== 0) },
    { label: '边框', value: formatEdges(border), region: 'border', zero: border.every((v) => v === 0) },
    { label: '内边距', value: formatEdges(padding), region: 'padding', zero: padding.every((v) => v === 0) },
    { label: '内容区', value: `${node.contentWidth} × ${node.contentHeight} px`, region: 'content',
      zero: node.contentWidth === 0 || node.contentHeight === 0 },
  ];
}

/** One source of truth for the inspector and canvas picker. Font size and
 * line height remain distinct; transformed size comes from the renderer. */
export function textProperties(node: PreviewNode): NodeProperty[] {
  if (node.kind !== 1) return [];
  const pixels = node.fontSize === undefined ? undefined : `${formatNumber(node.fontSize)} px`;
  return [
    { label: '字号', value: pixels === undefined ? '未提供' : node.fontApx === undefined
      ? pixels : `${formatNumber(node.fontApx)} apx → ${pixels}` },
    { label: '实际渲染字号', value: node.renderedFontSize === undefined ? '未提供' : `${formatNumber(node.renderedFontSize)} px` },
    { label: '文字颜色', value: hexColor(node.foreground), color: node.foreground },
    ...(node.fontFamily ? [{ label: '字体', value: node.fontFamily }] : []),
    ...(node.fontWeight ? [{ label: '字重', value: String(node.fontWeight) }] : []),
    { label: '文字行高', value: `${formatNumber(node.renderedLineHeight ?? node.fontHeight)} px`,
      zero: (node.renderedLineHeight ?? node.fontHeight) === 0 },
    { label: '文字对齐', value: ['自动', '左对齐', '居中', '右对齐'][node.alignment] ?? '自动' },
  ];
}

export function styleProperties(node: PreviewNode): NodeProperty[] {
  return [
    ...textProperties(node),
    { label: '背景颜色', value: hexColor(node.background), color: node.background, zero: node.backgroundOpacity === 0 },
    { label: '背景透明度', value: `${Math.round(node.backgroundOpacity / 2.55)}%`, zero: node.backgroundOpacity === 0 },
    { label: '整体透明度', value: `${Math.round(node.opacity / 2.55)}%`, zero: node.opacity === 0 },
    { label: '圆角', value: `${node.radius} px`, zero: node.radius === 0 },
    { label: '边框宽度', value: `${node.border} px`, zero: node.border === 0 },
    ...boxProperties(node),
  ];
}
