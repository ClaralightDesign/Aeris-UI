import { RecordIcon } from '@phosphor-icons/react/dist/csr/Record';
import { StopIcon } from '@phosphor-icons/react/dist/csr/Stop';
import { Button } from '@heroui/react';
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { type PreviewPerformance, type RenderSample } from './preview';
import { ScrollPane } from './scroll-pane';

const recordingLimit = 60_000;
const now = () => performance.timeOrigin + performance.now();
const localTime = new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
const ms = (value: number) => value.toFixed(2) + ' ms';
const seconds = (value: number) => (value / 1000).toFixed(2) + ' s';

type Recording = {
  name: string;
  startedAt: number;
  samples: RenderSample[];
};
type FinishedRecording = Recording & { endedAt: number; reason?: 'limit' | 'preview' | 'error' };
type LiveMetrics = NonNullable<ReturnType<PreviewPerformance['read']>>;

function RecordingTimeline({ result }: { result: FinishedRecording }) {
  const [index, setIndex] = useState(0);
  const { samples, startedAt, endedAt } = result;
  const sample = samples[index];
  const duration = endedAt - startedAt;
  const chart = useMemo(() => {
    const max = samples.reduce((peak, item) => Math.max(peak, item.durationMs), 0);
    const x = (item: RenderSample) => (item.at - startedAt) / (duration || 1) * 1000;
    const y = (item: RenderSample) => 92 - item.durationMs / (max || 1) * 84;
    const strokes = samples.map((item) => `M${x(item)} ${y(item)}V92`).join('');
    return { max, x, y, strokes };
  }, [samples, startedAt, duration]);
  return <section className="debug-performance-section debug-performance-result" aria-label="性能录制结果">
    <header><h2>性能录制</h2><span>{result.name} · {seconds(duration)}</span></header>
    {result.reason && <p className="debug-performance-note">{{
      limit: '录制已达到 60 秒上限',
      preview: '预览已切换，录制已结束',
      error: '预览运行失败，录制已结束',
    }[result.reason]}</p>}
    {sample ? <>
      <div className="debug-performance-plot">
        <div className="debug-performance-axis"><span>{chart.max > 0 ? ms(chart.max) : '耗时'}</span><span>0 ms</span></div>
        <svg viewBox="-3 0 1006 100" preserveAspectRatio="none" aria-hidden="true" onClick={(event) => {
          const rect = event.currentTarget.getBoundingClientRect();
          const position = (event.clientX - rect.left) / rect.width * 1006 - 3;
          const nearest = samples.reduce((closest, item, next) =>
            Math.abs(chart.x(item) - position) < Math.abs(chart.x(samples[closest]) - position) ? next : closest, 0);
          setIndex(nearest);
        }}>
          <path className="debug-performance-grid" d="M0 8H1000M0 50H1000M0 92H1000" />
          <path className="debug-performance-samples" d={chart.strokes} />
          <path className="debug-performance-marker" d={`M${chart.x(sample)} 5V96`} />
          <path className="debug-performance-point" d={`M${chart.x(sample)} ${chart.y(sample)}h.01`} />
        </svg>
        <div className="debug-performance-time"><span>0 s</span><span>{seconds(duration)}</span></div>
      </div>
      <label className="debug-performance-scrubber">
        <span>绘制样本</span>
        <input type="range" min={0} max={samples.length - 1} step={1} value={index} disabled={samples.length < 2}
          aria-valuetext={`第 ${index + 1} 次，${seconds(sample.at - startedAt)}，耗时 ${ms(sample.durationMs)}`}
          onChange={(event) => setIndex(Number(event.currentTarget.value))} />
        <span>{index + 1} / {samples.length}</span>
      </label>
      <dl className="debug-performance-sample-detail">
        <div><dt>录制时间点</dt><dd>{seconds(sample.at - startedAt)}</dd></div>
        <div><dt>绘制及提交耗时</dt><dd>{ms(sample.durationMs)}</dd></div>
        <div><dt>Canvas 提交次数</dt><dd>{sample.submissions}</dd></div>
      </dl>
    </> : <p className="debug-performance-note">录制期间没有发生绘制。</p>}
  </section>;
}

