// The page: the game runs in a worker (worker.js), and the page draws
// the cells it sends on a canvas, as a terminal would. It sends the
// worker the keys and mouse buttons by their binding names (`k`,
// `ctrl+q`, `click`), the pointer's cell, the window's size in cells and
// the pads' raw events, which only the page can read.
//
// URL parameters: `seed=N` (default 1, as `rlml run`), `stats` shows the
// times of draw and the steps in the corner.

import init, { key_binding, mouse_binding, wheel_binding, Pads } from './pkg/rlml_web.js';

const params = new URLSearchParams(location.search);
const seed = Number(params.get('seed') ?? 1) >>> 0;
const message = document.getElementById('message');
const statsBox = document.getElementById('stats');
const canvas = document.getElementById('screen');
const ctx = canvas.getContext('2d', { alpha: false });

// a phone takes a larger font, so the map and the log read well on its
// small screen; a larger default font (the reader's own setting) makes it
// larger still. The font shrinks from this until the game's layout fits
// the window (`fits` in the worker), down to MIN_FONT_PX.
const ROOT_PX = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
const FONT_PX = Math.round((Math.min(window.innerWidth, window.innerHeight) < 520 ? 18 : 16) * Math.max(1, ROOT_PX / 16));
const MIN_FONT_PX = 7;
const LINE = 1.2;
const FAMILY = 'Menlo, Monaco, "SF Mono", "DejaVu Sans Mono", Consolas, "Liberation Mono", monospace';
// cells.rs: the flags over the character's 21 bits
const CHAR = (1 << 21) - 1, BOLD = 1 << 21, DIM = 1 << 22, ITALIC = 1 << 23, UNDERLINE = 1 << 24, WIDE = 1 << 25;
// one wheel step: a line, or this many pixels of a trackpad's scrolling
const WHEEL_PX = 40;

function show(text, error) {
  message.textContent = text;
  message.className = error ? 'error' : '';
  message.hidden = false;
}

// cell size in device pixels, from the font's own measures; measured
// again when the pixel ratio changes (zoom, another screen) or the worker
// picks another font size
let dpr, fontCss = FONT_PX, fontPx, cellW, cellH;
function font(flags) {
  return `${flags & ITALIC ? 'italic ' : ''}${flags & BOLD ? 'bold ' : ''}${fontPx}px ${FAMILY}`;
}
function cellOf(css) {
  const px = Math.round(css * dpr);
  ctx.font = `${px}px ${FAMILY}`;
  return [px, Math.ceil(ctx.measureText('M').width), Math.ceil(px * LINE)];
}
function measure() {
  dpr = window.devicePixelRatio || 1;
  [fontPx, cellW, cellH] = cellOf(fontCss);
  ctx.font = font(0);
}
// the page's own room: the body, which a host may pad (safe areas)
function room() {
  const b = document.body;
  return [b.clientWidth || window.innerWidth, b.clientHeight || window.innerHeight];
}
function gridOf(cw, ch) {
  const [w, h] = room();
  return [Math.max(20, Math.floor((w * dpr) / cw)), Math.max(10, Math.floor((h * dpr) / ch))];
}
const pixels = () => {
  const [w, h] = room();
  return { pw: Math.round(w * dpr), ph: Math.round(h * dpr) };
};
// the window in cells at each font size from the largest down: the
// worker takes the first whose grid holds the layout
function fonts() {
  dpr = window.devicePixelRatio || 1;
  const out = [];
  let last = '';
  for (let css = FONT_PX; css >= MIN_FONT_PX; css--) {
    const [, cw, ch] = cellOf(css);
    const [cols, rows] = gridOf(cw, ch);
    const key = `${cols}x${rows}`;
    if (key !== last) out.push({ css, cols, rows });
    last = key;
  }
  ctx.font = font(0);
  return out;
}
measure();
let [cols, rows] = gridOf(cellW, cellH);
let sent = '';

function size(w, h) {
  canvas.width = w * cellW;
  canvas.height = h * cellH;
  canvas.style.width = `${canvas.width / dpr}px`;
  canvas.style.height = `${canvas.height / dpr}px`;
}
size(cols, rows);

const hexes = new Map();
function hex(c) {
  let s = hexes.get(c);
  if (s === undefined) {
    s = '#' + c.toString(16).padStart(6, '0');
    hexes.set(c, s);
  }
  return s;
}

