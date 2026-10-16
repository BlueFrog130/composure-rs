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

**Command Set**:
The explicit list of Commands a bot serves; the single source both for dispatching Interactions and for Sync.
_Avoid_: Router, registry

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