export function PerformancePanel({ source, target, name, hidden }: {
  source?: PreviewPerformance;
  target: string;
  name: string;
  hidden: boolean;
}) {
  const [live, setLive] = useState<{ source: PreviewPerformance; target: string; metrics: LiveMetrics | null }>();
  const [recording, setRecording] = useState(false);
  const [result, setResult] = useState<FinishedRecording>();
  const current = useRef<Recording>(undefined);
  const timer = useRef<number>(undefined);

  const finish = useCallback((reason?: FinishedRecording['reason'], at = now()) => {
    const active = current.current;
    if (!active) return;
    current.current = undefined;
    window.clearTimeout(timer.current);
    setRecording(false);
    setResult({ ...active, endedAt: Math.min(at, active.startedAt + recordingLimit), reason });
  }, []);

  useLayoutEffect(() => {
    const unsubscribe = source?.subscribe((sample) => {
      if (!sample) {
        setLive(undefined);
        finish('error');
        return;
      }
      const active = current.current;
      if (!active || sample.at < active.startedAt) return;
      const deadline = active.startedAt + recordingLimit;
      if (sample.at <= deadline) active.samples.push(sample);
      if (sample.at >= deadline) finish('limit', deadline);
    });
    return () => {
      unsubscribe?.();
      finish('preview');
    };
  }, [source, target, finish]);

  useEffect(() => {
    if (hidden || !source) return;
    const sample = () => {
      const metrics = source.read();
      setLive((previous) => previous?.source === source && previous.target === target
        && previous.metrics?.lastRender === metrics?.lastRender
        && previous.metrics?.memoryBytes === metrics?.memoryBytes ? previous : { source, target, metrics });
    };
    sample();
    const interval = window.setInterval(sample, 250);
    return () => window.clearInterval(interval);
  }, [source, target, hidden]);

  const metrics = live?.source === source && live?.target === target ? live?.metrics : undefined;
  function start() {
    if (current.current || !source?.read()) return;
    const startedAt = now();
    current.current = { name, startedAt, samples: [] };
    setResult(undefined);
    setRecording(true);
    timer.current = window.setTimeout(() => finish('limit', startedAt + recordingLimit), recordingLimit);
  }

  return <div className="debug-performance" hidden={hidden}>
    <div className="debug-panel-toolbar debug-performance-toolbar">
      <Button variant="ghost" className="debug-performance-record" isDisabled={!recording && !metrics}
        onPress={recording ? () => finish() : start}>
        {recording ? <StopIcon size={16} aria-hidden="true" /> : <RecordIcon size={16} aria-hidden="true" />}
        {recording ? '停止录制' : '开始录制'}
      </Button>
      {recording && <span role="status" className="debug-performance-recording">正在录制</span>}
    </div>
    <ScrollPane className="debug-panel-surface debug-performance-body" contentClassName="debug-performance-content"
      viewport={{ role: 'region', 'aria-label': '性能内容' }}>
      <section className="debug-performance-section" aria-label="实时性能指标">
        <header><h2>实时指标</h2>{metrics && <span>最近绘制 <time dateTime={new Date(metrics.lastRender.at).toISOString()}>
          {localTime.format(metrics.lastRender.at)}
        </time></span>}</header>
        <dl className="debug-performance-metrics">
          <div><dt>LVGL 绘制及提交耗时</dt><dd data-metric="duration">{metrics ? ms(metrics.lastRender.durationMs) : '—'}</dd></div>
          <div><dt>Canvas 提交次数</dt><dd data-metric="submissions">{metrics?.submissions ?? '—'}</dd></div>
          <div><dt>WASM 线性内存容量</dt><dd data-metric="memory">{metrics ? (metrics.memoryBytes / 1048576).toFixed(2) + ' MiB' : '—'}</dd>
            <dd className="debug-performance-note">WASM 已分配的线性内存</dd></div>
        </dl>
        {!metrics && <p className="debug-performance-note">当前预览暂无性能数据。</p>}
      </section>
      {result && <RecordingTimeline key={result.startedAt} result={result} />}
    </ScrollPane>
  </div>;
}
