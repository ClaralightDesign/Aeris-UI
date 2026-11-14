export type PreviewNode = {
  id: number;
  parent: number;
  kind: number;
  value: string;
  x: number;
  y: number;
  width: number;
  height: number;
  contentX: number;
  contentY: number;
  contentWidth: number;
  contentHeight: number;
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
  alignment: number;
  checked: boolean;
  disabled: boolean;
};

export type Snapshot = {
  height: number;
  nodes: PreviewNode[];
  frameTime: number;
  memoryBytes: number;
};

export type PreviewWindow = Window & {
  aerisDebugger?: { snapshot: () => Snapshot | null; screenshot: () => string | null };
};

export const width = 336;
export const kindName = ['屏幕', '文字', '图片', '容器'];

export function nodeName(node: PreviewNode) {
  return node.kind === 1 && node.value ? node.value : kindName[node.kind];
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
