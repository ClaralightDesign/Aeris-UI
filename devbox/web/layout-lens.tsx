import { useEffect, useId, useMemo, useRef, useState, type PointerEvent, type RefObject } from 'react';
import { type DeviceProfile, type PreviewNode, type PreviewWindow, type Snapshot } from './preview';

export type Point = { x: number; y: number };
export type Rect = Point & { width: number; height: number };

export function useLayoutModifier(frames: RefObject<Map<string, HTMLIFrameElement>>, frameReady: number) {
  const [held, setHeld] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    const { signal } = controller;
    const options = { capture: true, signal };
    const clear = () => setHeld(false);
    function key(event: KeyboardEvent) {
      if (event.altKey && !event.ctrlKey && !event.metaKey && !event.getModifierState('AltGraph')) {
        if (event.key === 'Alt') event.preventDefault();
        setHeld(true);
      } else clear();
    }
    function bind(target: Window) {
      target.addEventListener('keydown', key, options);
      target.addEventListener('keyup', key, options);
      target.addEventListener('blur', clear, { signal });
      target.addEventListener('pointermove', (event) => {
        if (event.pointerType !== 'mouse' && event.pointerType !== 'pen') return;
        setHeld(event.altKey && !event.ctrlKey && !event.metaKey);
      }, options);
    }
    bind(window);
    for (const frame of frames.current.values()) {
      if (frame.contentWindow) bind(frame.contentWindow);
    }
    document.addEventListener('visibilitychange', () => { if (document.hidden) clear(); }, { signal });
    return () => controller.abort();
  }, [frames, frameReady]);

  return held;
}

function intersect(a: Rect, b: Rect): Rect | undefined {
  const x = Math.max(a.x, b.x);
  const y = Math.max(a.y, b.y);
  const width = Math.min(a.x + a.width, b.x + b.width) - x;
  const height = Math.min(a.y + a.height, b.y + b.height) - y;
  return width > 0 && height > 0 ? { x, y, width, height } : undefined;
}

export function layoutAreaForDevice(area: Rect, x: number, y: number, width: number, height: number) {
  return intersect({ ...area, x: area.x - x, y: area.y - y }, { x: 0, y: 0, width, height });
}

function visibleRects(nodes: PreviewNode[], width: number, height: number) {
  const screen = { x: 0, y: 0, width, height };
  const clips = new Map<number, Rect | undefined>([[0, screen]]);
  const visible = new Map<number, Rect>();
  // The snapshot is in tree order. Carry the inherited clip through overflow-visible parents.
  for (const node of nodes) {
    const clip = clips.get(node.parent);
    const shown = !node.hidden && node.opacity > 0;
    const rect = clip && shown ? intersect(clip, node) : undefined;
    if (rect) visible.set(node.id, rect);
    clips.set(node.id, shown ? node.overflow ? clip : rect : undefined);
  }
  return visible;
}

function cornerRadius(node: PreviewNode) {
  // LVGL also uses a large sentinel for a fully rounded object.
  return Math.max(0, Math.min(node.radius, Math.floor(Math.min(node.width, node.height) / 2)));
}

function dimensions({ width, height }: Rect) {
  return `${Math.round(width)} × ${Math.round(height)}`;
}

type ElementRole = 'group' | 'control' | 'text' | 'detail';
type MeasuredElement = { node: PreviewNode; visible: Rect; role: ElementRole; radius: number };
type Label = { id: string; text: string; box: Rect };

function layoutElements(nodes: PreviewNode[], width: number, height: number) {
  const visible = visibleRects(nodes, width, height);
  const children = new Map<number, PreviewNode[]>();
  for (const node of nodes) {
    if (!visible.has(node.id)) continue;
    const siblings = children.get(node.parent);
    if (siblings) siblings.push(node);
    else children.set(node.parent, [node]);
  }

  const screenBackground = nodes.find((node) => node.kind === 0)?.background ?? 0;
  const elements = new Map<string, MeasuredElement>();
  for (const node of nodes) {
    const rect = visible.get(node.id);
    if (!rect || node.kind === 0 || rect.width === width && rect.height === height) continue;
    const descendants = children.get(node.id) ?? [];
    const image = node.kind === 2 || node.className === 'NinePatch';
    const imageFace = node.component && descendants.some((child) => child.kind === 2 && sameRect(node, child));
    const radius = cornerRadius(node);
    const painted = node.border > 0 || node.backgroundOpacity > 0 &&
      (node.background !== screenBackground || node.clickable || radius > 0);
    const small = node.width <= 16 && node.height <= 16;
    let role: ElementRole;
    if (node.kind === 1 && node.value.trim()) role = 'text';
    else if (node.kind === 1) continue;
    else if (node.width < 4 || node.height < 4 || small ||
      image && !node.component && node.className !== 'NinePatch') role = 'detail';
    else if (image || imageFace || painted || node.component && radius > 0) {
      const hasContent = node.width >= 24 && node.height >= 24 && descendants.some((child) => child.kind !== 2);
      role = hasContent ? 'group' : 'control';
    } else continue;

    const key = `${node.x},${node.y},${node.width},${node.height},${radius},${role === 'text' ? node.value : ''}`;
    // A track and its fill, or a component and its image, can share exactly the same bounds.
    const existing = elements.get(key);
    if (!existing || existing.role === 'detail') elements.set(key, { node, visible: rect, role, radius });
  }
  return [...elements.values()];
}

