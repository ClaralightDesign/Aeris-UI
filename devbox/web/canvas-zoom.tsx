import { NumberInput } from '@claralight-design/react';
import { Button, Popover } from '@heroui/react';
import { MagnifyingGlassPlusIcon } from '@phosphor-icons/react/dist/csr/MagnifyingGlassPlus';
import { useEffect, useId, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import Scritto from '@scritto/react';
import { MAX_ZOOM, MIN_ZOOM } from './canvas-viewport';
import { Hint } from './hint';
import { Smooth, popoverStroke } from './smooth';

function ZoomPercentInput({ percent, onChange }: { percent: number; onChange: (percent: number) => void }) {
  const id = useId();
  function apply(value: number | null) {
    if (value !== null) onChange(Math.round(value * 10) / 10);
  }
  return <div className="debug-zoom-field" onKeyDown={(event) => {
    if (event.key === 'Enter' && event.target instanceof HTMLInputElement) {
      event.preventDefault();
      event.target.blur();
    }
  }}>
    <label className="debug-sr-only" htmlFor={id}>缩放百分比</label>
    <NumberInput id={id} value={Math.round(percent * 10) / 10} min={MIN_ZOOM} max={MAX_ZOOM} step={1}
      prefix={<span title="按住并左右拖动调整缩放" className="debug-zoom-scrub-label">缩放</span>}
      suffix="%" format={{ maximumFractionDigits: 1 }}
      onValueChange={(value, reason) => { if (reason === 'scrub') apply(value); }}
      onValueCommitted={apply} onCancel={apply} />
  </div>;
}

export function CanvasZoom({ viewport, fit, reset, zoomIn, zoomOut, setPercent, label = '缩放', scope, disabled, children }: {
  viewport: { percent: number; showZoomFeedback: boolean };
  fit: () => void;
  reset: () => void;
  zoomIn: () => void;
  zoomOut: () => void;
  setPercent: (percent: number) => void;
  label?: string;
  scope?: string;
  disabled?: boolean;
  children?: ReactNode;
}) {
  const { percent, showZoomFeedback } = viewport;
  const previous = useRef(percent);
  const [open, setOpen] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const value = `${percent.toFixed(1)}%`;
  const modifier = /Mac|iPhone|iPad|iPod/.test(navigator.userAgent) ? '⌘' : 'Ctrl ';
  useEffect(() => {
    const changed = percent !== previous.current;
    previous.current = percent;
    if (open || !showZoomFeedback) {
      setExpanded(false);
      return;
    }
    if (!changed) return;
    setExpanded(true);
    const timer = window.setTimeout(() => setExpanded(false), 1500);
    return () => window.clearTimeout(timer);
  }, [open, percent, showZoomFeedback]);

  return <Hint label={label} hidden={open || expanded || disabled}>
    <Popover isOpen={open} onOpenChange={setOpen}>
      <Smooth radius={6}><Button variant="ghost" className="debug-icon debug-zoom-trigger" aria-label={label}
        isDisabled={disabled} data-expanded={expanded} style={{ '--debug-zoom-digits': value.length - 2 } as CSSProperties}>
        <MagnifyingGlassPlusIcon size={18} aria-hidden="true" />
        <span className="debug-zoom-trigger-value" aria-hidden="true"><Scritto value={value} /></span>
      </Button></Smooth>
      <Popover.Content className="debug-popover-wrap"><Smooth radius={20} stroke={popoverStroke}>
        <Popover.Dialog aria-label={label} className="debug-popover debug-zoom-menu" data-zoom-scope={scope ?? ''}>
          <ZoomPercentInput percent={percent} onChange={setPercent} />
          <div className="debug-zoom-actions">
            {[
              { label: '放大', key: '+', action: zoomIn },
              { label: '缩小', key: '−', action: zoomOut },
              { label: '适应画布', action: fit },
              { label: '缩放至 50%', action: () => setPercent(50) },
              { label: '缩放至 100%', key: '0', action: reset },
              { label: '缩放至 200%', action: () => setPercent(200) },
            ].map(({ label, key, action }) => <Smooth key={label} radius={8}>
              <Button variant="ghost" className="debug-menu-item" onPress={() => { action(); setOpen(false); }}>
                <span>{label}</span>{key && <span className="debug-zoom-shortcut" aria-hidden="true">{modifier}{key}</span>}
              </Button>
            </Smooth>)}
          </div>
          {children}
        </Popover.Dialog>
      </Smooth></Popover.Content>
    </Popover>
  </Hint>;
}
