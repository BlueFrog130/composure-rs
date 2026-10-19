//! Commands defined with the macros: their Definitions, and dispatching Interactions to them.

use composure::{
    Choice, Ctx, app,
    command::{
        CommandSetDefinition, Ephemeral, ErrorKind, HandlerError, InteractionHandler,
        ResolvedMember, UserOrRole, dispatch,
    },
    group,
    types::{Embed, Interaction, InteractionResponse, Message, PartialChannel, Role, User},
};
use futures::executor::block_on;
use serde_json::{Value, json};

/// Repeat some text
#[composure::command]
async fn echo(
    _ctx: Ctx,
    #[option("What to say", min_len = 1, max_len = 200)] text: &str,
    #[option("How many times", min = 1, max = 5)] times: Option<u8>,
) -> String {
    text.repeat(times.unwrap_or(1).into())
}

/// Reply with pong
#[composure::command(contexts(Guild, BotDm), integration_types(GuildInstall, UserInstall))]
fn ping() -> &'static str {
    "pong"
}

/// Show the State the Adapter passed in
#[composure::command(name = "state")]
fn show_state(ctx: Ctx<u32>) -> String {
    format!("state {}", ctx.state())
}

#[derive(Choice, Debug)]
enum Color {
    Red,
    #[choice(name = "Dark blue", value = "dark-blue")]
    DarkBlue,
}

#[derive(Choice, Debug)]
enum Size {
    #[choice(value = 1)]
    Small,
    #[choice(name = "Large", value = 10)]
    Big,
}

/// Pick a color and size
#[composure::command]
fn paint(#[option("Color")] color: Color, #[option("Size")] size: Option<Size>) -> String {
    format!("{color:?} {size:?}")
}

/// Fails on purpose
#[composure::command]
async fn fail() -> Result<String, String> {
    Err("boom".into())
}

/// Describe resolved options
#[composure::command]
fn resolve(
    #[option("A user")] user: User,
    #[option("A member")] member: Option<ResolvedMember>,
    #[option("A channel", channel_types(GuildText))] channel: Option<PartialChannel>,
    #[option("A role")] role: Option<Role>,
    #[option("A user or role")] who: Option<UserOrRole>,
) -> String {
    let who = match who {
        Some(UserOrRole::User(user)) => user.username,
        Some(UserOrRole::Role(role)) => role.name,
        None => "-".into(),
    };
    format!(
        "{} {} {} {} {who}",
        user.username,
        member.map_or("-".into(), |m| m.display_name().to_string()),
        channel.and_then(|c| c.name).unwrap_or("-".into()),
        role.map_or("-".into(), |r| r.name),
    )
}

/// Ban someone
#[composure::command]
fn ban(#[option("Who")] user: &User) -> Embed {
    Embed::new().with_title(&format!("banned {}", user.username))
}

/// Kick someone
#[composure::command]
fn kick() -> &'static str {
    "kicked"
}

/// Add a role
#[composure::command]
fn add() -> &'static str {
    "added"
}

/// Role tools
#[group(subcommands(add))]
struct Roles;

/// Moderation tools
#[group(subcommands(ban, kick, Roles), permissions = Permissions::BanMembers | Permissions::KickMembers, scope = "dev")]
struct Admin;

#[composure::command(user, name = "Inspect")]
fn inspect(target: &User) -> String {
    format!("inspect {}", target.username)
}

#[composure::command(message, name = "Quote")]
fn quote(target: &Message) -> Ephemeral<String> {
    Ephemeral(format!("> {}", target.content))
}

/// Refuses with a message for the user
#[composure::command]
fn refuse() -> Result<String, HandlerError> {
    Err(HandlerError::user("Not allowed here"))
}

/// What a bounded-State Command needs from the State
trait Counter {
    fn count(&self) -> u32;
}

impl Counter for u32 {
    fn count(&self) -> u32 {
        *self
    }
}

/// Count with `impl Bound`
#[composure::command]
async fn count(ctx: Ctx<impl Counter>) -> String {
    format!("count {}", ctx.state().count())
}

/// Count with a named type parameter
#[composure::command]
fn tally<S: Counter>(ctx: Ctx<S>, #[option("Add")] add: Option<u32>) -> String {
    format!("tally {}", ctx.state().count() + add.unwrap_or(0))
}

/// Everything under test
#[app(handlers(
    echo, ping, show_state, paint, fail, resolve, Admin, inspect, quote, refuse, count, tally
))]
pub struct Bot;