function sameRect(a: Rect, b: Rect) {
  return a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;
}

function contains(outer: Rect, inner: Rect) {
  const tolerance = 0.001;
  return outer.x <= inner.x + tolerance && outer.y <= inner.y + tolerance &&
    outer.x + outer.width >= inner.x + inner.width - tolerance &&
    outer.y + outer.height >= inner.y + inner.height - tolerance;
}

function placeLabels(elements: MeasuredElement[], screen: Rect, scale: number) {
  const labels: Label[] = [];
  const gap = 5 / scale;
  const height = 20 / scale;
  const available = { x: screen.x + gap, y: screen.y + gap, width: screen.width - gap * 2, height: screen.height - gap * 2 };

  function place(element: MeasuredElement, radius = false) {
    const { node, visible, role } = element;
    const text = radius ? `r ${element.radius}` : dimensions(node);
    const width = (text.length * 7 + 12) / scale;
    const centerX = node.x + (node.width - width) / 2;
    const centerY = node.y + (node.height - height) / 2;
    const right = node.x + node.width;
    const bottom = node.y + node.height;
    const above = { x: centerX, y: node.y - height - gap };
    const below = { x: centerX, y: bottom + gap };
    const beside = { x: right + gap, y: centerY };
    const before = { x: node.x - width - gap, y: centerY };
    const inside = { x: centerX, y: bottom - height - gap };
    let anchors: Point[];
    if (radius) {
      const leftCorner = Math.max(node.x + gap, available.x);
      const rightCorner = Math.min(right - width - gap, available.x + available.width - width);
      const inset = element.radius + gap;
      anchors = [];
      for (const y of [node.y - height / 2, node.y + gap, bottom - height - gap]) {
        if (leftCorner <= node.x + inset) anchors.push({ x: leftCorner, y });
        if (rightCorner + width >= right - inset) anchors.push({ x: rightCorner, y });
      }
    } else {
      anchors = role === 'text' ? [beside, below, above, before] : [below, above, beside, before, inside];
    }

    for (const anchor of anchors) {
      const box = { ...anchor, width, height };
      if (!contains(available, box)) continue;
      // Keep labels attached to edges that survived the parent's clipping.
      const near = { x: box.x - gap, y: box.y - gap, width: width + gap * 2, height: height + gap * 2 };
      const touches = visible.x <= near.x + near.width && visible.x + visible.width >= near.x &&
        visible.y <= near.y + near.height && visible.y + visible.height >= near.y;
      if (!touches || labels.some((label) => intersect(label.box, near))) continue;
      labels.push({ id: `${node.id}-${radius ? 'radius' : 'size'}`, text, box });
      return;
    }
  }

  const shapes = elements.filter(({ role }) => role !== 'text');
  const groups = shapes.filter(({ role }) => role === 'group');
  // Group corners have priority, followed by dimensions for primary controls.
  for (const group of groups) if (group.radius > 0) place(group, true);
  for (const element of shapes) place(element);
  for (const element of shapes) if (element.role !== 'group' && element.radius > 0) place(element, true);

  const labeledTextParents = new Set<number>();
  for (const element of elements) {
    if (element.role !== 'text' || labeledTextParents.has(element.node.parent)) continue;
    if (shapes.some((shape) => contains(shape.node, element.node))) continue;
    place(element);
    labeledTextParents.add(element.node.parent);
  }
  return labels;
}

