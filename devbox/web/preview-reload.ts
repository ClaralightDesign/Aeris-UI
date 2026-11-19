import { useEffect, useRef, useState } from 'react';
import { flushSync } from 'react-dom';

type SavedPreview = { revision: number; state: unknown };
type ReloadRuntime = {
  capture(): SavedPreview | null;
  restore(saved: SavedPreview, check: () => void): Promise<void>;
  activate(saved: SavedPreview): boolean;
  revision(): number;
  failed(): boolean;
};

export type PreviewFrame = {
  key: number;
  url: string;
  scene: string;
  build: number | null;
};

type PreviewSlot = {
  active: PreviewFrame;
  pending?: PreviewFrame;
  error?: string;
};

type OpenPreview = { device: string; scene: string; build: number | null; enabled: boolean };

const timeout = 14_000;
const delay = (ms: number) => new Promise<void>((done) => window.setTimeout(done, ms));

function runtime(frame?: HTMLIFrameElement) {
  return (frame?.contentWindow as (Window & { aerisReload?: ReloadRuntime }) | null)?.aerisReload;
}

export function usePreviewReload(onCommit: (device: string, frame: HTMLIFrameElement) => void) {
  const [slots, setSlots] = useState<Record<string, PreviewSlot>>({});
  const current = useRef(slots);
  const nextKey = useRef(0);
  const frames = useRef(new Map<number, HTMLIFrameElement>());
  const publication = useRef<number | null>(null);
  const commit = useRef(onCommit);
  commit.current = onCommit;

  useEffect(() => () => { current.current = {}; }, []);

  function publish(value: Record<string, PreviewSlot>) {
    current.current = value;
    setSlots(value);
  }

  function update(device: string, slot: PreviewSlot) {
    publish({ ...current.current, [device]: slot });
  }

  function frameFor({ device, scene, build }: OpenPreview, staged = false): PreviewFrame {
    const key = ++nextKey.current;
    const params = new URLSearchParams({ scene, device, debug: '1', reloadable: '1', revision: String(key) });
    if (staged) {
      params.set('staged', '1');
      params.set('workbenchBuild', String(build));
    }
    return { key, url: '/preview.html?' + params, scene, build };
  }

  function open(options: OpenPreview) {
    update(options.device, { active: frameFor(options) });
  }

  function sync(options: OpenPreview[]) {
    const next: Record<string, PreviewSlot> = {};
    for (const entry of options) {
      const old = current.current[entry.device];
      next[entry.device] = old
        ? entry.enabled ? old : old.pending || old.error ? { active: old.active } : old
        : { active: frameFor(entry) };
    }
    publish(next);
  }

  function setPublication(build: number | null) {
    publication.current = build;
    const next = { ...current.current };
    let changed = false;
    for (const [device, slot] of Object.entries(next)) {
      if (slot.pending && slot.pending.build !== build) {
        next[device] = { active: slot.active };
        changed = true;
      }
    }
    if (changed) publish(next);
  }

  function reload(options: OpenPreview, force = false) {
    const { device, build, enabled } = options;
    const slot = current.current[device];
    if (!slot) return;
    if (!enabled || build === null) {
      if (force) open(options);
      return;
    }
    if (build !== publication.current || slot.pending?.build === build || (!force && slot.active.build === build)) return;
    const pending = frameFor({ ...options, scene: slot.active.scene }, true);
    update(device, { active: slot.active, pending });
    void restore(options, slot.active, pending);
  }

  async function restore(options: OpenPreview, active: PreviewFrame, pending: PreviewFrame) {
    const { device, build } = options;
    const superseded = new Error('预览已更换');
    const interaction = new Error('原预览有新的操作');
    const end = performance.now() + timeout;
    let saved: SavedPreview | null = null;
    let source: ReloadRuntime | undefined;
    let watcher: number | undefined;
    const isCurrent = () => current.current[device]?.pending === pending && publication.current === build;
    const check = () => {
      if (!isCurrent()) throw superseded;
      if (saved && source?.revision() !== saved.revision) throw interaction;
      if (performance.now() > end) throw new Error('恢复预览超时');
    };

    try {
      let candidate;
      let candidateFrame;
      for (;;) {
        check();
        const activeFrame = frames.current.get(active.key);
        candidateFrame = frames.current.get(pending.key);
        source = runtime(activeFrame);
        candidate = runtime(candidateFrame);
        if (source && candidate && candidateFrame) break;
        if (activeFrame?.contentDocument?.querySelector('canvas[data-ready]') && !source) {
          throw new Error('当前预览没有可用于恢复的操作记录');
        }
        if (candidateFrame?.dataset.loaded === 'true' && !candidate
            && !candidateFrame.contentDocument?.getElementById('gallery')) {
          throw new Error('新版预览无法加载');
        }
        await delay(40);
      }
      while (!(saved = source.capture())) {
        check();
        if (source.failed()) throw new Error('当前预览已停止运行');
        await delay(50);
      }
      await Promise.race([
        candidate.restore(saved, check),
        new Promise<never>((_, reject) => {
          watcher = window.setInterval(() => {
            try { check(); } catch (error) { reject(error); }
          }, 40);
        }),
      ]);
      check();
      const latest = source.capture();
      if (!latest || JSON.stringify(latest.state) !== JSON.stringify(saved.state)) {
        throw new Error('恢复期间原预览的页面状态已改变');
      }
      const wasFocused = document.activeElement === frames.current.get(active.key);
      if (!candidate.activate(saved)) throw new Error('新版的页面状态已改变');
      flushSync(() => {
        update(device, { active: pending });
        commit.current(device, candidateFrame);
      });
      if (wasFocused) candidateFrame.contentWindow?.focus();
    } catch (error) {
      if (!isCurrent()) return;
      if (error === interaction) {
        update(device, { active });
        await delay(100);
        const slot = current.current[device];
        if (slot?.active === active && !slot.pending) reload(options, true);
      } else {
        const message = (error as { message?: string } | undefined)?.message;
        update(device, { active, error: message || '无法恢复预览' });
      }
    } finally {
      window.clearInterval(watcher);
    }
  }

  return { slots, frames, open, sync, setPublication, reload };
}
