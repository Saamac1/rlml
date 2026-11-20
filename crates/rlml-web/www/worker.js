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

import init, { WebGame } from './pkg/rlml_web.js';

let game = null;
let timer = null;
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
  } catch (e) {
    return fail(e);
  }
  show();
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
      return show();
    }
    later(game.has_events() ? 0 : game.wait_ms());
  } catch (e) {
    fail(e);
  }
}

onmessage = async (e) => {
  const m = e.data;
  if (m.t === 'load') {
    try {
      await init();
      game = new WebGame(JSON.parse(m.bundle), m.seed, m.cols, m.rows);
      if (size) game.resize(size.cols, size.rows);
    } catch (err) {
      postMessage({ t: 'error', text: `Pelin lataus ei onnistunut:\n\n${err && err.message ? err.message : err}` });
      return;
    }
    postMessage({ t: 'ready', keys: game.bound_keys() });
    show();
  } else if (m.t === 'resize' && !game) {
    size = m;
  } else if (game) {
    switch (m.t) {
      case 'key': game.key(m.name); break;
      case 'press': game.press(m.name, m.x, m.y); break;
      case 'move': game.pointer(m.x, m.y); break;
      case 'resize': game.resize(m.cols, m.rows); break;
      case 'pad': for (const words of m.events) game.pad(words); break;
      default: return;
    }
    if (!showing) later(0);
  }
};
