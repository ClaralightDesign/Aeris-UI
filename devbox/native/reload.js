// Included in the DevBox-served preview. Exported component previews do not journal input.
function createPreviewReload({ stage, storage, contains, inspect, hit, scrolling, scrollTo, input, boot, ready, failed }) {
  const maxActions = 160;
  const maxEvents = 2400;
  const maxStorage = 4 * 1024 * 1024;
  const actions = [];
  let base;
  let shadow;
  let pending;
  let revision = 0;
  let eventCount = 0;
  let lastInput = -Infinity;
  let invalid = false;
  let live = !stage;
  let started = false;

  function readStorage() {
    const values = new Map();
    let size = 0;
    for (let index = 0; index < localStorage.length; index++) {
      const key = localStorage.key(index);
      if (!key?.startsWith(storage)) continue;
      const value = localStorage.getItem(key);
      if (value === null) continue;
      size += key.length + value.length;
      if (size > maxStorage) throw new Error("应用存储超过恢复上限");
      values.set(key.slice(storage.length), value);
    }
    return values;
  }

  const storageAccess = {
    getItem(key) { return live ? localStorage.getItem(storage + key) : shadow.get(key) ?? null; },
    setItem(key, value) {
      if (live) localStorage.setItem(storage + key, value);
      else shadow.set(key, value);
    },
    removeItem(key) {
      if (live) localStorage.removeItem(storage + key);
      else shadow.delete(key);
    },
  };

  function launch() {
    if (started) return;
    started = true;
    boot();
  }

  function start() {
    if (!live || started) return;
    try { base = [...readStorage()]; }
    catch { invalid = true; }
    launch();
  }

  function tree() {
    const nodes = inspect().filter((node) => !node.hidden && node.width > 0 && node.height > 0);
    const byId = new Map(nodes.map((node) => [node.id, node]));
    const children = new Map();
    for (const node of nodes) {
      const siblings = children.get(node.parent);
      if (siblings) siblings.push(node);
      else children.set(node.parent, [node]);
    }
    return { nodes, byId, children, moving: nodes.some((node) => node.moving) };
  }

  function onScreen(node, current) {
    const root = current.children.get(0)?.[0];
    return root && node.x < root.width && node.x + node.width > 0 && node.y < root.height && node.y + node.height > 0;
  }

  function label(value) {
    return value?.trim().replace(/\s+/g, " ").slice(0, 120) ?? "";
  }

  function staticLabel(value) {
    const text = label(value);
    return /^\d{1,3}:\d\d(?::\d\d)?(?:[.,]\d+)?$/.test(text) ? "" : text;
  }

  function firstLabel(node, current) {
    if (node.kind === 1) return staticLabel(node.value);
    for (const child of current.children.get(node.id) ?? []) {
      if (child.kind !== 1 && (child.clickable || child.scroll)) continue;
      const text = firstLabel(child, current);
      if (text) return text;
    }
    return "";
  }

  function identity(node, current) {
    const type = node.component || node.className;
    const text = node.label || (node.kind === 1 || type === "AerisUI.Page"
        || (!node.scroll && (node.clickable || node.state !== undefined)) ? firstLabel(node, current) : "");
    return [type, text];
  }

  function sameIdentity(left, right) {
    return left[0] === right[0] && left[1] === right[1];
  }

  function siblingOrder(node, current, name) {
    if (!name[1] || !name[0]?.startsWith("AerisUI.")) return undefined;
    const siblings = current.children.get(node.parent);
    const names = siblings.map((sibling) => identity(sibling, current));
    const peers = names.filter((nameAtIndex) => sameIdentity(nameAtIndex, name));
    if (peers.length < 2) return undefined;
    const index = siblings.indexOf(node);
    const before = names.slice(0, index);
    const after = names.slice(index + 1);
    // A named row can occur more than once. Its order is usable only while both
    // the number of those rows and their surrounding labels still agree.
    return JSON.stringify([
      before.filter((candidate) => sameIdentity(candidate, name)).length,
      peers.length,
      before.findLast((candidate) => !sameIdentity(candidate, name)),
      after.find((candidate) => !sameIdentity(candidate, name)),
    ]);
  }

  function pathTo(node, current) {
    const path = [];
    for (let item = node; item; item = current.byId.get(item.parent)) {
      if (item.parent && !current.byId.has(item.parent)) return null;
      const name = identity(item, current);
      const order = siblingOrder(item, current, name);
      path.unshift(order === undefined ? name : [...name, order]);
    }
    return find(path, current)?.id === node.id ? path : null;
  }

  function find(path, current) {
    let parents = [0];
    let matches;
    for (const [type, text, order] of path) {
      matches = parents.flatMap((id) => current.children.get(id) ?? []).filter((node) => {
        const name = identity(node, current);
        return sameIdentity([type, text], name)
          && (order === undefined || siblingOrder(node, current, name) === order);
      });
      if (!matches.length) return null;
      parents = matches.map(({ id }) => id);
    }
    return matches?.length === 1 ? matches[0] : null;
  }

  function screen(current) {
    let titles = current.nodes.filter(({ component }) =>
      component === "AerisUI.HeaderTitle" || component === "AerisUI.ShrinkingTitle").map(({ value }) => staticLabel(value)).filter(Boolean);
    const dialogs = current.nodes.filter(({ component }) =>
      component === "AerisUI.Dialog" || component === "AerisUI.ConfirmDialog")
      .map((node) => [node.component, firstLabel(node, current)]);
    if (!titles.length) {
      // Older app dependencies still expose ordinary LVGL labels and their tree.
      titles = current.nodes.filter(({ kind }) => kind === 1).map(({ value }) => staticLabel(value))
        .filter((text) => /\p{L}/u.test(text)).slice(0, 5);
    }
    return JSON.stringify([[...new Set(titles)].sort(), dialogs]);
  }

  function stateOf(current) {
    const states = [];
    for (const node of current.nodes) {
      const state = controlState(node, current);
      const stockToggle = node.className === "lv_checkbox" || node.className === "lv_switch";
      if (!node.scroll && state === undefined && !node.checked && !stockToggle) continue;
      const path = pathTo(node, current);
      if (!path) {
        if (onScreen(node, current) && (state !== undefined || node.checked || node.scroll?.some(Boolean))) {
          const [type, text] = identity(node, current);
          throw new Error("无法唯一识别「" + (text || type) + "」的状态");
        }
        continue;
      }
      states.push({ path, state, checked: node.checked, scroll: node.scroll });
    }
    const page = screen(current);
    return { page, states };
  }

  function controlState(node, current) {
    if (node.state !== undefined) return node.state;
    // Older Aeris UI builds draw the stock radio's inner dot only when selected.
    if (node.component === "AerisUI.Radio") return Number((current.children.get(node.id)?.length ?? 0) > 1);
  }

  function changed() {
    revision++;
    lastInput = performance.now();
  }

  function record(action) {
    if (invalid) return;
    if (!action || actions.length >= maxActions || (eventCount += action.events?.length ?? 1) > maxEvents) {
      invalid = true;
      return;
    }
    actions.push(action);
  }

  function anchor(x, y, dx = 0, dy = 0) {
    const id = hit(x, y, dx, dy);
    if (!id) return undefined;
    const current = tree();
    const node = current.byId.get(id);
    const path = node && pathTo(node, current);
    if (!path) return null;
    return { path, page: screen(current), node };
  }

  function relative(node, x, y) {
    return [(x + 0.5 - node.x) / node.width, (y + 0.5 - node.y) / node.height];
  }

  function pointer(x, y, phase) {
    if (live && !invalid) {
      changed();
      if (phase === 0) {
        const target = anchor(x, y);
        pending = { target, start: lastInput, events: [] };
      }
      if (pending?.target) {
        pending.events.push({ at: lastInput - pending.start, point: relative(pending.target.node, x, y), phase });
        if (pending.events.length > maxEvents) {
          invalid = true;
          pending = undefined;
        }
      }
      if (phase >= 2 && pending) {
        const { target, events } = pending;
        if (target === null) record(null);
        else if (target) record({ kind: "pointer", path: target.path, page: target.page, events });
        pending = undefined;
      }
    }
    input.pointer(x, y, phase);
  }

  function wheel(x, y, dx, dy) {
    if (live && !invalid) {
      const target = anchor(x, y, dx, dy);
      if (target !== undefined) changed();
      if (target === null) record(null);
      else if (target) {
        const point = relative(target.node, x, y);
        const previous = actions.at(-1);
        const event = { point, dx, dy, at: lastInput };
        if (previous?.kind === "wheel" && JSON.stringify(previous.path) === JSON.stringify(target.path)
            && lastInput - previous.events.at(-1).at < 180) {
          if (++eventCount > maxEvents) invalid = true;
          else previous.events.push(event);
        } else {
          record({ kind: "wheel", path: target.path, page: target.page, events: [event] });
        }
      }
    }
    input.wheel(x, y, dx, dy);
  }

  function back() {
    if (live && !invalid) {
      changed();
      record({ kind: "back", page: screen(tree()) });
    }
    input.back();
  }

  function capture() {
    if (invalid) throw new Error("操作记录超出范围或包含无法确认的控件");
    if (!ready() || pending || scrolling() || performance.now() - lastInput < 350) return null;
    const current = tree();
    if (current.moving) return null;
    const state = stateOf(current);
    return structuredClone({ base, actions, eventCount, revision, state });
  }

  function coordinates(node, [x, y]) {
    return [Math.floor(node.x + node.width * x), Math.floor(node.y + node.height * y)];
  }

  function verifyTarget(node, point, dx, dy) {
    if (!contains(...point) || hit(...point, dx, dy) !== node.id) throw new Error("新版中的操作目标已改变");
  }

  function matches(expected, current, adjust) {
    if (current.moving || screen(current) !== expected.page) return false;
    for (const entry of expected.states) {
      const node = find(entry.path, current);
      if (!node || node.checked !== entry.checked) return false;
      const differentScroll = entry.scroll && entry.scroll.some((value, i) => value !== (node.scroll?.[i] ?? 0));
      if (differentScroll && adjust && node.scroll && !node.snap) {
        scrollTo(node.id, ...entry.scroll);
        // List positioning can invoke callbacks. Re-read the tree on the next frame.
        return false;
      }
      if (differentScroll || (entry.state !== undefined && controlState(node, current) !== entry.state)) return false;
    }
    return true;
  }

  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const nextFrame = () => new Promise((resolve) => requestAnimationFrame(resolve));

  async function waitFor(test, check, ms, message) {
    const end = performance.now() + ms;
    do {
      check();
      if (failed()) throw new Error("新版应用启动失败");
      const result = test();
      if (result) return result;
      await nextFrame();
    } while (performance.now() < end);
    throw new Error(message);
  }

  async function restore(saved, check) {
    base = saved.base;
    shadow = new Map(base);
    launch();
    await waitFor(ready, check, 8000, "等待新版预览首帧超时");
    for (const action of saved.actions) {
      const current = await waitFor(() => {
        const current = tree();
        return !scrolling() && !current.moving && screen(current) === action.page && current;
      }, check, 900, "新版无法复现原页面路径");
      if (action.kind === "back") {
        input.back();
        await nextFrame();
        continue;
      }
      const node = find(action.path, current);
      if (!node) throw new Error("新版中的操作目标无法唯一定位");
      const start = performance.now();
      const first = action.events[0].at;
      for (const event of action.events) {
        const delay = event.at - first - (performance.now() - start);
        if (delay > 0) await sleep(Math.min(delay, 1200));
        check();
        const point = coordinates(node, event.point);
        if (action.kind === "wheel") {
          verifyTarget(node, point, event.dx, event.dy);
          input.wheel(...point, event.dx, event.dy);
        } else {
          if (!event.phase) verifyTarget(node, point, 0, 0);
          if (event.phase === 2 && !contains(...point)) throw new Error("新版中的手势已超出屏幕");
          input.pointer(...point, event.phase);
        }
      }
      await nextFrame();
    }
    await waitFor(() => !scrolling(), check, 1500, "等待滚动停止超时");
    await waitFor(() => matches(saved.state, tree(), true), check, 1200, "新版中的页面或控件状态与原预览不一致");
    await nextFrame();
    await waitFor(() => matches(saved.state, tree(), false), check, 800, "新版中的页面或控件状态与原预览不一致");
    actions.push(...saved.actions);
    eventCount = saved.eventCount;
  }

  function activate(saved) {
    if (live || failed() || !matches(saved.state, tree(), false)) return false;
    const current = readStorage();
    if (current.size !== shadow.size || [...current].some(([key, value]) => shadow.get(key) !== value)) {
      throw new Error("应用存储与原预览不一致");
    }
    shadow = undefined;
    live = true;
    return true;
  }

  const api = { capture, restore, activate, revision: () => revision, failed };
  return { start, storageAccess, pointer, wheel, back, api };
}
