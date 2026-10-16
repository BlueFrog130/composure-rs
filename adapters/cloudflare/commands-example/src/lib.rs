//! Example bot: every command defined once with `#[command]`, collected into a Command Set, and
//! served on Cloudflare Workers. `src/bin/sync.rs` syncs the same Command Set with Discord.

use composure::{
    Choice, Ctx, command,
    command::{Ephemeral, HandlerError, InteractionHandler},
    commands, group,
    models::{Embed, InteractionResponse, Message, MessageComponentInteraction, User},
};
use composure_adapter_cloudflare::CloudflareInteractionBot;
use worker::{Context, Env, Request, Response, event};

/// Say hello
#[command(
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall)
)]
pub fn hello(ctx: Ctx) -> String {
    // Server nickname in guilds; display name or username in DMs
    let name = match (ctx.member(), ctx.user()) {
        (Some(member), _) => member.display_name(),
        (None, Some(user)) => user.display_name(),
        (None, None) => "stranger",
    };
    format!("Hello, {name}!")
}

/// Repeat some text
#[command]
pub fn echo(
    #[option("What to say", max_len = 200)] text: &str,
    #[option("How many times", min = 1, max = 5)] times: Option<u8>,
) -> String {
    vec![text; times.unwrap_or(1).into()].join(" ")
}

#[derive(Choice)]
pub enum Flavor {
    Vanilla,
    Chocolate,
    #[choice(name = "Mint chip", value = "mint")]
    MintChip,
}

/// Order ice cream
#[command(name = "ice-cream")]
pub fn ice_cream(
    #[option("Flavor")] flavor: Flavor,
    #[option("Scoops")] scoops: Option<u8>,
) -> Embed {
    Embed::new()
        .with_title(&format!("{} x {}", scoops.unwrap_or(1), flavor.name()))
        .with_color(0xf5c2e7)
}

/// What `/greet` needs from the State: `worker::Env` on Workers, a stand-in in tests
pub trait Vars {
    fn var(&self, name: &str) -> Option<String>;
}

impl Vars for Env {
    fn var(&self, name: &str) -> Option<String> {
        Env::var(self, name).ok().map(|var| var.to_string())
    }
}

/// Greet with the GREETING var from wrangler.jsonc
#[command]
pub fn greet(ctx: Ctx<impl Vars>) -> Result<String, HandlerError> {
    let greeting = ctx
        .state()
        .var("GREETING")
        .ok_or_else(|| HandlerError::new("GREETING is not set"))?;
    Ok(format!("{greeting} from the worker!"))
}

/// Ban a member
#[command]
pub fn ban(
    ctx: Ctx,
    #[option("Who to ban")] user: &User,
    #[option("Why")] reason: Option<&str>,
) -> Result<Ephemeral<String>, HandlerError> {
    if ctx.user().is_some_and(|me| me.id == user.id) {
        return Err(HandlerError::user("You can't ban yourself."));
    }
    Ok(Ephemeral(format!(
        "Would ban {} ({})",
        user.display_name(),
        reason.unwrap_or("no reason")
    )))
}

/// Kick a member
#[command]
pub fn kick(#[option("Who to kick")] user: &User) -> Ephemeral<String> {
    Ephemeral(format!("Would kick {}", user.display_name()))
}

/// Moderation tools (dev guild only)
#[group(subcommands(ban, kick), scope = "dev", permissions = Permissions::BanMembers)]
pub struct Mod;

/// Shows a user's avatar
#[command(user, name = "Avatar")]
pub fn avatar(target: &User) -> String {
    match &target.avatar {
        Some(hash) => format!(
            "https://cdn.discordapp.com/avatars/{}/{hash}.png",
            target.id
        ),
        None => format!("{} has no avatar", target.display_name()),
    }
}

/// Counts the words in a message
#[command(message, name = "Word count")]
pub fn word_count(target: &Message) -> String {
    format!("{} words", target.content.split_whitespace().count())
}

commands! {
    /// Every command this bot serves
    pub struct Bot = [hello, echo, ice_cream, greet, Mod, avatar, word_count];
}

/// Everything that isn't a command: here, any button click
pub struct Components;

impl<S> InteractionHandler<S> for Components {
    async fn component(
        &self,
        component: MessageComponentInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        Ok(InteractionResponse::respond_with_ephemeral_message(
            format!("Clicked {}", component.data.custom_id),
        ))
    }
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> worker::Result<Response> {
    CloudflareInteractionBot::new(req, env)
        .with_handler(Bot.with_fallback(Components))
        .process()
        .await
}
