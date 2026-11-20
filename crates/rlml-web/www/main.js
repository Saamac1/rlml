// The page: the game runs in a worker (worker.js), and the page draws
// the cells it sends on a canvas, as a terminal would, and sends it the
// keys by their binding names (`k`, `ctrl+q`, `space`).
//
// URL parameters: `seed=N` (default 1, as `rlml run`), `stats` shows the
// times of draw and the steps in the corner.

import init, { key_binding } from './pkg/rlml_web.js';

const params = new URLSearchParams(location.search);
const seed = Number(params.get('seed') ?? 1) >>> 0;
const message = document.getElementById('message');
const statsBox = document.getElementById('stats');
const canvas = document.getElementById('screen');
const ctx = canvas.getContext('2d', { alpha: false });

const FONT_PX = 16;
const LINE = 1.2;
const FAMILY = 'Menlo, Monaco, "SF Mono", "DejaVu Sans Mono", Consolas, "Liberation Mono", monospace';
// cells.rs: the flags over the character's 21 bits
const CHAR = (1 << 21) - 1, BOLD = 1 << 21, DIM = 1 << 22, ITALIC = 1 << 23, UNDERLINE = 1 << 24, WIDE = 1 << 25;

function show(text, error) {
  message.textContent = text;
  message.className = error ? 'error' : '';
  message.hidden = false;
}

// cell size in device pixels, from the font's own measures
const dpr = window.devicePixelRatio || 1;
const fontPx = Math.round(FONT_PX * dpr);
function font(flags) {
  return `${flags & ITALIC ? 'italic ' : ''}${flags & BOLD ? 'bold ' : ''}${fontPx}px ${FAMILY}`;
}
ctx.font = font(0);
const cellW = Math.ceil(ctx.measureText('M').width);
const cellH = Math.ceil(fontPx * LINE);
const cols = Math.max(20, Math.floor((window.innerWidth * dpr) / cellW));
const rows = Math.max(10, Math.floor((window.innerHeight * dpr) / cellH));

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
  if (canvas.width !== w * cellW || canvas.height !== h * cellH) size(w, h);
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

addEventListener('keydown', (e) => {
  // the browser's own (cmd+r, cmd+w) and keys still being composed
  if (!worker || e.metaKey || e.isComposing) return;
  const name = key_binding(e.key, e.code, e.ctrlKey, e.altKey, e.shiftKey);
  if (name === undefined) return;
  if (bound.has(name)) e.preventDefault();
  worker.postMessage({ t: 'key', name });
});

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
  worker.postMessage({ t: 'load', bundle, seed, cols, rows });
}

start().catch((e) => show(String(e && e.message ? e.message : e), true));
