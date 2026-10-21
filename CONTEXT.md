# Composure

Framework for Discord bots that only receive HTTP interactions (no gateway) and run on edge/serverless platforms or a hosted server.

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
Platform-provided resources (bindings, secrets, env), plus the Bot Data, that an Adapter passes through to Commands.
_Avoid_: Env (outside the Cloudflare Adapter)

**Credentials**:
What identifies and authorizes the Discord application: its public key (required, verifies Interactions), and optionally the bot token and application id. The only configuration Composure itself needs; where it is loaded from is the Adapter's (or the bot's) choice.
_Avoid_: Config, settings, App config

**Bot Data**:
The bot author's own typed configuration and resources, built once and shared by every Interaction; reaches Commands through the State on every Adapter.
_Avoid_: App data, user data, vars

**Response**:
What a Command returns to Discord for an Interaction; plain text, embeds, and full interaction responses all count.

**Deferred Response**:
A Response that only acknowledges the Interaction (Discord shows a loading state) and is completed later by Background Work editing the original response.
_Avoid_: Async response, delayed reply

**Platform**:
What an Adapter's State offers Commands beyond its own resources: a Transport for REST, the bot token, and a way to run Background Work (`Worker` on Cloudflare, `Host` on a hosted server).
_Avoid_: Runtime, environment

**Background Work**:
Work an Adapter runs after it has answered the platform request (`wait_until` on Cloudflare), bounded by the platform's post-response time limit or, on a hosted server, by the shutdown grace period.
_Avoid_: Task, job, spawn

**Webhook Event**:
A non-interaction notification Discord POSTs to the app's events URL (app authorized, entitlement created, ...); ACKed immediately and handled as Background Work.
_Avoid_: Gateway event, Interaction

**Event Handler**:
What answers Webhook Events, registered on an App alongside its Commands.

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
REST client authorized by the bot token; available only when the Credentials include one.
_Avoid_: HTTP client, API client

**Endpoint**:
One Discord REST route (method + path) exposed as a client method.
_Avoid_: Route (that is what an App lists; `routes!` and the spec check are the exception), call

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

**App**:
The explicit list (`#[app]`) of Routes a bot serves: Commands, Groups, Component and Modal Handlers, Views, and other Apps; the single source both for dispatching Interactions and for Sync. Apps nest, so a module can list its own and the root lists the module's.
_Avoid_: Command Set, router, registry, bot (that is the Bot Client)

**Route**:
Anything an App lists: it answers the Interactions addressed to it (by command name and type, or by `custom_id` prefix) and declares what the App's compile-time checks need.
_Avoid_: Handler (alone), endpoint

**Error Renderer**:
How an App answers handler errors: a `fn(&HandlerError) -> InteractionResponse` set with `#[app(errors = ..)]` (at most one App in a tree), used for the initial response and for failed Background Work. Without one, errors are the ephemeral text of `HandlerError::response`. Colors are otherwise the bot's own `const`s, set where it builds messages; there is no framework palette.
_Avoid_: Theme (removed, ADR 0015), error handler (handlers produce errors; this renders them)

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
