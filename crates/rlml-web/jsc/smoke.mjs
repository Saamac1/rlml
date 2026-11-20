// The worker's calls in JavaScriptCore's shell, without a browser: the
// wasm module and game.json from www/ (build.sh), the keys pressed one
// a player's turn as `rlml run` presses them, the log written out and
// the times of draw, input and tick printed. smoke.sh runs it.
//   jsc -m smoke.mjs -- WWW KEYSFILE OUTFILE
import './polyfill.mjs';
import { initSync, WebGame, key_binding } from '../www/pkg/rlml_web.js';
const [www, keysFile, outFile] = globalThis.arguments;
initSync({ module: readFile(`${www}/pkg/rlml_web_bg.wasm`, 'binary') });
const now = () => preciseTime() * 1000;
print('key_binding:', key_binding('ArrowUp', 'ArrowUp', false, false, false), key_binding('K', 'KeyK', false, false, true), key_binding('Shift', 'ShiftLeft', false, false, true));
try { new WebGame({ main: 'game.rlml', game: { 'game.rlml': 'game G\n- start NOWHERE\n' }, engine: {} }, 1, 80, 24); }
catch (e) { print('load error shown:', String(e).split('\n')[0]); }
const g = new WebGame(JSON.parse(readFile(`${www}/game.json`)), 1, 100, 34);
print('bound keys:', g.bound_keys().length);
const times = { draw: [], input: [], tick: [] };
let clock = 0;
function frame() { while (g.next_frame() !== undefined) {} const t = now(); const c = g.draw(); times.draw.push(now() - t); return c; }
const first = frame();
print('first frame:', first[0], 'x', first[1], 'full', first[2], 'cells', (first.length - 3) / 4);
const keys = readFile(keysFile).trim().split(/\s+/);
for (const k of keys) {
  clock += 10; g.key(k);
  let t = now(); g.input(clock); times.input.push(now() - t);
  t = now(); g.tick(clock); times.tick.push(now() - t); frame();
  // the world's rounds at the `round` pace until the player's turn
  for (let i = 0; i < 4; i++) { clock += 300; t = now(); g.tick(clock); times.tick.push(now() - t); frame(); }
}
const s = (v) => { v.sort((a, b) => a - b); return `mean ${(v.reduce((a, b) => a + b, 0) / v.length).toFixed(2)} p95 ${v[Math.floor((v.length - 1) * 0.95)].toFixed(2)} max ${v[v.length - 1].toFixed(2)} ms (n ${v.length})`; };
for (const n of ['draw', 'input', 'tick']) print(n.padEnd(6), s(times[n]));
writeFile(outFile, g.log_lines().join('\n') + '\n');
