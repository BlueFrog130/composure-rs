# Composure regression bot (Cloudflare)

A bot that answers every interaction with a description of what it parsed: option values, resolved users/channels/roles/attachments, select values, modal inputs. Tests send signed Discord-shaped payloads and assert on the exact responses.

`tests/common` holds the fixtures: a fixed ed25519 test key and about 20 cases. They cover every option type, subcommands and groups, user/message commands, a user-installed command in a bot DM, autocomplete (string, partial integer/number input, inside groups), buttons, string/user selects, and modals (action rows and labels). Error paths are covered too: handler error, tampered or missing signature, and a non-interaction body.

| Test | Runs | What it covers |
|---|---|---|
| `tests/offline.rs` | always (`cargo test`) | Fixtures through `handle_interaction` on the host: signature, parsing, dispatch, response JSON |
| `tests/wrangler.rs` | `#[ignore]` | The same fixtures over HTTP against the bot in `wrangler dev`, so on the real Workers runtime |

Running the `wrangler dev` test needs Node and `worker-build` (`cargo install worker-build@0.8.7`):

```sh
cargo test -p composure_example_cloudflare --test wrangler -- --ignored --nocapture
```

It writes `.dev.vars` with the fixtures' public key, starts wrangler on port 8799 (override with `WRANGLER_PORT`), and stops it afterwards.

To try the bot in Discord, deploy it with `npx wrangler deploy`, set `DISCORD_PUBLIC_KEY` with `npx wrangler secret put DISCORD_PUBLIC_KEY`, and point the application's Interactions Endpoint URL at the worker. The commands it understands are the ones `composure_api`'s live regression publishes.
