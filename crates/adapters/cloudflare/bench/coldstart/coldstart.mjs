// Cold start benchmark for worker-build outputs, on workerd (Miniflare) and raw V8.
//
//   node coldstart.mjs [--samples N] [--warm M] [--json FILE] <name>=<build dir> ...
//
// Each build dir is a worker-build `-d` output (index.js + index_bg.wasm). Per variant:
// - wasm / gzip: bundle size (Cloudflare limits and fetch time go by these)
// - compile eager / lazy: `new WebAssembly.Module` in a fresh Node (same V8 as workerd), median
//   of N, with every function compiled up front (eager) or on first call (lazy)
// - startup: a fresh workerd (Miniflare) until it serves, minus the same for a JS Worker that
//   only answers PONG; this is the script's compile + global scope + `#[event(start)]`
// - ping: the first request after startup (wasm instance's first call, Credentials loaded and
//   the public key imported, a PING parsed and answered)
// - first cmd: the first slash command after that (`/greet`: dispatch and its code compiled)
// - warm: median of M more `/greet`s
// Variants are interleaved per sample so drift hits them alike. Timers are taken on the Node
// side; Workers freeze clocks while code runs, so the worker can't time itself.

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const self = fileURLToPath(import.meta.url);

// `node coldstart.mjs --compile FILE N`: N compiles of FILE, median ms on stdout. Each compile
// gets a unique trailing custom section, so V8's per-process module cache never hits.
if (process.argv[2] === "--compile") {
  const bytes = readFileSync(process.argv[3]);
  const n = Number(process.argv[4]);
  const times = [];
  for (let i = 0; i < n; i++) {
    const tag = Buffer.from(`bench${i}`);
    const custom = Buffer.concat([Buffer.from([0, tag.length + 2, 1, 0x62]), tag]); // name "b"
    const unique = Buffer.concat([bytes, custom]);
    const t = performance.now();
    new WebAssembly.Module(unique);
    times.push(performance.now() - t);
  }
  console.log(median(times).toFixed(2));
  process.exit(0);
}

const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const i = args.indexOf(name);
  if (i < 0) return fallback;
  const [, value] = args.splice(i, 2);
  return value;
};
const samples = Number(opt("--samples", 15));
const warm = Number(opt("--warm", 50));
const jsonOut = opt("--json", null);
const variants = args.map((arg) => {
  const [name, dir] = arg.split(/=(.*)/s);
  return { name, dir };
});
if (variants.length === 0) {
  console.error("usage: node coldstart.mjs [--samples N] [--warm M] [--json FILE] name=dir ...");
  process.exit(2);
}

const { Miniflare } = await import("miniflare");
const { GREET, PING, options, request } = await import("./shared.mjs");

async function time(fn) {
  const t = performance.now();
  const result = await fn();
  return [performance.now() - t, result];
}

/** One cold start: a new workerd process with only this Worker */
async function run(variant) {
  const mf = new Miniflare(options(variant));
  try {
    const [startup] = await time(() => mf.ready);
    const [ping] = await time(() => request(mf, PING, `"type":1`));
    const [first] = await time(() => request(mf, GREET, "Hello"));
    const warmTimes = [];
    for (let i = 0; i < warm; i++) warmTimes.push((await time(() => request(mf, GREET, "Hello")))[0]);
    return { startup, ping, first, warm: median(warmTimes) };
  } finally {
    await mf.dispose();
  }
}

function median(xs) {
  const sorted = [...xs].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

function compile(wasm, lazy) {
  const flag = lazy ? "--wasm-lazy-compilation" : "--no-wasm-lazy-compilation";
  const out = execFileSync(process.execPath, [flag, self, "--compile", wasm, String(samples)]);
  return Number(out.toString().trim());
}

const rows = variants.map((v) => {
  const wasm = join(v.dir, "index_bg.wasm");
  const bytes = readFileSync(wasm);
  return {
    name: v.name,
    wasm: bytes.length / 1024,
    gzip: gzipSync(bytes, { level: 9 }).length / 1024,
    js: readFileSync(join(v.dir, "index.js")).length / 1024,
    eager: compile(wasm, false),
    lazy: compile(wasm, true),
    runs: [],
  };
});

// Warm the machine (disk cache, workerd binary) before timing
await run(null);
await run(variants[0]);
const baseline = [];
for (let s = 0; s < samples; s++) {
  baseline.push(await run(null));
  for (const [i, v] of variants.entries()) rows[i].runs.push(await run(v));
  process.stderr.write(`sample ${s + 1}/${samples}\r`);
}
process.stderr.write("\n");

const pick = (runs, key) => median(runs.map((r) => r[key]));
const base = Object.fromEntries(["startup", "ping", "first", "warm"].map((k) => [k, pick(baseline, k)]));
// The Worker's share: the same step on the JS baseline subtracted
const own = (row, key) => pick(row.runs, key) - base[key];
const results = rows.map((row) => ({
  name: row.name,
  wasm_kb: row.wasm,
  gzip_kb: row.gzip,
  js_kb: row.js,
  compile_eager_ms: row.eager,
  compile_lazy_ms: row.lazy,
  startup_ms: own(row, "startup"),
  ping_ms: own(row, "ping"),
  first_cmd_ms: own(row, "first"),
  warm_ms: own(row, "warm"),
}));

const f = (n, d = 1) => n.toFixed(d);
console.log(`\ncold start on workerd (miniflare), median of ${samples}; warm = median of ${warm}`);
console.log(
  `JS baseline, subtracted below: startup ${f(base.startup)}, ping ${f(base.ping)}, first ${f(base.first)}, warm ${f(base.warm, 2)} ms\n`,
);
console.log("| variant | wasm KB | gzip KB | compile eager ms | compile lazy ms | startup ms | ping ms | first cmd ms | warm ms |");
console.log("|---|--:|--:|--:|--:|--:|--:|--:|--:|");
for (const r of results) {
  console.log(
    `| ${r.name} | ${f(r.wasm_kb, 0)} | ${f(r.gzip_kb, 0)} | ${f(r.compile_eager_ms)} | ${f(r.compile_lazy_ms, 2)} | ${f(r.startup_ms)} | ${f(r.ping_ms)} | ${f(r.first_cmd_ms)} | ${f(r.warm_ms, 2)} |`,
  );
}
if (jsonOut) writeFileSync(jsonOut, JSON.stringify({ samples, warm, baseline_ms: base, results }, null, 2));
