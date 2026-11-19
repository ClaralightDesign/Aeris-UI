export type BoxEdges = [top: number, right: number, bottom: number, left: number];
export type Affine = [a: number, b: number, c: number, d: number, e: number, f: number];
type Backdrop = { x: number; y: number; width: number; height: number; opacity: number; radius: number };

export type PreviewNode = {
  id: number;
  parent: number;
  kind: number;
  className?: string;
  component?: string;
  utilityClasses?: string;
  value: string;
  x: number;
  y: number;
  width: number;
  height: number;
  contentX: number;
  contentY: number;
  contentWidth: number;
  contentHeight: number;
  margin?: BoxEdges;
  borderWidths?: BoxEdges;
  hidden: boolean;
  clickable: boolean;
  scrollable: boolean;
  overflow: boolean;
  events: number;
  background: number;
  foreground: number;
  backgroundOpacity: number;
  opacity: number;
  radius: number;
  border: number;
  fontHeight: number;
  fontSize?: number;
  fontApx?: number;
  renderedFontSize?: number;
  renderedLineHeight?: number;
  fontFamily?: string;
  fontWeight?: number;
  alignment: number;
  checked: boolean;
  disabled: boolean;
  layerGeometry?: { clipRadius: number; outline?: 'circle'; transform?: Affine; backdrop?: Backdrop };
};

export type Snapshot = {
  height: number;
  nodes: PreviewNode[];
  layerNodes?: PreviewNode[];
  frameTime: number;
  memoryBytes: number;
};

export type RenderSample = { at: number; durationMs: number; submissions: number };
export type PreviewPerformance = {
  read: () => { lastRender: RenderSample; submissions: number; memoryBytes: number } | null;
  subscribe: (listener: (sample: RenderSample | null) => void) => () => void;
};

export type PreviewWindow = Window & {
  aerisDebugger?: {
    snapshot: (options?: { layers?: boolean }) => Snapshot | null;
    screenshot: () => string | null;
    contains?: (x: number, y: number) => boolean;
    performance?: PreviewPerformance;
  };
};

export type DeviceProfile = {
  id: string;
  name: string;
  width: number;
  height: number;
  shape: 'rectangle' | 'capsule' | 'circle';
};

export const defaultDevice: DeviceProfile = {
  id: 'xiaomi-band-10-pro-3.101.043',
  name: '小米手环 10 Pro',
  width: 336,
  height: 480,
  shape: 'rectangle',
};
const kindName = ['lv_screen', 'lv_label', 'lv_image', 'lv_obj'];

export function nodeClass(node: PreviewNode) {
  return node.className || kindName[node.kind] || 'lv_obj';
}

export function nodeName(node: PreviewNode) {
  if (node.layerGeometry?.backdrop && node.kind === 2 && !node.component) return '背景模糊';
  return node.className === 'NinePatch' ? node.className : node.component || nodeClass(node);
}

export function pickNode(nodes: PreviewNode[], x: number, y: number) {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  function contains(node: PreviewNode) {
    return node.x <= x && node.y <= y && x < node.x + node.width && y < node.y + node.height;
  }
  for (let index = nodes.length - 1; index >= 0; index--) {
    const node = nodes[index];
    if (node.hidden || !contains(node)) continue;
    let parent = byId.get(node.parent);
    while (parent && (parent.overflow || contains(parent))) parent = byId.get(parent.parent);
    if (!parent) return node;
  }
}
