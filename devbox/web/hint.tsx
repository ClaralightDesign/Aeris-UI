import { Fragment, useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Smooth } from './smooth';

export function Hint({ label, keys, children, hidden }: { label: string; keys?: ReactNode; children: ReactNode; hidden?: boolean }) {
  const host = useRef<HTMLSpanElement>(null);
  const tooltip = useRef<HTMLSpanElement>(null);
  const shadowId = useId();
  const [open, setOpen] = useState(false);
  const [anchor, setAnchor] = useState({ top: 0, x: 0 });
  const [placement, setPlacement] = useState({ center: 0, arrowLeft: 0 });
  useEffect(() => {
    if (hidden) setOpen(false);
  }, [hidden]);
  function show() {
    if (hidden) return;
    const rect = host.current?.getBoundingClientRect();
    if (!rect) return;
    setAnchor({ top: rect.bottom + 6, x: rect.left + rect.width / 2 });
    setOpen(true);
  }
  useLayoutEffect(() => {
    const element = tooltip.current;
    if (!element) return;
    function place(width: number) {
      const inset = 8;
      const half = width / 2;
      const center = Math.max(half + inset, Math.min(document.documentElement.clientWidth - half - inset, anchor.x));
      const limit = Math.max(0, half - 11 - inset);
      const offset = Math.max(-limit, Math.min(limit, anchor.x - center));
      setPlacement({ center, arrowLeft: half - 11 + offset });
    }
    place(element.getBoundingClientRect().width);
    const observer = new ResizeObserver(() => place(element.getBoundingClientRect().width));
    observer.observe(element);
    return () => observer.disconnect();
  }, [anchor.x, open, hidden]);
  function hide() {
    setOpen(false);
  }
  useEffect(() => {
    if (!open) return;
    window.addEventListener('resize', hide);
    window.addEventListener('scroll', hide, true);
    return () => {
      window.removeEventListener('resize', hide);
      window.removeEventListener('scroll', hide, true);
    };
  }, [open]);
  const shapeId = shadowId + '-shape';
  const shape = { clipPath: `url("#${shapeId}")` };
  return <span className="debug-tooltip-host" ref={host}
    onMouseEnter={show} onMouseLeave={hide} onFocusCapture={show} onBlurCapture={hide}>
    {children}
    {open && !hidden && createPortal(
      <span ref={tooltip} className="debug-tooltip" style={{ top: anchor.top, left: placement.center }} role="tooltip">
        <svg className="debug-tooltip-filter" width="100%" height="100%" aria-hidden="true">
          <defs>
            <clipPath id={shapeId}>
              <rect y={7.75} width="100%" height={28.25} rx={8} />
              <path transform={`translate(${placement.arrowLeft} 0)`}
                d="M0 8c3 0 6-5 9-7.35a3.1 3.1 0 0 1 4 0C16 3 19 8 22 8v.25H0Z" />
            </clipPath>
            <filter id={shadowId} colorInterpolationFilters="sRGB">
              {/* Both parts of the silhouette share the same 25% white fill. */}
              <feComponentTransfer in="SourceAlpha" result="silhouette">
                <feFuncA type="linear" slope={4} />
              </feComponentTransfer>
              {([{ dy: -1, blur: 1, opacity: .1 }, { dy: 1, blur: 1, opacity: .3 }, { dy: 1, blur: 3, opacity: .1 }]).map(
                ({ dy, blur, opacity }, index) => <Fragment key={index}>
                  <feOffset in="silhouette" dy={dy} result="offset" />
                  <feGaussianBlur in="offset" stdDeviation={blur / 2} result="blurred" />
                  <feComposite in="silhouette" in2="blurred" operator="out" result="edge" />
                  <feFlood floodColor="white" floodOpacity={opacity} result="highlight" />
                  <feComposite in="highlight" in2="edge" operator="in" result={`shadow${index}`} />
                </Fragment>,
              )}
              <feMerge>
                <feMergeNode in="SourceGraphic" />
                <feMergeNode in="shadow2" />
                <feMergeNode in="shadow1" />
                <feMergeNode in="shadow0" />
              </feMerge>
            </filter>
          </defs>
        </svg>
        <span className="debug-tooltip-glass" aria-hidden="true" style={shape} />
        <span className="debug-tooltip-surface" aria-hidden="true" style={{ filter: `url("#${shadowId}")` }}>
          <span className="debug-tooltip-fill" style={shape} />
        </span>
        <Smooth radius={8}><span className="debug-tooltip-body" data-shortcut={!!keys}>
          <span className="debug-tooltip-label">{label}</span>
          {keys && <span className="debug-tooltip-keys">{keys}</span>}
        </span></Smooth>
      </span>,
      document.body,
    )}
  </span>;
}