/// Bounded-State Commands only, so any `Counter` State works
#[app(handlers(count, tally))]
struct Counting;

#[app(handlers(ping))]
struct Small;

const USER: &str = "100000000000000001";
const ROLE: &str = "200000000000000001";
const CHANNEL: &str = "300000000000000001";
const MESSAGE: &str = "500000000000000001";

fn user() -> Value {
    json!({ "id": USER, "username": "alice", "discriminator": "0", "global_name": null, "avatar": null })
}

fn interaction(data: Value) -> Interaction {
    serde_json::from_value(json!({
        "id": "1100173248714518568",
        "application_id": "1052322265397739523",
        "type": 2,
        "token": "token",
        "version": 1,
        "guild_id": "798662131062931547",
        "user": user(),
        "data": data,
    }))
    .unwrap()
}

fn chat(name: &str, options: Value) -> Interaction {
    interaction(json!({ "id": "1", "name": name, "type": 1, "options": options }))
}

fn run_with<S>(
    handler: &impl InteractionHandler<S>,
    state: &S,
    interaction: Interaction,
) -> Result<Value, HandlerError> {
    block_on(dispatch(interaction, handler, state)).map(|r| serde_json::to_value(r).unwrap())
}

fn run(interaction: Interaction) -> Result<Value, HandlerError> {
    run_with(&Bot, &7u32, interaction)
}

fn content(interaction: Interaction) -> String {
    let response = run(interaction).unwrap();
    response["data"]["content"].as_str().unwrap().to_string()
}

#[test]
fn options_are_extracted() {
    let options = json!([
        { "name": "text", "type": 3, "value": "hi" },
        { "name": "times", "type": 4, "value": 3 },
    ]);
    assert_eq!(content(chat("echo", options)), "hihihi");
    assert_eq!(
        content(chat(
            "echo",
            json!([{ "name": "text", "type": 3, "value": "hi" }])
        )),
        "hi"
    );
}

#[test]
fn state_reaches_ctx() {
    assert_eq!(content(chat("state", json!([]))), "state 7");
    assert_eq!(content(chat("ping", Value::Null)), "pong");
}

#[test]
fn choices_are_extracted() {
    let options = json!([
        { "name": "color", "type": 3, "value": "dark-blue" },
        { "name": "size", "type": 4, "value": 10 },
    ]);
    assert_eq!(content(chat("paint", options)), "DarkBlue Some(Big)");
    assert_eq!(
        (Color::DarkBlue.name(), Color::DarkBlue.value()),
        ("Dark blue", "dark-blue")
    );
    assert_eq!((Size::Big.name(), Size::Big.value()), ("Large", 10));

    let unknown = run(chat(
        "paint",
        json!([{ "name": "color", "type": 3, "value": "green" }]),
    ));
    assert_eq!(unknown.unwrap_err().kind(), ErrorKind::InvalidOption);
}

#[test]
fn missing_or_out_of_range_options_are_errors() {
    let missing = run(chat("echo", json!([]))).unwrap_err();
    assert_eq!(missing.kind(), ErrorKind::InvalidOption);
    assert!(missing.to_string().contains("text"), "{missing}");

    let overflow = run(chat(
        "echo",
        json!([{ "name": "text", "type": 3, "value": "hi" }, { "name": "times", "type": 4, "value": 300 }]),
    ));
    assert_eq!(overflow.unwrap_err().kind(), ErrorKind::InvalidOption);
}

#[test]
fn command_errors_are_handler_errors() {
    let error = run(chat("fail", json!([]))).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Handler);
    assert_eq!(error.to_string(), "boom");
    assert_eq!(
        serde_json::to_value(error.response()).unwrap()["data"]["flags"],
        64
    );
}

#[test]
fn user_errors_are_shown_to_the_user() {
    let error = run(chat("refuse", json!([]))).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::User);
    let response = serde_json::to_value(error.response()).unwrap();
    assert_eq!(response["data"]["content"], "Not allowed here");
    assert_eq!(response["data"]["flags"], 64);
}

#[test]
fn bounded_state() {
    assert_eq!(content(chat("count", json!([]))), "count 7");

    struct Fake;
    impl Counter for Fake {
        fn count(&self) -> u32 {
            40
        }
    }
    let options = json!([{ "name": "add", "type": 4, "value": 2 }]);
    let response = run_with(&Counting, &Fake, chat("tally", options)).unwrap();
    assert_eq!(response["data"]["content"], "tally 42");
}

