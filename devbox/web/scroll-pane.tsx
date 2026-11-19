import { ScrollAreaContent, ScrollAreaRoot, ScrollAreaScrollbar, ScrollAreaThumb, ScrollAreaViewport } from '@claralight-design/react';
import type { ComponentProps, ReactNode } from 'react';

type ViewportProps = ComponentProps<typeof ScrollAreaViewport>;

export function ScrollPane({ className, contentClassName, viewport, orientation = 'vertical', edge = 'none', showBars = true, children }: {
  className?: string;
  contentClassName?: string;
  viewport?: ViewportProps;
  orientation?: 'vertical' | 'horizontal' | 'both';
  edge?: 'none' | 'mask';
  showBars?: boolean;
  children: ReactNode;
}) {
  const { className: viewportClass, style, ...viewportProps } = viewport ?? {};
  const vertical = orientation !== 'horizontal';
  const horizontal = orientation !== 'vertical';
  const viewportClasses = ['debug-scroll-viewport', edge === 'mask' && 'cl-scroll-viewport', viewportClass].filter(Boolean).join(' ');
  return <ScrollAreaRoot data-cl-slot="scroll-area" className={'cl-scroll-area debug-scroll-area' + (className ? ' ' + className : '')}>
    <ScrollAreaViewport {...viewportProps} data-cl-slot="scroll-area-viewport"
      className={viewportClasses}
      style={{ ...(vertical ? {} : { overflowY: 'hidden' }), ...(horizontal ? {} : { overflowX: 'hidden' }), ...style }}>
      <ScrollAreaContent data-cl-slot="scroll-area-content" className={contentClassName}
        style={{ minWidth: horizontal ? 'max-content' : 0 }}>{children}</ScrollAreaContent>
    </ScrollAreaViewport>
    {showBars && vertical && <ScrollAreaScrollbar orientation="vertical" data-cl-slot="scroll-area-scrollbar" className="cl-scrollbar">
      <ScrollAreaThumb data-cl-slot="scroll-area-thumb" className="cl-scroll-thumb" />
    </ScrollAreaScrollbar>}
    {showBars && horizontal && <ScrollAreaScrollbar orientation="horizontal" data-cl-slot="scroll-area-scrollbar" className="cl-scrollbar">
      <ScrollAreaThumb data-cl-slot="scroll-area-thumb" className="cl-scroll-thumb" />
    </ScrollAreaScrollbar>}
  </ScrollAreaRoot>;
}
