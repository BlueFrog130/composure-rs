// CPU profile of a Worker's first requests on workerd, by function (self time).
//
//   node profile.mjs <build dir> [--out FILE.cpuprofile] [--top N]
//
// Starts a fresh workerd (Miniflare) with the inspector on, samples every 25 us while it serves a
// PING then a `/greet` (each profiled separately), and prints the hottest functions. Function
// names need the wasm name section (worker-build's default keeps it). Open the .cpuprofile in
// Chrome DevTools or VS Code for the flame graph.

import { writeFileSync } from "node:fs";
import { Miniflare } from "miniflare";
import { GREET, PING, options, request } from "./shared.mjs";

const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const i = args.indexOf(name);
  if (i < 0) return fallback;
  const [, value] = args.splice(i, 2);
  return value;
};
const out = opt("--out", null);
const top = Number(opt("--top", 25));
const dir = args[0];
const port = 9230;

const mf = new Miniflare({ ...options({ name: "profiled", dir }), inspectorPort: port });
await mf.ready;

const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
const target = targets.find((t) => t.id.includes("profiled")) ?? targets[0];
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve) => ws.addEventListener("open", resolve, { once: true }));
let nextId = 0;
const pending = new Map();
ws.addEventListener("message", (event) => {
  const message = JSON.parse(event.data);
  if (message.id !== undefined) pending.get(message.id)?.(message);
});
const send = (method, params = {}) =>
  new Promise((resolve) => {
    const id = ++nextId;
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });

await send("Profiler.enable");
await send("Profiler.setSamplingInterval", { interval: 25 });

async function profiled(label, payload, expect) {
  await send("Profiler.start");
  await request(mf, payload, expect);
  const { result } = await send("Profiler.stop");
  report(label, result.profile);
  if (out) writeFileSync(out.replace(/(\.cpuprofile)?$/, `-${label}.cpuprofile`), JSON.stringify(result.profile));
}

function report(label, profile) {
  const nodes = new Map(profile.nodes.map((n) => [n.id, n]));
  const totals = new Map();
  let all = 0;
  profile.samples.forEach((id, i) => {
    const { functionName, url } = nodes.get(id).callFrame;
    const key = functionName || `(anonymous ${url.split("/").pop()})`;
    if (key === "(idle)") return;
    const dt = profile.timeDeltas[i + 1] ?? 0;
    totals.set(key, (totals.get(key) ?? 0) + dt);
    all += dt;
  });
  console.log(`\n${label}: ${(all / 1000).toFixed(2)} ms busy`);
  for (const [name, us] of [...totals].sort((a, b) => b[1] - a[1]).slice(0, top)) {
    console.log(`  ${(us / 1000).toFixed(2).padStart(7)} ms  ${name.slice(0, 160)}`);
  }
}

await profiled("ping", PING, `"type":1`);
await profiled("greet", GREET, "Hello");
await profiled("warm", GREET, "Hello");
ws.close();
await mf.dispose();
