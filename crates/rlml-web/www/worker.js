// The game in a worker: the terminal loop of rlml-tui's term.rs with
// timers in place of waiting (game.rs).
//
//   1. in-between frames, one each frame_ms, then the frame of now;
//   2. wait: wait_ms, or until an event comes (a key, the mouse, a new
//      size, a pad);
//   3. the events that came, then a tick; back to 1.
//
// Time is performance.now(). Each frame goes to the page as its changed
// cells (cells.rs).
//
// Saves and the profile live in the origin private file system (OPFS), in
// rlml/GAME/ as the data folder (D 3.2): saves/auto-1, profile. The game
// keeps them in memory (store.rs); the folder is read before it starts,
// and after each step its changes are written in the background, one at a
// time and in order. Without OPFS the saves last as long as the page.

import init, { WebGame } from './pkg/rlml_web.js';

let game = null;
let timer = null;
// the game's data folder in OPFS, or null without OPFS
let folder = null;
// the writes so far, in order
let writing = Promise.resolve();
// in-between frames are showing: events wait for them to end
let showing = false;
// a new size that came while the game was loading
let size = null;

const SAMPLES = 300;
const times = { draw: [], input: [], tick: [] };
let lastStats = 0;

function record(name, t) {
  const v = times[name];
  v.push(t);
  if (v.length > SAMPLES) v.shift();
}

function summary(v) {
  if (v.length === 0) return { n: 0 };
  const s = [...v].sort((a, b) => a - b);
  return {
    n: s.length,
    mean: s.reduce((a, b) => a + b, 0) / s.length,
    p95: s[Math.floor((s.length - 1) * 0.95)],
    max: s[s.length - 1],
  };
}

function sendStats() {
  const now = performance.now();
  if (now - lastStats < 1000) return;
  lastStats = now;
  postMessage({ t: 'stats', draw: summary(times.draw), input: summary(times.input), tick: summary(times.tick) });
}

function fail(e) {
  clearTimeout(timer);
  timer = null;
  game = null;
  postMessage({ t: 'error', text: `Ajonaikainen virhe: ${e && e.message ? e.message : e}` });
}

function send(cells) {
  postMessage({ t: 'cells', cells: cells.buffer }, [cells.buffer]);
}

function later(ms) {
  clearTimeout(timer);
  timer = setTimeout(step, ms);
}

// 3: the events, then a tick
function step() {
  timer = null;
  if (!game) return;
  try {
    const t0 = performance.now();
    game.input(t0);
    const t1 = performance.now();
    game.tick(t1);
    const t2 = performance.now();
    record('input', t1 - t0);
    record('tick', t2 - t1);
    persist();
    tellField();
  } catch (e) {
    return fail(e);
  }
  show();
}

// a scene's field opens or closes: the page brings up the phone's
// keyboard for it
let field = '';
function tellField() {
  const f = game.field();
  if (f !== field) {
    field = f;
    postMessage({ t: 'field', kind: f });
  }
}

// the game's folder: rlml/GAME in OPFS, or null where there is none
async function openFolder(name) {
  try {
    const root = await navigator.storage.getDirectory();
    const rlml = await root.getDirectoryHandle('rlml', { create: true });
    return await rlml.getDirectoryHandle(name || 'game', { create: true });
  } catch (e) {
    return null;
  }
}

// every file under dir as [path, text]; a .tmp file is a write that broke off
async function readFolder(dir, prefix, out) {
  for await (const [name, h] of dir.entries()) {
    if (h.kind === 'directory') {
      await readFolder(h, `${prefix}${name}/`, out);
    } else if (!name.endsWith('.tmp') && !name.endsWith('.crswap')) {
      out.push([`${prefix}${name}`, await (await h.getFile()).text()]);
    }
  }
  return out;
}

async function writeChange(path, text) {
  const parts = path.split('/');
  let dir = folder;
  for (const p of parts.slice(0, -1)) dir = await dir.getDirectoryHandle(p, { create: true });
  const name = parts[parts.length - 1];
  if (text === null) {
    try { await dir.removeEntry(name); } catch (e) { /* already gone */ }
    return;
  }
  const file = await dir.getFileHandle(name, { create: true });
  // the file takes its new text when the stream closes
  const w = await file.createWritable();
  await w.write(text);
  await w.close();
}

// the game's changes to its folder, written after the ones before
function persist() {
  if (!game) return;
  const changes = game.store_changes();
  if (!folder || changes.length === 0) return;
  writing = writing
    .then(async () => {
      for (const [path, text] of changes) await writeChange(path, text);
    })
    // the game goes on; the saves it made stay in memory
    .catch((e) => console.warn('rlml: a save was not written to OPFS:', e));
}

// 1 and 2
function show() {
  if (!game) return;
  try {
    const f = game.next_frame();
    if (f !== undefined) {
      showing = true;
      send(f);
      clearTimeout(timer);
      timer = setTimeout(show, game.frame_ms());
      return;
    }
    showing = false;
    const t0 = performance.now();
    const cells = game.draw();
    record('draw', performance.now() - t0);
    send(cells);
    sendStats();
    if (game.quit()) {
      // a tab cannot be closed: a new game starts
      game.restart();
      persist();
      return show();
    }
    later(game.has_events() ? 0 : game.wait_ms());
  } catch (e) {
    fail(e);
  }
}

// the first font size, largest first, whose window in cells holds the
// game's layout, or the smallest; the page measures its cells by it
function resize(m) {
  const list = m.fonts && m.fonts.length ? m.fonts : [{ css: 0, cols: m.cols, rows: m.rows }];
  const f = list.find((f) => game.fits(f.cols, f.rows, m.pw, m.ph)) || list[list.length - 1];
  if (f.css) postMessage({ t: 'font', css: f.css, cols: f.cols, rows: f.rows });
  game.resize(f.cols, f.rows, m.pw, m.ph);
}

onmessage = async (e) => {
  const m = e.data;
  if (m.t === 'load') {
    try {
      await init();
      game = new WebGame(JSON.parse(m.bundle), m.seed, m.cols, m.rows);
      folder = await openFolder(game.game_name());
      game.storage(folder ? await readFolder(folder, '', []) : []);
      resize(size || m);
    } catch (err) {
      postMessage({ t: 'error', text: `Pelin lataus ei onnistunut:\n\n${err && err.message ? err.message : err}` });
      return;
    }
    postMessage({ t: 'ready', keys: game.bound_keys() });
    tellField();
    show();
  } else if (m.t === 'resize' && !game) {
    size = m;
  } else if (game) {
    switch (m.t) {
      case 'key': game.key(m.name); break;
      case 'press': game.press(m.name, m.x, m.y); break;
      case 'move': game.pointer(m.x, m.y); break;
      case 'resize': resize(m); break;
      case 'release': game.release(); break;
      case 'pad': for (const words of m.events) game.pad(words); break;
      default: return;
    }
    if (!showing) later(0);
  }
};
