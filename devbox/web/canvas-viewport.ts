import { useEffect, useRef, useState, type RefObject } from 'react';

export const MIN_ZOOM = 5;
export const MAX_ZOOM = 6400;
const ZOOM_STEP = 1.2;
const BASE_SCALE = 0.5;

type Point = { x: number; y: number };
type Viewport = Point & { percent: number };
type GestureEvent = MouseEvent & { scale: number };
const home: Viewport = { x: 0, y: 0, percent: 100 };

export function useCanvasViewport(
  stage: RefObject<HTMLDivElement | null>,
  preview: RefObject<HTMLIFrameElement | null>,
  frameReady: number,
  previewUrl: string,
) {
  const [viewport, setViewport] = useState(home);
  const [space, setSpace] = useState(false);
  const [dragging, setDragging] = useState(false);
  const heldSpace = useRef(false);
  const pan = useRef<{ pointerId: number; at: Point; capture: Element }>(undefined);
  const gesture = useRef<number>(undefined);

  function zoom(update: (percent: number) => number, anchor: Point = { x: 0, y: 0 }) {
    setViewport((current) => {
      const percent = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, update(current.percent)));
      if (percent === current.percent) return current;
      const factor = percent / current.percent;
      return { percent, x: anchor.x + (current.x - anchor.x) * factor, y: anchor.y + (current.y - anchor.y) * factor };
    });
  }

  function move(dx: number, dy: number) {
    setViewport((current) => ({ ...current, x: current.x + dx, y: current.y + dy }));
  }

  function reset() {
    setViewport(home);
  }

  useEffect(() => {
    const canvasStage = stage.current;
    if (!canvasStage) return;
    const iframe = preview.current;
    const frame = iframe?.contentWindow;
    const cleanups: (() => void)[] = [];
    let blockClick = false;

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
    }

    function keyDown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.isComposing || !preview.current) return;
      const target = event.target as Element | null;
      if (target?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable=false])')) return;
      if ((event.metaKey || event.ctrlKey) && !event.altKey) {
        if (event.key === '0') reset();
        else if (event.key === '+' || event.key === '=') zoom((percent) => percent * ZOOM_STEP);
        else if (event.key === '-' || event.key === '_') zoom((percent) => percent / ZOOM_STEP);
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

    function panPoint(event: PointerEvent, fromFrame: boolean) {
      // Screen coordinates stay stable as the iframe moves under a captured pointer.
      return fromFrame ? { x: event.screenX, y: event.screenY } : { x: event.clientX, y: event.clientY };
    }

    function bindPointerInput(target: EventTarget, fromFrame: boolean) {
      const options = { capture: true, passive: false };
      bind(target, 'wheel', (raw) => {
        if (!preview.current) return;
        const event = raw as WheelEvent;
        event.preventDefault();
        if (gesture.current !== undefined || pan.current) return;
        const units = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? canvasStage!.clientHeight : 1;
        if (event.ctrlKey || event.metaKey) {
          zoom((percent) => percent * Math.exp(-event.deltaY * units * 0.01), anchor(event, fromFrame));
        } else {
          move(-event.deltaX * units, -event.deltaY * units);
        }
      }, options);

      bind(target, 'gesturestart', (event) => {
        if (!preview.current) return;
        event.preventDefault();
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
        if (!preview.current || pan.current) return;
        blockClick = false;
        if (event.button !== 1 && !(event.button === 0 && (heldSpace.current || event.target === canvasStage))) return;
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
  }, [stage, preview, frameReady, previewUrl]);

  return {
    viewport, scale: viewport.percent / 100 * BASE_SCALE, space, dragging, reset,
    setPercent: (percent: number) => zoom(() => percent),
  };
}