#[test]
fn unknown_commands_are_reported() {
    let error = run(chat("nope", json!([]))).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UnknownCommand);
    assert_eq!(
        serde_json::to_value(error.response()).unwrap()["data"]["content"],
        "Unknown command."
    );
}

#[test]
fn resolved_options() {
    let data = json!({
        "id": "1",
        "name": "resolve",
        "type": 1,
        "options": [
            { "name": "user", "type": 6, "value": USER },
            { "name": "member", "type": 6, "value": USER },
            { "name": "channel", "type": 7, "value": CHANNEL },
            { "name": "role", "type": 8, "value": ROLE },
            { "name": "who", "type": 9, "value": ROLE },
        ],
        "resolved": {
            "users": { USER: user() },
            "members": { USER: { "nick": "Ali", "roles": [], "joined_at": "2021-01-12T21:18:10.481000+00:00", "permissions": "0" } },
            "channels": { CHANNEL: { "id": CHANNEL, "type": 0, "name": "general" } },
            "roles": { ROLE: { "id": ROLE, "name": "mods", "color": 0, "hoist": false, "position": 1, "permissions": "0", "managed": false, "mentionable": true, "flags": 0 } },
        },
    });
    assert_eq!(content(interaction(data)), "alice Ali general mods mods");
}

fn subcommand(name: &str, options: Value) -> Value {
    json!({ "name": name, "type": 1, "options": options })
}

#[test]
fn groups_route_to_subcommands() {
    let banned = chat(
        "admin",
        json!([subcommand(
            "ban",
            json!([{ "name": "user", "type": 6, "value": USER }])
        )]),
    );
    let banned = interaction_with_resolved_user(banned);
    assert_eq!(
        run(banned).unwrap()["data"]["embeds"][0]["title"],
        "banned alice"
    );

    assert_eq!(
        content(chat("admin", json!([subcommand("kick", json!([]))]))),
        "kicked"
    );

    let nested = chat(
        "admin",
        json!([{ "name": "roles", "type": 2, "options": [subcommand("add", json!([]))] }]),
    );
    assert_eq!(content(nested), "added");

    let unknown = run(chat("admin", json!([subcommand("nope", json!([]))]))).unwrap_err();
    assert_eq!(unknown.kind(), ErrorKind::UnknownCommand);
}

fn interaction_with_resolved_user(interaction: Interaction) -> Interaction {
    let Interaction::ApplicationCommand(command) = interaction else {
        unreachable!()
    };
    let options = command.data.options.as_ref().unwrap();
    let sub = options.subcommand().unwrap();
    let mut data =
        json!({ "id": "1", "name": "admin", "type": 1, "resolved": { "users": { USER: user() } } });
    data["options"] = json!([subcommand(
        &sub.name,
        json!([{ "name": "user", "type": 6, "value": USER }])
    )]);
    self::interaction(data)
}

#[test]
fn context_menu_commands() {
    let inspected = interaction(json!({
        "id": "1", "name": "Inspect", "type": 2, "target_id": USER,
        "resolved": { "users": { USER: user() } },
    }));
    assert_eq!(content(inspected), "inspect alice");

    let quoted = interaction(json!({
        "id": "1", "name": "Quote", "type": 3, "target_id": MESSAGE,
        "resolved": { "messages": { MESSAGE: {
            "id": MESSAGE, "channel_id": CHANNEL, "author": user(), "content": "hello",
            "timestamp": "2021-01-12T21:18:10.481000+00:00", "edited_timestamp": null, "tts": false,
            "mention_everyone": false, "mentions": [], "mention_roles": [], "attachments": [],
            "embeds": [], "pinned": false, "type": 0,
        } } },
    }));
    let response = run(quoted).unwrap();
    assert_eq!(response["data"]["content"], "> hello");

    // Same name as a chat-input command but a different type is not a match
    let wrong_type =
        interaction(json!({ "id": "1", "name": "echo", "type": 2, "target_id": USER }));
    assert_eq!(
        run(wrong_type).unwrap_err().kind(),
        ErrorKind::UnknownCommand
    );
}

struct Components;

impl<S: Sync> InteractionHandler<S> for Components {
    async fn component(
        &self,
        component: composure::types::MessageComponentInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        Ok(InteractionResponse::respond_with_message(format!(
            "clicked {}",
            component.data.custom_id
        )))
    }

    async fn command(
        &self,
        command: composure::types::ApplicationCommandInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        Ok(InteractionResponse::respond_with_message(format!(
            "fallback {}",
            command.data.name
        )))
    }
}