// cells.rs: [width, height, full, then per cell: index, char | flags, fg, bg]
function paint(buf) {
  const w = buf[0], h = buf[1];
  if (buf[2] || canvas.width !== w * cellW || canvas.height !== h * cellH) size(w, h);
  let lastFont = '';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let i = 3; i < buf.length; i += 4) {
    const idx = buf[i], cf = buf[i + 1], fg = buf[i + 2], bg = buf[i + 3];
    const x = (idx % w) * cellW, y = Math.floor(idx / w) * cellH;
    const span = cf & WIDE ? 2 : 1;
    ctx.fillStyle = hex(bg);
    ctx.fillRect(x, y, cellW * span, cellH);
    const ch = cf & CHAR;
    if (ch !== 0 && ch !== 32) {
      const f = font(cf);
      if (f !== lastFont) {
        ctx.font = f;
        lastFont = f;
      }
      ctx.fillStyle = hex(fg);
      ctx.globalAlpha = cf & DIM ? 0.55 : 1;
      ctx.fillText(String.fromCodePoint(ch), x + (cellW * span) / 2, y + cellH / 2, cellW * span);
      ctx.globalAlpha = 1;
    }
    if (cf & UNDERLINE) {
      ctx.fillStyle = hex(fg);
      ctx.fillRect(x, y + cellH - Math.max(1, Math.round(dpr * 1.5)), cellW * span, Math.max(1, Math.round(dpr)));
    }
  }
}

function statsText(s) {
  const row = (name, t) => `${name.padEnd(6)} ${t.n ? `mean ${t.mean.toFixed(2)}  p95 ${t.p95.toFixed(2)}  max ${t.max.toFixed(2)} ms  (n ${t.n})` : '–'}`;
  return [row('draw', s.draw), row('input', s.input), row('tick', s.tick)].join('\n');
}

let bound = new Set();
let worker = null;
const send = (m) => worker && worker.postMessage(m);

// keys
addEventListener('keydown', (e) => {
  // the browser's own (cmd+r, cmd+w) and keys still being composed
  if (!worker || e.metaKey || e.isComposing) return;
  const name = key_binding(e.key, e.code, e.ctrlKey, e.altKey, e.shiftKey);
  if (name === undefined) return;
  if (bound.has(name)) e.preventDefault();
  send({ t: 'key', name });
});

// the mouse and touch: a pixel's cell on the canvas, or none outside it
function cellAt(e) {
  const r = canvas.getBoundingClientRect();
  if (r.width === 0 || r.height === 0) return null;
  const px = ((e.clientX - r.left) * canvas.width) / r.width;
  const py = ((e.clientY - r.top) * canvas.height) / r.height;
  const x = Math.floor(px / cellW), y = Math.floor(py / cellH);
  const w = canvas.width / cellW, h = canvas.height / cellH;
  return x >= 0 && y >= 0 && x < w && y < h ? [x, y] : null;
}

// the pointer's moves go only when its cell changes; the worker keeps the
// last of those that came before a frame
let pointed = null;
function moveTo(c) {
  if (!c || (pointed && pointed[0] === c[0] && pointed[1] === c[1])) return;
  pointed = c;
  send({ t: 'move', x: c[0], y: c[1] });
}
function pressAt(c, name) {
  moveTo(c);
  send({ t: 'press', name, x: c[0], y: c[1] });
}

// a finger: a tap is a click, held still it presses (a held button
// repeats), and a vertical swipe scrolls as the wheel does (M 8.1)
const HOLD_MS = 150;
let touch = null;

canvas.addEventListener('pointerdown', (e) => {
  const c = cellAt(e);
  if (!c) return;
  e.preventDefault();
  canvas.setPointerCapture?.(e.pointerId);
  if (e.pointerType === 'mouse') {
    const name = mouse_binding(e.button, e.ctrlKey, e.altKey, e.shiftKey);
    if (name !== undefined) pressAt(c, name);
    return;
  }
  const t = { id: e.pointerId, cell: c, y: e.clientY, pressed: false, swiping: false, timer: 0 };
  t.timer = setTimeout(() => {
    t.timer = 0;
    if (!t.swiping) {
      t.pressed = true;
      pressAt(t.cell, 'click');
    }
  }, HOLD_MS);
  touch = t;
});

canvas.addEventListener('pointermove', (e) => {
  if (e.pointerType === 'mouse') {
    moveTo(cellAt(e));
    return;
  }
  const t = touch;
  if (!t || t.id !== e.pointerId || t.pressed) return;
  const step = cellH / dpr;
  const dy = e.clientY - t.y;
  if (!t.swiping && Math.abs(dy) < step * 0.6) return;
  t.swiping = true;
  if (t.timer) {
    clearTimeout(t.timer);
    t.timer = 0;
  }
  // the finger down shows what is above: the wheel up
  const steps = Math.trunc(dy / step);
  if (steps === 0) return;
  t.y += steps * step;
  const name = wheel_binding(steps > 0, false, false, false);
  for (let i = 0; i < Math.min(Math.abs(steps), 5); i++) send({ t: 'press', name, x: t.cell[0], y: t.cell[1] });
});