export function LayoutSelection({ area, onArea, scale, panX, panY, onPoint }: {
  area?: Rect;
  onArea: (area?: Rect) => void;
  scale: number;
  panX: number;
  panY: number;
  onPoint?: (point?: Point) => void;
}) {
  const drag = useRef<{ id: number; start: Point; client: Point }>(undefined);
  function coordinates(event: PointerEvent<HTMLDivElement>): Point {
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = Math.max(0, Math.min(bounds.width, event.clientX - bounds.left));
    const y = Math.max(0, Math.min(bounds.height, event.clientY - bounds.top));
    return { x: (x - bounds.width / 2 - panX) / scale, y: (y - bounds.height / 2 - panY) / scale };
  }

  function update(event: PointerEvent<HTMLDivElement>) {
    const start = drag.current;
    if (!onPoint && !start) return;
    const at = coordinates(event);
    onPoint?.(at);
    if (!start || start.id !== event.pointerId) return;
    const dx = Math.abs(at.x - start.start.x);
    const dy = Math.abs(at.y - start.start.y);
    onArea(dx > 0 && dy > 0 && Math.hypot(event.clientX - start.client.x, event.clientY - start.client.y) >= 3 ? {
      x: Math.min(at.x, start.start.x), y: Math.min(at.y, start.start.y), width: dx, height: dy,
    } : undefined);
  }

  return <div className="debug-layout-surface"
    onPointerEnter={update} onPointerMove={update}
    onPointerLeave={() => {
      if (drag.current) return;
      onPoint?.(undefined);
    }}
    onPointerDown={(event) => {
      if (event.button !== 0 || drag.current) return;
      const client = { x: event.clientX, y: event.clientY };
      const start = coordinates(event);
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      onArea(undefined);
      drag.current = { id: event.pointerId, start, client };
      onPoint?.(start);
    }}
    onPointerUp={(event) => {
      if (drag.current?.id !== event.pointerId) return;
      update(event);
      drag.current = undefined;
    }}
    onLostPointerCapture={() => { drag.current = undefined; }}
    onPointerCancel={() => {
      drag.current = undefined;
      onArea(undefined);
      onPoint?.(undefined);
    }}>
    {area && <>
      <div className="debug-layout-marquee" style={{
        left: `calc(50% + ${panX + area.x * scale}px)`,
        top: `calc(50% + ${panY + area.y * scale}px)`,
        width: area.width * scale, height: area.height * scale,
      }} aria-hidden="true" />
      <div className="debug-layout-badge" style={{
        left: `clamp(44px, 50% + ${panX + (area.x + area.width / 2) * scale}px, 100% - 44px)`,
        top: `clamp(0px, 50% + ${panY + (area.y + area.height) * scale + 6}px, 100% - 24px)`,
      }}>
        <div aria-label={`框选区域 ${dimensions(area)} 像素`}>{dimensions(area)}</div>
      </div>
    </>}
  </div>;
}

export function LayoutLens({ profile, height, scale, frame, activeSnapshot, area }: {
  profile: DeviceProfile;
  height: number;
  scale: number;
  frame?: HTMLIFrameElement;
  activeSnapshot?: Snapshot;
  area: Rect;
}) {
  const { width } = profile;
  const [localSnapshot, setLocalSnapshot] = useState<Snapshot>();
  const clipId = useId();
  const renderer = (frame?.contentWindow as PreviewWindow | null | undefined)?.aerisDebugger;
  const nodes = (activeSnapshot ?? localSnapshot)?.nodes;
  const layout = useMemo(() => layoutElements(nodes ?? [], width, height), [nodes, width, height]);
  const primary = useMemo(() => layout.filter(({ role }) => role !== 'detail'), [layout]);
  // Position labels once for the screen, then reveal them through the same clip as the outlines.
  const labels = useMemo(() => placeLabels(primary.length ? primary : layout, { x: 0, y: 0, width, height }, scale),
    [primary, layout, width, height, scale]);

  useEffect(() => {
    if (activeSnapshot || !renderer) return;
    function sample() {
      setLocalSnapshot(renderer?.snapshot() ?? undefined);
    }
    sample();
    const timer = window.setInterval(sample, 250);
    return () => window.clearInterval(timer);
  }, [activeSnapshot, renderer]);

  const screenRadius = profile.shape === 'rectangle' ? 0 : Math.min(width, profile.height) / 2;
  const intersecting = layout.filter(({ visible }) => intersect(visible, area));
  const visiblePrimary = intersecting.filter(({ role }) => role !== 'detail');
  const elements = visiblePrimary.length ? visiblePrimary : intersecting.slice(-1);

  return <><div className="debug-layout-paper"
      style={{
        left: area.x, top: area.y, width: area.width, height: area.height,
        clipPath: screenRadius
          ? `inset(${-area.y}px ${area.x + area.width - width}px ${area.y + area.height - profile.height}px ${-area.x}px round ${screenRadius}px)`
          : undefined,
      }} aria-hidden="true" />
    <svg className="debug-layout-drawing" viewBox={`0 0 ${width} ${height}`}
      style={screenRadius ? { clipPath: `inset(0 0 ${height - profile.height}px round ${screenRadius}px)` } : undefined}>
      <clipPath id={clipId}><rect {...area} /></clipPath>
      <g clipPath={`url(#${clipId})`} aria-hidden="true">
        {elements.map(({ node, visible: rect, role, radius }) => {
          const clipped = !sameRect(rect, node);
          const id = clipId + '-' + node.id;
          return <g key={node.id}>
            {clipped && <clipPath id={id}><rect {...rect} /></clipPath>}
            <rect className="debug-layout-node" data-role={role}
              x={node.x} y={node.y} width={node.width} height={node.height}
              rx={radius} clipPath={clipped ? `url(#${id})` : undefined} />
          </g>;
        })}
      </g>
      <g clipPath={`url(#${clipId})`}>
        {labels.filter(({ box }) => intersect(box, area)).map(({ id, text, box }) => <g className="debug-layout-label" key={id}>
          <rect {...box} rx={6 / scale} />
          <text x={box.x + box.width / 2} y={box.y + box.height / 2} textAnchor="middle"
            dominantBaseline="central" fontSize={12 / scale}>{text}</text>
        </g>)}
      </g>
    </svg></>;
}
