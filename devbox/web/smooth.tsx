import { Slot, useSmoothCorners, type BorderConfig } from '@lisse/react';
import { useRef, type ReactElement } from 'react';

export function Smooth({ radius, stroke, children }: {
  radius: number | 'capsule';
  stroke?: BorderConfig;
  children: ReactElement;
}) {
  const element = useRef<HTMLElement>(null);
  useSmoothCorners(element, {
    // Lisse limits the radius to half the shorter side and preserves the smoothing.
    radius: radius === 'capsule' ? 999 : radius,
    smoothing: 0.6,
    preserveSmoothing: true,
  }, { autoEffects: false, effects: stroke ? { innerBorder: stroke } : undefined });
  return <Slot ref={element}>{children}</Slot>;
}

export const popoverStroke = { width: 1, color: '#ffffff', opacity: 0.19 };
