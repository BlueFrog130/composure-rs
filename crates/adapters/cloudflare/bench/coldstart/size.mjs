// Where a wasm file's bytes go: section sizes, code per crate, and the largest functions.
//
//   node size.mjs <file.wasm> [topN]
//
// Function names come from the name section (worker-build's default output keeps it). twiggy
// can't read these modules: they use exception handling.
import { readFileSync } from "node:fs";
import { gzipSync } from "node:zlib";

const buf = readFileSync(process.argv[2]);
const topN = Number(process.argv[3] ?? 30);
let pos = 8;
const leb = () => { let r = 0, s = 0, b; do { b = buf[pos++]; r += (b & 0x7f) * 2 ** s; s += 7; } while (b & 0x80); return r; };
const str = () => { const n = leb(); const s = buf.toString("utf8", pos, pos + n); pos += n; return s; };

const ids = ["custom","type","import","function","table","memory","global","export","start","element","code","data","datacount","tag"];
const sections = [];
let imported = 0, bodies = [], names = new Map(), dataBytes = 0;
while (pos < buf.length) {
  const id = buf[pos++]; const size = leb(); const start = pos; const end = pos + size;
  let name = ids[id] ?? `id${id}`;
  if (id === 0) name = "custom:" + str();
  if (id === 2) { const n = leb(); for (let i = 0; i < n; i++) { str(); str(); const k = buf[pos++]; if (k === 0) { leb(); imported++; } else if (k === 1) { pos++; const f = leb(); leb(); if (f & 1) leb(); } else if (k === 2) { const f = leb(); leb(); if (f & 1) leb(); } else if (k === 3) { pos += 2; } else if (k === 4) { pos++; leb(); } } }
  if (id === 10) { const n = leb(); for (let i = 0; i < n; i++) { const sz = leb(); bodies.push(sz); pos += sz; } }
  if (id === 11) dataBytes = size;
  if (name === "custom:name") {
    while (pos < end) { const sub = buf[pos++]; const sz = leb(); const sEnd = pos + sz;
      if (sub === 1) { const n = leb(); for (let i = 0; i < n; i++) { const idx = leb(); names.set(idx, str()); } }
      pos = sEnd; }
  }
  sections.push([name, size]);
  pos = end;
}
const kb = (n) => (n / 1024).toFixed(1).padStart(8) + " KB";
console.log(`total ${kb(buf.length)}  gzip ${kb(gzipSync(buf, { level: 9 }).length)}`);
for (const [n, s] of sections.sort((a, b) => b[1] - a[1])) if (s > 512) console.log(`  ${n.padEnd(28)}${kb(s)}`);

const fns = bodies.map((s, i) => [names.get(i + imported) ?? `fn${i + imported}`, s]);
const crate = (n) => {
  const m = n.match(/^(?:<+)?(?:&(?:mut )?)?(?:dyn )?([a-z_][a-z0-9_]*)(?:\[|::)/);
  return m ? m[1] : n.startsWith("__") ? "(runtime)" : "(other)";
};
const per = new Map();
for (const [n, s] of fns) per.set(crate(n), (per.get(crate(n)) ?? 0) + s);
console.log(`\ncode by crate (first path segment; generics land in the caller's instantiation crate)`);
for (const [c, s] of [...per].sort((a, b) => b[1] - a[1]).slice(0, 20)) console.log(`  ${c.padEnd(28)}${kb(s)}`);
console.log(`\ntop ${topN} functions`);
for (const [n, s] of fns.sort((a, b) => b[1] - a[1]).slice(0, topN)) console.log(`  ${kb(s)}  ${n.slice(0, 150)}`);
