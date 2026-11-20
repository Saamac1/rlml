// UTF-8 TextEncoder and TextDecoder, which the wasm-bindgen glue needs and
// JavaScriptCore's shell lacks.
globalThis.TextEncoder = class { encode(s = '') { const out = []; for (const ch of s) { let c = ch.codePointAt(0);
  if (c < 0x80) out.push(c); else if (c < 0x800) out.push(0xc0 | c >> 6, 0x80 | c & 63);
  else if (c < 0x10000) out.push(0xe0 | c >> 12, 0x80 | c >> 6 & 63, 0x80 | c & 63);
  else out.push(0xf0 | c >> 18, 0x80 | c >> 12 & 63, 0x80 | c >> 6 & 63, 0x80 | c & 63); } return new Uint8Array(out); }
  encodeInto(s, view) { const b = this.encode(s); view.set(b); return { read: s.length, written: b.length }; } };
globalThis.TextDecoder = class { constructor() {} decode(b) { if (!b) return ''; let s = '', i = 0; const u = b instanceof Uint8Array ? b : new Uint8Array(b.buffer || b);
  while (i < u.length) { let c = u[i++]; if (c >= 0xf0) c = (c & 7) << 18 | (u[i++] & 63) << 12 | (u[i++] & 63) << 6 | u[i++] & 63;
  else if (c >= 0xe0) c = (c & 15) << 12 | (u[i++] & 63) << 6 | u[i++] & 63; else if (c >= 0xc0) c = (c & 31) << 6 | u[i++] & 63;
  s += String.fromCodePoint(c); } return s; } };
