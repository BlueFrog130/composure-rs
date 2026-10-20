// What both coldstart.mjs and profile.mjs send: a signing key, Discord-shaped payloads, the
// Miniflare options for a worker-build output, and a checked request.

import { generateKeyPairSync, sign } from "node:crypto";
import { join } from "node:path";

// The signing key Discord would have; the worker gets its public half as DISCORD_PUBLIC_KEY
const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const publicHex = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");

const user = {
  id: "53908232506183680",
  username: "alice",
  discriminator: "0",
  global_name: "Alice",
  avatar: "a_d5efa99b3eeaa7dd43acca82f5692432",
  avatar_decoration_data: null,
  collectibles: null,
  primary_guild: null,
  public_flags: 64,
};
// Shaped like what Discord sends for a guild slash command (~1.5 KB)
const interaction = (type, data) => ({
  app_permissions: "2248473465835073",
  application_id: "1052358444704862208",
  authorizing_integration_owners: { 0: "290926444748734465" },
  channel: {
    flags: 0,
    guild_id: "290926444748734465",
    id: "1052358570903326730",
    last_message_id: "1294701442004602880",
    name: "general",
    nsfw: false,
    parent_id: null,
    permissions: "2251799813685247",
    position: 0,
    rate_limit_per_user: 0,
    topic: null,
    type: 0,
  },
  channel_id: "1052358570903326730",
  context: 0,
  data,
  entitlement_sku_ids: [],
  entitlements: [],
  guild: { features: [], id: "290926444748734465", locale: "en-US" },
  guild_id: "290926444748734465",
  guild_locale: "en-US",
  id: "1294708417736515594",
  locale: "en-US",
  member: {
    avatar: null,
    banner: null,
    communication_disabled_until: null,
    deaf: false,
    flags: 0,
    joined_at: "2021-01-12T21:18:10.481000+00:00",
    mute: false,
    nick: null,
    pending: false,
    permissions: "2251799813685247",
    premium_since: null,
    roles: ["1052363286102274130"],
    unusual_dm_activity_until: null,
    user,
  },
  token: "aW50ZXJhY3Rpb246MTI5NDcwODQxNzczNjUxNTU5NDp0b2tlbg".repeat(4),
  type,
  version: 1,
});
export const PING = { application_id: "1052358444704862208", id: "1", token: "t", type: 1, version: 1, user };
export const GREET = interaction(2, {
  guild_id: "290926444748734465",
  id: "1052358444704862218",
  name: "greet",
  type: 1,
});

function signed(payload) {
  const body = JSON.stringify(payload);
  const timestamp = String(Math.floor(Date.now() / 1000));
  const signature = sign(null, Buffer.from(timestamp + body), privateKey).toString("hex");
  return {
    method: "POST",
    body,
    headers: {
      "Content-Type": "application/json",
      "X-Signature-Ed25519": signature,
      "X-Signature-Timestamp": timestamp,
    },
  };
}

// Answers like the bot would, so its timings are the harness's own overhead
const BASELINE = `export default {
  async fetch(req) {
    const body = await req.json();
    return Response.json(body.type === 1 ? { type: 1 } : { type: 4, data: { content: "Hello" } });
  },
};`;

export function options(variant) {
  const shared = {
    // Worker logs would swamp the output
    handleRuntimeStdio(stdout, stderr) {
      stdout.resume();
      stderr.resume();
    },
    compatibilityDate: "2026-07-30",
    bindings: { DISCORD_PUBLIC_KEY: publicHex, GREETING: "Hello", DISCORD_APPLICATION_ID: "" },
    kvNamespaces: ["NOTES"],
  };
  if (!variant) return { workers: [{ ...shared, name: "baseline", modules: true, script: BASELINE }] };
  const worker = {
    ...shared,
    name: variant.name,
    modulesRoot: variant.dir,
    modules: [
      { type: "ESModule", path: join(variant.dir, "index.js") },
      { type: "CompiledWasm", path: join(variant.dir, "index_bg.wasm") },
    ],
  };
  return { workers: [worker] };
}

export async function request(mf, payload, expect) {
  const response = await mf.dispatchFetch("http://localhost/", signed(payload));
  const text = await response.text();
  if (response.status !== 200 || !text.includes(expect)) {
    throw new Error(`unexpected response ${response.status}: ${text.slice(0, 300)}`);
  }
}