#[test]
fn fallback_handles_the_rest() {
    let handler = Small.with_fallback(Components);
    let pong = run_with(&handler, &(), chat("ping", json!([]))).unwrap();
    assert_eq!(pong["data"]["content"], "pong");

    let other = run_with(&handler, &(), chat("other", json!([]))).unwrap();
    assert_eq!(other["data"]["content"], "fallback other");

    let click: Interaction = serde_json::from_value(json!({
        "id": "1", "application_id": "1", "type": 3, "token": "t", "version": 1,
        "data": { "custom_id": "button", "component_type": 2 },
    }))
    .unwrap();
    assert_eq!(
        run_with(&handler, &(), click).unwrap()["data"]["content"],
        "clicked button"
    );
}

#[test]
fn functions_stay_callable() {
    assert_eq!(ping(), "pong");
    assert_eq!(
        block_on(echo(
            Ctx::new(&std::sync::Arc::new(dummy_command()), &()),
            "a",
            Some(2)
        )),
        "aa"
    );
    // A local binding can share a command's name
    let ping = 1;
    assert_eq!(ping, 1);
}

fn dummy_command() -> composure::types::ApplicationCommandInteraction {
    let Interaction::ApplicationCommand(command) = chat("echo", json!([])) else {
        unreachable!()
    };
    command
}

fn definitions() -> Vec<(Option<&'static str>, Value)> {
    Bot::definitions()
        .into_iter()
        .map(|d| (d.scope, serde_json::to_value(d.command).unwrap()))
        .collect()
}

#[test]
fn chat_input_definitions() {
    let definitions = definitions();
    assert_eq!(
        definitions[0],
        (
            None,
            json!({
                "type": 1,
                "name": "echo",
                "description": "Repeat some text",
                "options": [
                    { "type": 3, "name": "text", "description": "What to say", "required": true, "min_length": 1, "max_length": 200 },
                    { "type": 4, "name": "times", "description": "How many times", "required": false, "min_value": 1, "max_value": 5 },
                ],
            })
        )
    );
    assert_eq!(
        definitions[1].1,
        json!({
            "type": 1, "name": "ping", "description": "Reply with pong",
            "integration_types": [0, 1], "contexts": [0, 1],
        })
    );
    assert_eq!(definitions[2].1["name"], "state");
    assert_eq!(
        definitions[3].1["options"],
        json!([
            { "type": 3, "name": "color", "description": "Color", "required": true, "choices": [
                { "name": "Red", "value": "Red" }, { "name": "Dark blue", "value": "dark-blue" },
            ] },
            { "type": 4, "name": "size", "description": "Size", "required": false, "choices": [
                { "name": "Small", "value": 1 }, { "name": "Large", "value": 10 },
            ] },
        ])
    );
    assert_eq!(definitions[5].1["options"][2]["channel_types"], json!([0]));
}

#[test]
fn bounded_integers_get_type_range() {
    /// Bounded
    #[composure::command]
    fn bounded(#[option("Small")] small: i8, #[option("Big")] big: i64) -> String {
        format!("{small}{big}")
    }
    use composure::command::CommandDefinition;
    let definition = serde_json::to_value(bounded::definition()).unwrap();
    assert_eq!(definition["options"][0]["min_value"], -128);
    assert_eq!(definition["options"][0]["max_value"], 127);
    assert!(definition["options"][1].get("min_value").is_none());
}

#[test]
fn group_definitions() {
    let (scope, admin) = &definitions()[6];
    assert_eq!(*scope, Some("dev"));
    assert_eq!(
        *admin,
        json!({
            "type": 1,
            "name": "admin",
            "description": "Moderation tools",
            "default_member_permissions": "6",
            "options": [
                { "type": 1, "name": "ban", "description": "Ban someone", "options": [
                    { "type": 6, "name": "user", "description": "Who", "required": true },
                ] },
                { "type": 1, "name": "kick", "description": "Kick someone" },
                { "type": 2, "name": "roles", "description": "Role tools", "options": [
                    { "type": 1, "name": "add", "description": "Add a role" },
                ] },
            ],
        })
    );
}

#[test]
fn context_menu_definitions() {
    let definitions = definitions();
    assert_eq!(definitions[7].1, json!({ "type": 2, "name": "Inspect" }));
    assert_eq!(definitions[8].1, json!({ "type": 3, "name": "Quote" }));
}

#[test]
fn compile_errors() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