function lift(e) {
  if (e.pointerType === 'mouse') {
    send({ t: 'release' });
    return;
  }
  const t = touch;
  if (!t || t.id !== e.pointerId) return;
  touch = null;
  if (t.timer) {
    // a tap shorter than the hold: a click now, let go a moment later so
    // the button shows it was pressed
    clearTimeout(t.timer);
    if (!t.swiping) {
      pressAt(t.cell, 'click');
      setTimeout(() => send({ t: 'release' }), 120);
      return;
    }
  }
  send({ t: 'release' });
}
canvas.addEventListener('pointerup', lift);
canvas.addEventListener('pointercancel', lift);
addEventListener('mousemove', (e) => moveTo(cellAt(e)));
// the right button is the game's
canvas.addEventListener('contextmenu', (e) => e.preventDefault());

let wheelPx = 0;
canvas.addEventListener('wheel', (e) => {
  e.preventDefault();
  const c = cellAt(e);
  if (!c) return;
  // lines and pages are steps; a trackpad's pixels add up to them
  let steps;
  if (e.deltaMode !== 0) {
    steps = Math.sign(e.deltaY);
  } else {
    wheelPx += e.deltaY;
    steps = Math.trunc(wheelPx / WHEEL_PX);
    wheelPx -= steps * WHEEL_PX;
  }
  const name = wheel_binding(steps < 0, e.ctrlKey, e.altKey, e.shiftKey);
  for (let i = 0; i < Math.min(Math.abs(steps), 5); i++) send({ t: 'press', name, x: c[0], y: c[1] });
}, { passive: false });

// the window's size in cells at each font size; the worker answers with
// the size it took (`font`)
function sizes() {
  const list = fonts();
  const p = pixels();
  return { list, p, key: `${p.pw}x${p.ph}@${dpr}:${list.map((f) => f.css + '=' + f.cols + 'x' + f.rows).join(',')}` };
}
new ResizeObserver(() => {
  const s = sizes();
  if (s.key === sent) return;
  sent = s.key;
  // a new cell size draws every cell again, as a new size does; the
  // pixels tell portrait from landscape (D 11.1)
  send({ t: 'resize', fonts: s.list, cols: s.list[0].cols, rows: s.list[0].rows, ...s.p });
}).observe(document.body);
// a long press shows no menu and selects nothing (M 8.1)
addEventListener('contextmenu', (e) => e.preventDefault());
addEventListener('selectstart', (e) => e.preventDefault());

// pads: gilrs reads the Gamepad API once a frame; the browser shows a
// pad only after one of its buttons is pressed on the page
function pollPads(pads) {
  try {
    const events = pads.poll();
    if (events.length) send({ t: 'pad', events });
  } catch (e) {
    console.warn('rlml-web: pads stop:', e);
    return;
  }
  requestAnimationFrame(() => pollPads(pads));
}

async function start() {
  await init();
  const res = await fetch('game.json');
  if (!res.ok) throw new Error(`game.json: ${res.status} ${res.statusText}`);
  const bundle = await res.text();
  worker = new Worker('worker.js', { type: 'module' });
  worker.onerror = (e) => show(`Worker: ${e.message || e}`, true);
  worker.onmessage = (e) => {
    const m = e.data;
    switch (m.t) {
      case 'ready':
        bound = new Set(m.keys);
        message.hidden = true;
        statsBox.hidden = !params.has('stats');
        try {
          pollPads(new Pads());
        } catch (err) {
          console.warn('rlml-web: no pads:', err);
        }
        break;
      case 'font':
        // the size the worker took: its cells come next
        fontCss = m.css;
        measure();
        cols = m.cols;
        rows = m.rows;
        break;
      case 'cells':
        paint(new Uint32Array(m.cells));
        break;
      case 'stats':
        statsBox.textContent = statsText(m);
        console.debug('rlml-web times (ms):', m);
        break;
      case 'error':
        show(m.text, true);
        break;
    }
  };
  const s = sizes();
  sent = s.key;
  worker.postMessage({ t: 'load', bundle, seed, fonts: s.list, cols: s.list[0].cols, rows: s.list[0].rows, ...s.p });
}

start().catch((e) => show(String(e && e.message ? e.message : e), true));
