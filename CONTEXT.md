# Composure

Framework for Discord bots that only receive HTTP interactions (no gateway) and run on edge/serverless platforms.

## Language

### Runtime

**Interaction**:
One inbound request from Discord (ping, command invocation, component click, autocomplete, modal submit).
_Avoid_: Event, request (when meaning the Discord payload)

**Adapter**:
The platform integration (e.g. Cloudflare Workers) that turns a platform request into an Interaction and hands it to an Interaction Handler.
_Avoid_: Runtime, driver

**Interaction Handler**:
Anything that answers an Interaction with a Response; the single contract every Adapter talks to.
_Avoid_: Command handler (when meaning the low-level contract)

**State**:
Platform-provided resources (bindings, secrets, env) an Adapter passes through to Commands.
_Avoid_: Env (outside the Cloudflare Adapter)

**Response**:
What a Command returns to Discord for an Interaction; plain text, embeds, and full interaction responses all count.

**Deferred Response**:
A Response that only acknowledges the Interaction (Discord shows a loading state) and is completed later by Background Work editing the original response.
_Avoid_: Async response, delayed reply

**Platform**:
What an Adapter's State offers Commands beyond its own resources: a Transport for REST, the bot token, and a way to run Background Work (`Worker` on Cloudflare).
_Avoid_: Runtime, environment

**Background Work**:
Work an Adapter runs after it has answered the platform request (`wait_until` on Cloudflare), bounded by the platform's post-response time limit.
_Avoid_: Task, job, spawn

**Webhook Event**:
A non-interaction notification Discord POSTs to the app's events URL (app authorized, entitlement created, ...); ACKed immediately and handled as Background Work.
_Avoid_: Gateway event, Interaction

**Event Handler**:
What answers Webhook Events, registered on the Command Set alongside its Commands.

### Discord types

**Types**:
Rust representations of Discord's data model (`composure::types`), shared by Interactions, Responses and REST; organized by resource, not by direction.
_Avoid_: Models, DTOs

**Snowflake**:
Discord's 64-bit time-ordered id, serialized as a string.

### REST

**Transport**:
The Adapter-provided way to send one HTTP request and to sleep; the REST client's only contact with the platform.
_Avoid_: HTTP client, connector

**Interaction Client**:
REST client authorized by an Interaction's token (follow-ups, editing the original response); needs no bot token and is always available to a Command.

**Bot Client**:
REST client authorized by the bot token; available only when the Adapter has a `DISCORD_TOKEN` secret.
_Avoid_: HTTP client, API client

**Endpoint**:
One Discord REST route (method + path) exposed as a client method.
_Avoid_: Route (outside the spec check), call

### Commands

**Command**:
A user-invokable application command defined once in code, from which both its runtime behaviour and its Command Definition derive.
_Avoid_: Slash command (too narrow; user/message commands are Commands too)

**Command Definition**:
The description of a Command that Discord stores (name, description, Options, permissions, contexts).
_Avoid_: Command schema, registration

**Option**:
A typed input a user supplies when invoking a Command.
_Avoid_: Argument, parameter (when meaning the Discord-side input)

**Choice**:
One of a fixed set of values an Option is restricted to.

**Group**:
A Command that only contains subcommands (or nested Groups) and cannot be invoked itself.
_Avoid_: Parent command, category

**Component Handler**:
Code answering clicks/selections on message components whose `custom_id` starts with its prefix; may carry typed state encoded in the rest of the `custom_id`.
_Avoid_: Button handler, listener

**Modal Handler**:
Code answering a modal submit routed by `custom_id` prefix, receiving the submitted fields as a typed Modal.

**Modal**:
A typed form shown to the user and read back on submit (`#[derive(Modal)]`): labeled text inputs, selects, radio and checkbox groups, checkboxes, and file uploads.
_Avoid_: Form (alone), dialog

**View**:
A reusable piece of message or modal layout, defined as a pure function of its Props; the components it renders and the handlers for their events derive from it. Re-rendering means answering an Interaction with the View rendered from new Props.
_Avoid_: Template, widget, component (that word is Discord's)

**Props**:
The inputs a View renders from; the only state its inline handlers see, carried between Interactions as Custom ID State.
_Avoid_: Signal, store

**Custom ID State**:
Values a Component or Modal Handler needs, carried in the `custom_id` after its prefix instead of stored anywhere.
_Avoid_: Session, payload

**Autocomplete Handler**:
Code suggesting Choices for one Option while the user types.

**Command Set**:
The explicit list of Commands a bot serves; the single source both for dispatching Interactions and for Sync.
_Avoid_: Router, registry

**Theme**:
A Command Set's embed colors (primary plus success, warning, error, info) and branding (author, footer), filled into every message Response and error it answers with (primary also as the accent color of Components V2 Containers); REST messages are untouched.
_Avoid_: Style, skin, palette (for the whole thing)

### Deployment

**Sync**:
Making Discord's stored Command Definitions match the ones in code, deleting any Discord has that code does not.
_Avoid_: Migration, register, publish

**Scope**:
A named deployment destination for a top-level Command, resolved to concrete guilds at Sync time; Commands without a Scope are global.
_Avoid_: Guild (when meaning the named destination), environment

**Sync Plan**:
The creates, updates, and deletes a Sync would perform, shown before it is applied.
_Avoid_: Diff, migration
