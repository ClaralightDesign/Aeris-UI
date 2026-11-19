import { useEffect, useRef, useState, type RefObject } from 'react';

export const MIN_ZOOM = 5;
export const MAX_ZOOM = 6400;
const ZOOM_STEP = 1.2;
const BASE_SCALE = 0.5;
const WHEEL_IDLE_MS = 200;
const BACK_DISTANCE = 48;

type Point = { x: number; y: number };
type Viewport = Point & { percent: number; showZoomFeedback: boolean };
type GestureEvent = MouseEvent & { scale: number };
const home: Viewport = { x: 0, y: 0, percent: 100, showZoomFeedback: false };

type ViewportOptions = {
  preview?: RefObject<HTMLIFrameElement | null>;
  frameReady?: number;
  previewUrl?: string;
  enabled?: boolean;
  scope?: string;
  customPrimaryDrag?: boolean;
};

export function useCanvasViewport(
  stage: RefObject<HTMLDivElement | null>,
  { preview, frameReady, previewUrl, enabled, scope, customPrimaryDrag }: ViewportOptions,
) {
  const [viewport, setViewport] = useState(home);
  const [space, setSpace] = useState(false);
  const [dragging, setDragging] = useState(false);
  const heldSpace = useRef(false);
  const pan = useRef<{ pointerId: number; at: Point; capture: Element }>(undefined);
  const gesture = useRef<number>(undefined);

  function zoom(update: (percent: number) => number, anchor: Point = { x: 0, y: 0 }, showZoomFeedback = true) {
    setViewport((current) => {
      const percent = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, update(current.percent)));
      if (percent === current.percent) return current;
      const factor = percent / current.percent;
      return { percent, showZoomFeedback, x: anchor.x + (current.x - anchor.x) * factor, y: anchor.y + (current.y - anchor.y) * factor };
    });
  }

  function move(dx: number, dy: number) {
    setViewport((current) => ({ ...current, x: current.x + dx, y: current.y + dy }));
  }

  function zoomIn(showZoomFeedback: boolean) {
    zoom((percent) => percent * ZOOM_STEP, undefined, showZoomFeedback);
  }

  function zoomOut(showZoomFeedback: boolean) {
    zoom((percent) => percent / ZOOM_STEP, undefined, showZoomFeedback);
  }

  function reset(showZoomFeedback: boolean) {
    setViewport({ ...home, showZoomFeedback });
  }

  function fit(width: number, height: number) {
    const area = stage.current;
    if (!area) return;
    const padding = getComputedStyle(area);
    const availableWidth = area.clientWidth - parseFloat(padding.paddingLeft) - parseFloat(padding.paddingRight);
    const availableHeight = area.clientHeight - parseFloat(padding.paddingTop) - parseFloat(padding.paddingBottom);
    if (availableWidth <= 0 || availableHeight <= 0 || height <= 0) return;
    const percent = Math.min(availableWidth / width, availableHeight / height) / BASE_SCALE * 100;
    setViewport({ ...home, percent: Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, Math.floor(percent * 10) / 10)) });
  }

  useEffect(() => {
    const canvasStage = stage.current;
    if (!canvasStage) return;
    const iframe = preview?.current;
    const frame = iframe?.contentWindow;
    const ready = () => enabled ?? Boolean(preview?.current);
    const cleanups: (() => void)[] = [];
    let blockClick = false;
    let frameWheel: { at: number; x: number; y: number; intent?: 'scroll' | 'back' | 'handled' } | undefined;

    function bind(target: EventTarget, event: string, handler: EventListener, options?: AddEventListenerOptions) {
      target.addEventListener(event, handler, options);
      cleanups.push(() => target.removeEventListener(event, handler, options));
    }

    function point(event: MouseEvent, fromFrame: boolean): Point {
      if (fromFrame && iframe && frame) {
        const rect = iframe.getBoundingClientRect();
        return { x: rect.left + event.clientX * rect.width / frame.innerWidth,
          y: rect.top + event.clientY * rect.height / frame.innerHeight };
      }
      return { x: event.clientX, y: event.clientY };
    }

    function anchor(event: MouseEvent, fromFrame: boolean): Point {
      const rect = canvasStage!.getBoundingClientRect();
      const at = point(event, fromFrame);
      return { x: at.x - rect.left - rect.width / 2, y: at.y - rect.top - rect.height / 2 };
    }

    function setSpaceHeld(value: boolean) {
      heldSpace.current = value;
      setSpace(value);
    }

    function stopPan(pointerId?: number) {
      const active = pan.current;
      if (!active || (pointerId !== undefined && pointerId !== active.pointerId)) return;
      pan.current = undefined;
      if (active.capture.hasPointerCapture(active.pointerId)) active.capture.releasePointerCapture(active.pointerId);
      setDragging(false);
    }

    function finishInteraction() {
      setSpaceHeld(false);
      stopPan();
      gesture.current = undefined;
      frameWheel = undefined;
    }

    function keyDown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.isComposing || !ready()) return;
      const target = event.target as Element | null;
      if (target?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable=false])')) return;
      const scopedViewport = target?.closest?.('[data-zoom-scope]') ??
        (event.currentTarget === window ? document.querySelector('[data-zoom-scope]:hover') : null);
      if ((scopedViewport?.getAttribute('data-zoom-scope') || undefined) !== scope) return;
      if ((event.metaKey || event.ctrlKey) && !event.altKey) {
        if (event.key === '0') reset(true);
        else if (event.key === '+' || event.key === '=') zoomIn(true);
        else if (event.key === '-' || event.key === '_') zoomOut(true);
        else return;
        event.preventDefault();
      } else if (event.code === 'Space' && !event.altKey &&
          (!target?.closest?.('button, a, [role=button]') || event.repeat && heldSpace.current)) {
        event.preventDefault();
        setSpaceHeld(true);
      }
    }

    function keyUp(event: KeyboardEvent) {
      if (event.code === 'Space') setSpaceHeld(false);
    }

    function handleFrameWheel(event: WheelEvent) {
      // Trackpads report pixel deltas. With natural scrolling, a right swipe produces negative X.
      if (event.deltaMode !== WheelEvent.DOM_DELTA_PIXEL) return;
      if (!frameWheel || event.timeStamp - frameWheel.at > WHEEL_IDLE_MS) {
        frameWheel = { at: event.timeStamp, x: 0, y: 0 };
      }
      frameWheel.at = event.timeStamp;
      frameWheel.x -= event.deltaX;
      frameWheel.y += Math.abs(event.deltaY);
      const movingRight = frameWheel.x > frameWheel.y * 1.5;
      if (!frameWheel.intent && Math.max(Math.abs(frameWheel.x), frameWheel.y) >= 8) {
        frameWheel.intent = movingRight ? 'back' : 'scroll';
      }
      if (frameWheel.intent === 'scroll' || (!frameWheel.intent && !movingRight)) return;

      event.preventDefault();
      event.stopPropagation();
      if (movingRight && frameWheel.x >= BACK_DISTANCE && frameWheel.intent === 'back') {
        frameWheel.intent = 'handled';
        // Use the preview's Escape handler, including when the iframe has not been clicked.
        frame?.document.querySelector('canvas')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      }
    }

    function panPoint(event: PointerEvent, fromFrame: boolean) {
      // Screen coordinates stay stable as the iframe moves under a captured pointer.
      return fromFrame ? { x: event.screenX, y: event.screenY } : { x: event.clientX, y: event.clientY };
    }

    function bindPointerInput(target: EventTarget, fromFrame: boolean) {
      const options = { capture: true, passive: false };
      bind(target, 'wheel', (raw) => {
        if (!ready()) return;
        const event = raw as WheelEvent;
        const zooming = event.ctrlKey || event.metaKey;
        if (fromFrame && !zooming && gesture.current === undefined && !pan.current) {
          handleFrameWheel(event);
          return;
        }
        frameWheel = undefined;
        event.preventDefault();
        if (gesture.current !== undefined || pan.current) {
          event.stopPropagation();
          return;
        }
        const units = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? canvasStage!.clientHeight : 1;
        if (zooming) {
          event.stopPropagation();
          zoom((percent) => percent * Math.exp(-event.deltaY * units * 0.01), anchor(event, fromFrame));
        } else if (!(event.target as Element | null)?.closest('.debug-device-frame, .debug-area-surface')) {
          move(-event.deltaX * units, -event.deltaY * units);
        }
      }, options);

      bind(target, 'gesturestart', (event) => {
        if (!ready()) return;
        event.preventDefault();
        frameWheel = undefined;
        gesture.current = 1;
      }, options);
      bind(target, 'gesturechange', (raw) => {
        const event = raw as GestureEvent;
        const previous = gesture.current;
        if (previous === undefined) return;
        event.preventDefault();
        if (pan.current || !Number.isFinite(event.scale) || event.scale <= 0) return;
        zoom((percent) => percent * event.scale / previous, anchor(event, fromFrame));
        gesture.current = event.scale;
      }, options);
      bind(target, 'gestureend', (event) => {
        if (gesture.current === undefined) return;
        event.preventDefault();
        gesture.current = undefined;
      }, options);

      bind(target, 'pointerdown', (raw) => {
        const event = raw as PointerEvent;
        if (!ready() || pan.current) return;
        blockClick = false;
        const primaryPan = event.button === 0 && (heldSpace.current || !customPrimaryDrag && event.target === canvasStage);
        if (event.button !== 1 && !primaryPan) return;
        const capture = fromFrame ? event.target as Element : canvasStage!;
        event.preventDefault();
        event.stopPropagation();
        capture.setPointerCapture(event.pointerId);
        pan.current = { pointerId: event.pointerId, at: panPoint(event, fromFrame), capture };
        blockClick = true;
        setDragging(true);
      }, options);
      bind(target, 'pointermove', (raw) => {
        const event = raw as PointerEvent;
        const active = pan.current;
        if (active?.pointerId !== event.pointerId) return;
        event.stopPropagation();
        const at = panPoint(event, fromFrame);
        move(at.x - active.at.x, at.y - active.at.y);
        active.at = at;
      }, options);
      for (const event of ['pointerup', 'pointercancel', 'lostpointercapture']) {
        bind(target, event, (raw) => {
          const pointer = raw as PointerEvent;
          if (pan.current?.pointerId !== pointer.pointerId) return;
          if (event !== 'lostpointercapture') pointer.stopPropagation();
          stopPan(pointer.pointerId);
        }, options);
      }
      bind(target, 'auxclick', (raw) => {
        if ((raw as MouseEvent).button === 1) raw.preventDefault();
      }, options);
      bind(target, 'click', (raw) => {
        if (!blockClick) return;
        raw.preventDefault();
        raw.stopPropagation();
        blockClick = false;
      }, options);
    }

    bindPointerInput(canvasStage, false);
    if (frame) bindPointerInput(frame, true);
    for (const target of frame ? [window, frame] : [window]) {
      bind(target, 'keydown', (event) => keyDown(event as KeyboardEvent), { capture: true });
      bind(target, 'keyup', (event) => keyUp(event as KeyboardEvent), { capture: true });
    }
    bind(window, 'blur', finishInteraction);
    return () => {
      cleanups.forEach((cleanup) => cleanup());
      finishInteraction();
    };
  }, [stage, preview, frameReady, previewUrl, enabled, scope, customPrimaryDrag]);

  return {
    viewport, scale: viewport.percent / 100 * BASE_SCALE, space, dragging,
    fit,
    reset: () => reset(false),
    zoomIn: () => zoomIn(false),
    zoomOut: () => zoomOut(false),
    setPercent: (percent: number) => zoom(() => percent, undefined, false),
  };
}
