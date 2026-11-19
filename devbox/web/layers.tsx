import { ArrowClockwiseIcon } from '@phosphor-icons/react/dist/csr/ArrowClockwise';
import { Button } from '@heroui/react';
import { useId, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { useCanvasViewport } from './canvas-viewport';
import { CanvasZoom } from './canvas-zoom';
import { ElementDetails } from './inspector';
import { nodeName, type Affine, type DeviceProfile, type PreviewNode } from './preview';
import { ScrollPane } from './scroll-pane';

type Rect = { x: number; y: number; width: number; height: number };
type Outline = Rect & { radius: number; transform?: Affine };
type Clip = { id: string; outline: Outline; parent?: Clip };
type Layer = {
  node: PreviewNode;
  outline: Outline;
  shape: 'rectangle' | 'rounded' | 'circle' | 'bounds' | 'backdrop';
  clip: Clip;
};
type LayerGeometry = { crop: Rect; display: Outline; layers: Layer[]; clips: Clip[] };

function cornerRadius(rect: Rect, radius: number) {
  return Math.max(0, Math.min(radius, Math.floor(Math.min(rect.width, rect.height) / 2)));
}

function bounds({ x, y, width, height, transform }: Outline): Rect {
  if (!transform) return { x, y, width, height };
  const [a, b, c, d, e, f] = transform;
  const left = a * x + c * y + e + Math.min(0, a * width) + Math.min(0, c * height);
  const top = b * x + d * y + f + Math.min(0, b * width) + Math.min(0, d * height);
  return { x: left, y: top, width: Math.abs(a * width) + Math.abs(c * height), height: Math.abs(b * width) + Math.abs(d * height) };
}

function intersect(a: Rect, b: Rect): Rect | undefined {
  const x = Math.max(a.x, b.x);
  const y = Math.max(a.y, b.y);
  const width = Math.min(a.x + a.width, b.x + b.width) - x;
  const height = Math.min(a.y + a.height, b.y + b.height) - y;
  return width > 0 && height > 0 ? { x, y, width, height } : undefined;
}

export function buildLayerGeometry(nodes: PreviewNode[], device: DeviceProfile, height: number): LayerGeometry {
  const crop = { x: 0, y: 0, width: device.width, height };
  const display = { ...crop, height: device.height, radius: device.shape === 'rectangle'
    ? 0 : Math.min(device.width, device.height) / 2 };
  const cropClip: Clip = { id: 'crop', outline: { ...crop, radius: 0 } };
  const screenClip: Clip = { id: 'screen', outline: display, parent: cropClip };
  const clips: Clip[] = [cropClip, screenClip];
  const parents = new Map<number, { clip: Clip; visible: Rect }>([[0, { clip: screenClip, visible: crop }]]);
  const parentIds = new Set(nodes.map((node) => node.parent));
  const layers: Layer[] = [];
  for (const node of nodes) {
    const parent = parents.get(node.parent);
    if (node.hidden || node.opacity <= 0 || !parent) continue;
    const geometry = node.layerGeometry;
    const transform = geometry?.transform;
    const rect = geometry?.backdrop ?? node;
    const clipRadius = cornerRadius(node, geometry?.clipRadius ?? 0);
    const circle = geometry?.outline === 'circle';
    const hasOutline = node.kind !== 2 || clipRadius > 0 || circle;
    const radius = circle && geometry?.backdrop ? Math.min(rect.width, rect.height) / 2 : node.kind === 1 ? 0
      : cornerRadius(rect, node.kind === 2 && !geometry?.backdrop ? clipRadius : node.radius);
    const shape = geometry?.backdrop ? 'backdrop' : circle ? 'circle' : radius ? 'rounded'
      : !hasOutline || node.kind === 3 && !node.backgroundOpacity && !node.border ? 'bounds' : 'rectangle';
    const { x, y, width, height } = rect;
    const outline = { x, y, width, height, radius, transform };
    const visible = intersect(parent.visible, bounds(outline));
    if (visible && width > 0 && height > 0) layers.push({ node, outline, shape, clip: parent.clip });
    if (!parentIds.has(node.id)) continue;
    if (node.overflow && !clipRadius) {
      parents.set(node.id, parent);
    } else if (visible) {
      const clip = { id: 'node-' + node.id, outline: { ...outline, radius: clipRadius }, parent: parent.clip };
      clips.push(clip);
      parents.set(node.id, { clip, visible });
    }
  }
  return { crop, display, layers, clips };
}

function matrix(transform: Affine) {
  return `matrix(${transform.join(' ')})`;
}

function OutlineShape({ outline, circle, className, id, selected, shape }: {
  outline: Outline;
  circle?: boolean;
  className?: string;
  id?: number;
  selected?: boolean;
  shape?: Layer['shape'];
}) {
  const { x, y, width, height, radius, transform } = outline;
  const props = { className, transform: transform && matrix(transform), 'data-layer-id': id,
    'data-selected': selected, 'data-shape': shape };
  if (circle) return <ellipse {...props} cx={x + width / 2} cy={y + height / 2} rx={width / 2} ry={height / 2} />;
  return <rect {...props} x={x} y={y} width={width} height={height} rx={radius} />;
}

function ClipDefinitions({ clips, prefix }: { clips: Clip[]; prefix: string }) {
  return <defs>{clips.map(({ id, outline }) => <clipPath key={id} id={prefix + id}>
    <OutlineShape outline={outline} className="debug-layer-clip" />
  </clipPath>)}</defs>;
}

function clipped(shape: ReactNode, clip: Clip | undefined, prefix: string) {
  for (; clip; clip = clip.parent) shape = <g clipPath={`url(#${prefix + clip.id})`}>{shape}</g>;
  return shape;
}

function BackdropBorder({ layer, prefix, selected }: { layer: Layer; prefix: string; selected: boolean }) {
  const face = { id: 'backdrop-' + layer.node.id, outline: layer.outline };
  const edges = new Map<string, Clip>();
  for (let clip: Clip | undefined = { ...face, parent: layer.clip }; clip; clip = clip.parent) {
    const { x, y, width, height, radius, transform } = clip.outline;
    edges.set([x, y, width, height, radius, transform].join(':'), clip);
  }
  // The intersection boundary consists of edges of the face and its clips that survive every other clip.
  const boundary = [...edges.values()];
  return <g className="debug-layer-backdrop-border" data-selected={selected}>
    <ClipDefinitions clips={[face]} prefix={prefix} />
    {boundary.map(({ id, outline }) => <g key={id}>
      {boundary.reduce<ReactNode>((shape, limit) => limit.id === id ? shape
        : <g clipPath={`url(#${prefix + limit.id})`}>{shape}</g>,
      <OutlineShape outline={outline} className="debug-layer-backdrop-edge" />)}
    </g>)}
  </g>;
}

function LayerPlane({ layer, selected, prefix }: { layer: Layer; selected: boolean; prefix: string }) {
  const backdrop = layer.node.layerGeometry?.backdrop;
  const outline = layer.outline;
  return <>
    {clipped(<>
      {backdrop && <foreignObject className="debug-layer-backdrop" x={outline.x} y={outline.y}
        width={outline.width} height={outline.height} transform={outline.transform && matrix(outline.transform)}>
        <div className="debug-layer-backdrop-filter" style={{
          '--debug-layer-blur-radius': `${backdrop.radius}px`,
          borderRadius: outline.radius,
          opacity: layer.node.opacity / 255 * backdrop.opacity,
        } as CSSProperties} />
      </foreignObject>}
      <OutlineShape outline={outline} circle={layer.shape === 'circle'} className="debug-layer-outline"
        id={layer.node.id} selected={selected} shape={layer.shape} />
    </>, layer.clip, prefix)}
    {backdrop && <BackdropBorder layer={layer} prefix={prefix} selected={selected} />}
  </>;
}

export function LayerHighlight({ geometry, layer }: { geometry: LayerGeometry; layer: Layer }) {
  const prefix = useId();
  const { width, height } = geometry.crop;
  const clips: Clip[] = [];
  for (let clip: Clip | undefined = layer.clip; clip; clip = clip.parent) clips.push(clip);
  return <svg className="debug-box-highlight debug-layer-highlight" viewBox={`0 0 ${width} ${height}`} aria-hidden="true">
    <ClipDefinitions clips={clips} prefix={prefix} />
    {layer.shape === 'backdrop' ? <BackdropBorder layer={layer} prefix={prefix} selected />
      : clipped(<OutlineShape outline={layer.outline} circle={layer.shape === 'circle'} className="debug-layer-focus"
        id={layer.node.id} shape={layer.shape} />, layer.clip, prefix)}
  </svg>;
}

const initialCamera = { pitch: 0, yaw: 0 };
const radians = Math.PI / 180;
const closestTurn = (angle: number, target: number) => target + Math.round((angle - target) / 360) * 360;

function cameraAxes({ pitch, yaw }: typeof initialCamera) {
  const sinX = Math.sin(pitch * radians);
  const sinY = Math.sin(yaw * radians);
  const cosX = Math.cos(pitch * radians);
  const cosY = Math.cos(yaw * radians);
  return { a: cosY, b: sinX * sinY, d: cosX, depthX: sinY, depthY: -sinX * cosY };
}

function LayerScene({ geometry, selected, onSelect }: {
  geometry: LayerGeometry;
  selected?: number;
  onSelect: (id: number) => void;
}) {
  const prefix = useId();
  const [back, setBack] = useState(false);
  const [dragging, setDragging] = useState(false);
  const stage = useRef<HTMLDivElement>(null);
  const stack = useRef<SVGGElement>(null);
  const camera = useRef(initialCamera);
  const target = useRef(initialCamera);
  const frame = useRef<number>(undefined);
  const lastFrame = useRef<number>(undefined);
  const backRef = useRef(false);
  const [reducedMotion] = useState(() => matchMedia('(prefers-reduced-motion: reduce)'));
  const drag = useRef<{ pointer: number; x: number; y: number; pitch: number; yaw: number; target?: number; moved: boolean }>(undefined);
  const { layers, display, crop } = geometry;
  const { viewport, scale, space, dragging: panning, reset: resetZoom, fit, zoomIn, zoomOut, setPercent } =
    useCanvasViewport(stage, { enabled: layers.length > 0, scope: 'layers', customPrimaryDrag: true });
  const initialized = useRef(false);
  const displayBounds = useRef(crop);
  const { width, height } = crop;
  const step = Math.min(12, Math.min(width, height) * .44 / Math.max(1, layers.length));
  const first = -step * layers.length / 2;
  const planes = layers.map((layer, index) => ({ layer, depth: first + step * (index + 1) }));
  if (back) planes.reverse();
  const deviceOutline = <g key="display" data-depth={first}>
    {clipped(<OutlineShape outline={display} className="debug-layer-display" />, geometry.clips[0], prefix)}
  </g>;

  function project({ a, b, d, depthX, depthY } = cameraAxes(camera.current)) {
    if (!stack.current) return;
    const { width, height } = displayBounds.current;
    for (const plane of stack.current.children) {
      const depth = Number(plane.getAttribute('data-depth'));
      plane.setAttribute('transform', matrix([a, b, 0, d,
        depthX * depth - a * width / 2, depthY * depth - b * width / 2 - d * height / 2]));
    }
  }
  useLayoutEffect(() => {
    displayBounds.current = crop;
    project();
  }, [layers, crop, back]);
  useLayoutEffect(() => {
    const area = stage.current;
    if (!area || !layers.length || initialized.current || !area.clientHeight || !area.clientWidth) return;
    initialized.current = true;
    if (width * scale > area.clientWidth || height * scale > area.clientHeight) fit(width, height);
  }, [layers.length, width, height, scale, fit]);
  useLayoutEffect(() => () => {
    if (frame.current !== undefined) cancelAnimationFrame(frame.current);
  }, []);

  function animate(time: number) {
    frame.current = undefined;
    const elapsed = time - (lastFrame.current ?? (time - 16));
    const desired = target.current;
    const current = camera.current;
    const settled = reducedMotion.matches || (
      Math.abs(desired.pitch - current.pitch) < .08 && Math.abs(desired.yaw - current.yaw) < .08
    );
    const amount = 1 - Math.exp(-elapsed / (drag.current?.moved ? 24 : 38));
    camera.current = settled ? desired : {
      pitch: current.pitch + (desired.pitch - current.pitch) * amount,
      yaw: current.yaw + (desired.yaw - current.yaw) * amount,
    };
    const axes = cameraAxes(camera.current);
    const behind = axes.a * axes.d < 0;
    if (behind !== backRef.current) {
      backRef.current = behind;
      setBack(behind);
    }
    project(axes);
    lastFrame.current = settled ? undefined : time;
    if (!settled) frame.current = requestAnimationFrame(animate);
  }
  function moveCamera(desired: typeof initialCamera) {
    target.current = desired;
    if (frame.current === undefined) frame.current = requestAnimationFrame(animate);
  }
  function fitLayers() {
    const { a, b, d, depthX, depthY } = cameraAxes(target.current);
    const depth = step * layers.length;
    const padding = 12;
    fit(Math.abs(a) * width + Math.abs(depthX) * depth + padding,
      Math.abs(b) * width + Math.abs(d) * height + Math.abs(depthY) * depth + padding);
  }
  function reset() {
    resetZoom();
    moveCamera({
      pitch: closestTurn(camera.current.pitch, initialCamera.pitch),
      yaw: closestTurn(camera.current.yaw, initialCamera.yaw),
    });
  }
  function endDrag() {
    drag.current = undefined;
    setDragging(false);
  }

  return <section className="debug-layer-overview" aria-label="3D 图层预览" data-zoom-scope="layers">
    <div className="debug-panel-toolbar debug-layer-toolbar">
      <span className="debug-layer-title">3D 预览</span>
      <CanvasZoom label="图层缩放" scope="layers" disabled={!layers.length} viewport={viewport}
        fit={fitLayers} reset={resetZoom} zoomIn={zoomIn} zoomOut={zoomOut} setPercent={setPercent} />
      <Button isIconOnly variant="ghost" className="debug-icon" aria-label="重置图层视角" isDisabled={!layers.length}
        onPress={reset}><ArrowClockwiseIcon size={16} aria-hidden="true" /></Button>
    </div>
    <div ref={stage} className="debug-panel-surface debug-layer-scene" data-dragging={dragging || panning}
      data-panning={space} role="group" tabIndex={layers.length ? 0 : undefined}
      aria-label="拖动或用方向键旋转图层，点击轮廓选择元素"
      onKeyDown={(event) => {
        const dx = Number(event.key === 'ArrowRight') - Number(event.key === 'ArrowLeft');
        const dy = Number(event.key === 'ArrowUp') - Number(event.key === 'ArrowDown');
        if (!dx && !dy) return;
        event.preventDefault();
        const previous = target.current;
        moveCamera({ ...previous, pitch: previous.pitch + dy * 6, yaw: previous.yaw + dx * 6 });
      }}
      onPointerDown={(event) => {
        if (!layers.length || event.button !== 0 || drag.current) return;
        const id = (event.target as Element).closest('[data-layer-id]')?.getAttribute('data-layer-id');
        event.currentTarget.focus({ preventScroll: true });
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { pointer: event.pointerId, x: event.clientX, y: event.clientY,
          pitch: target.current.pitch, yaw: target.current.yaw,
          target: id == null ? undefined : Number(id), moved: false };
      }}
      onPointerMove={(event) => {
        const start = drag.current;
        if (!start || start.pointer !== event.pointerId) return;
        const dx = event.clientX - start.x;
        const dy = event.clientY - start.y;
        if (!start.moved && Math.hypot(dx, dy) < 4) return;
        if (!start.moved) {
          start.moved = true;
          setDragging(true);
        }
        moveCamera({ ...target.current, pitch: start.pitch - dy * .35, yaw: start.yaw + dx * .35 });
      }}
      onPointerUp={(event) => {
        const start = drag.current;
        if (!start || start.pointer !== event.pointerId) return;
        endDrag();
        if (!start.moved && Math.hypot(event.clientX - start.x, event.clientY - start.y) < 4 && start.target !== undefined) {
          onSelect(start.target);
        }
      }}
      onLostPointerCapture={endDrag} onPointerCancel={endDrag}>
      {layers.length ? <svg aria-hidden="true">
        <ClipDefinitions clips={geometry.clips} prefix={prefix} />
        <g className="debug-layer-origin">
          <g transform={`translate(${viewport.x} ${viewport.y}) scale(${scale})`}>
            <g ref={stack}>
              {!back && deviceOutline}
              {planes.map(({ layer, depth }) => <g key={layer.node.id} className="debug-layer-plane" data-depth={depth} data-root={!layer.node.parent}>
                <LayerPlane layer={layer} selected={selected === layer.node.id} prefix={prefix} />
              </g>)}
              {back && deviceOutline}
            </g>
          </g>
        </g>
      </svg> : <p className="debug-empty">等待可检查的图层</p>}
    </div>
  </section>;
}

export function LayersPanel({ geometry, selected, onSelect }: {
  geometry: LayerGeometry;
  selected?: Layer;
  onSelect: (id: number) => void;
}) {
  const rows = useRef(new Map<number, HTMLButtonElement>());
  const screen = geometry.layers.find(({ node }) => node.className === 'lv_screen');
  const entries = geometry.layers.filter((layer) => layer !== screen).reverse();
  if (screen) entries.push(screen);
  return <div className="debug-layers">
    <LayerScene geometry={geometry} selected={selected?.node.id} onSelect={(id) => {
      onSelect(id);
      rows.current.get(id)?.scrollIntoView({ block: 'nearest' });
    }} />
    <ScrollPane className="debug-panel-surface debug-layer-list" viewport={{ role: 'region', 'aria-label': '图层列表' }}>
      {entries.map(({ node, shape, outline }) => <Button
        key={node.id} ref={(button) => { if (button) rows.current.set(node.id, button); else rows.current.delete(node.id); }}
        variant="ghost" className="debug-layer-row" onPress={() => onSelect(node.id)} aria-pressed={selected?.node.id === node.id}
        data-node-id={node.id} data-shape={shape}>
        <span title={node.value || nodeName(node)}>{nodeName(node)}{node.value && <small>{node.value}</small>}</span>
        <span>
          {shape === 'bounds' && <small>对象边界</small>}
          {shape === 'backdrop' && nodeName(node) !== '背景模糊' && <small>背景模糊</small>}
          {outline.width} × {outline.height}
        </span>
      </Button>)}
      {!geometry.layers.length && <p className="debug-empty">暂无图层</p>}
    </ScrollPane>
    <ElementDetails selected={selected?.node} layer />
  </div>;
}
