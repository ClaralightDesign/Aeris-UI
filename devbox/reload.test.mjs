import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';

const script = readFileSync(new URL('./native/reload.js', import.meta.url), 'utf8');

function storageFixture() {
  const values = new Map([['preview:opens', '3']]);
  let writes = 0;
  return {
    get length() { return values.size; },
    get writes() { return writes; },
    key: (i) => [...values.keys()][i] ?? null,
    getItem: (key) => values.get(key) ?? null,
    setItem(key, value) { writes++; values.set(key, value); },
    removeItem(key) { writes++; values.delete(key); },
  };
}

function preview(storage, { stage = false, offset = 0, target = 'Open' } = {}) {
  let time = 0;
  let details = false;
  let position = 0;
  let started = false;
  const create = runInNewContext(script + '\ncreatePreviewReload', {
    localStorage: storage, structuredClone,
    performance: { now: () => time },
    requestAnimationFrame(callback) { time += 16; queueMicrotask(callback); },
    setTimeout(callback, ms) { time += ms; queueMicrotask(callback); },
  });
  function inspect() {
    return [
      { id: 1, parent: 0, className: 'lv_screen', width: 336, height: 480 },
      { id: 2, parent: 1, kind: 1, component: 'AerisUI.HeaderTitle', value: details ? 'Details' : 'Home', width: 160, height: 30 },
      details
        ? { id: 4, parent: 1, component: 'AerisUI.NumericWheelColumn', label: 'Day', x: 30 + offset, y: 120,
          width: 80, height: 60, state: position, scroll: [0, position] }
        : { id: 3, parent: 1, component: 'AerisUI.JumpCard', label: target, x: 30 + offset, y: 120,
          width: 80, height: 60, clickable: true },
    ];
  }
  function hit(x, y) {
    return x >= 30 + offset && x < 110 + offset && y >= 120 && y < 180 ? (details ? 4 : 3) : 0;
  }
  const reload = create({
    stage, storage: 'preview:', inspect, hit, scrolling: () => false,
    contains: (x, y) => x >= 0 && x < 336 && y >= 0 && y < 480,
    input: {
      pointer(x, y, phase) {
        if (phase === 2 && hit(x, y) === 3) {
          details = true;
          const count = Number(reload.storageAccess.getItem('opens'));
          reload.storageAccess.setItem('opens', String(count + 1));
        }
      },
      wheel(x, y, dx, dy) { if (hit(x, y) === 4) position += dy; },
      back() { details = false; },
    },
    boot() { started = true; }, ready: () => started, failed: () => false,
    scrollTo(id, x, y) { if (id === 4) position = y; },
  });
  return { ...reload, idle: () => { time += 400; }, inspect };
}

function visit(source) {
  source.start();
  source.pointer(50, 140, 0);
  source.pointer(50, 140, 2);
  source.wheel(50, 140, 0, 35);
  source.idle();
  return source.api.capture();
}

test('replays relative to moved controls and verifies state before switching storage', async () => {
  const storage = storageFixture();
  const saved = visit(preview(storage));
  const candidate = preview(storage, { stage: true, offset: 110 });
  await candidate.api.restore(saved, () => {});
  assert.equal(candidate.inspect()[1].value, 'Details');
  assert.equal(candidate.inspect()[2].state, 35);
  assert.equal(storage.writes, 1);
  assert.equal(candidate.api.activate(saved), true);
  assert.equal(storage.getItem('preview:opens'), '4');
  assert.equal(storage.writes, 1);
  assert.deepEqual(candidate.api.capture().actions, saved.actions);
});

test('does not activate when a target changed or live storage diverged', async () => {
  const storage = storageFixture();
  const saved = visit(preview(storage));
  const missing = preview(storage, { stage: true, target: 'Different action' });
  await assert.rejects(missing.api.restore(saved, () => {}), /无法唯一定位/);
  assert.equal(storage.writes, 1);

  const candidate = preview(storage, { stage: true });
  await candidate.api.restore(saved, () => {});
  storage.setItem('preview:opens', '5');
  assert.throws(() => candidate.api.activate(saved), /存储与原预览不一致/);
  assert.equal(storage.getItem('preview:opens'), '5');
  assert.equal(storage.writes, 2);
});
