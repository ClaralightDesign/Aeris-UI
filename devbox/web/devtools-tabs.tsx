import { ScrollArea } from '@claralight-design/react';
import { Button } from '@heroui/react';
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { Smooth } from './smooth';

export function DevToolsTabs<T extends string>({ label, tabs, value, onChange, extra }: {
  label: string;
  tabs: readonly T[];
  value: T;
  onChange: (tab: T) => void;
  extra?: (tab: T) => ReactNode;
}) {
  const list = useRef<HTMLDivElement>(null);
  const viewport = useRef<HTMLDivElement>(null);
  const [indicator, setIndicator] = useState({ left: 0, width: 0 });
  const [ready, setReady] = useState(false);

  useLayoutEffect(() => {
    const items = list.current;
    const scroller = viewport.current;
    const button = items?.querySelector<HTMLButtonElement>('.debug-pill[data-active=true]');
    if (!items || !scroller || !button) return;
    const update = () => {
      const rect = button.getBoundingClientRect();
      if (!rect.width) return;
      const left = rect.left - items.getBoundingClientRect().left;
      setIndicator((previous) => previous.left === left && previous.width === rect.width
        ? previous : { left, width: rect.width });
      if (scroller.scrollWidth <= scroller.clientWidth) return;
      const behavior = matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth';
      const right = left + rect.width;
      if (left < scroller.scrollLeft) scroller.scrollTo({ left: Math.max(0, left - 12), behavior });
      else if (right > scroller.scrollLeft + scroller.clientWidth) {
        scroller.scrollTo({ left: right - scroller.clientWidth + 12, behavior });
      }
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(items);
    observer.observe(scroller);
    observer.observe(button);
    return () => observer.disconnect();
  }, [value]);

  useEffect(() => {
    const timer = window.setTimeout(() => setReady(true), 80);
    return () => window.clearTimeout(timer);
  }, []);

  return <div className="debug-tabs" role="group" aria-label={label}>
    <ScrollArea orientation="horizontal" edge="mask" scrollbars="hidden"
      className="debug-tab-scroll" viewportClassName="debug-tab-viewport" viewportRef={viewport}>
      <div className="debug-tabs-inner" ref={list}>
        <Smooth radius="capsule"><span className="debug-tab-indicator" data-ready={ready} aria-hidden="true"
          style={{ transform: `translate(${indicator.left}px, -50%)`, width: indicator.width }} /></Smooth>
        {tabs.map((tab, index) => <span className="debug-tab" key={tab}>
          {index > 0 && <span className="debug-tab-divider" aria-hidden="true" />}
          <Smooth radius="capsule"><Button variant="ghost" className="debug-pill" aria-pressed={value === tab}
            data-active={value === tab} onPress={() => onChange(tab)}>{tab}{extra?.(tab)}</Button></Smooth>
        </span>)}
      </div>
    </ScrollArea>
  </div>;
}
