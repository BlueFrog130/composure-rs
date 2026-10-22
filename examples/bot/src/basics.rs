//! Commands that only answer: Options, Choices, State, Groups, context menus, errors, and
//! embeds in the bot's colors ([`style`](crate::style)).

use composure::{
    Choice, Ctx, app, autocomplete, command,
    command::{Ephemeral, HandlerError, HasData},
    group,
    types::{Embed, Message, User},
};

use crate::style;

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
    style::embed(style::PRIMARY).with_title(&format!("{} x {}", scoops.unwrap_or(1), flavor.name()))
}

const FRUITS: [&str; 8] = [
    "apple",
    "apricot",
    "banana",
    "blueberry",
    "cherry",
    "grape",
    "mango",
    "melon",
];

/// Suggests fruits starting with what the user typed
#[autocomplete]
pub fn fruit_names(input: &str) -> Vec<&'static str> {
    let input = input.to_lowercase();
    FRUITS
        .into_iter()
        .filter(|fruit| fruit.starts_with(&input))
        .collect()
}

/// Pick a fruit, with suggestions while you type
#[command]
pub fn fruit(#[option("Fruit", autocomplete = fruit_names)] name: &str) -> String {
    format!("You picked {name}")
}

/// The bot's own settings, its Bot Data, loaded by each platform: from wrangler.toml vars on
/// Workers (`with_data`), from the environment on tokio, part of the server's data there
pub struct Config {
    /// What `/greet` says
    pub greeting: String,
}

/// So a State whose Bot Data is exactly a `Config` serves `/greet` too
impl AsRef<Config> for Config {
    fn as_ref(&self) -> &Config {
        self
    }
}

/// Greet with the configured greeting
#[command]
pub fn greet(ctx: Ctx<impl HasData<Data: AsRef<Config>>>) -> String {
    format!("{} from Composure!", ctx.state().data().as_ref().greeting)
}

/// Ban a member
#[command]
pub fn ban(
    ctx: Ctx,
    #[option("Who to ban")] user: &User,
    #[option("Why")] reason: Option<&str>,
) -> Result<Ephemeral<Embed>, HandlerError> {
    if ctx.user().is_some_and(|me| me.id == user.id) {
        return Err(HandlerError::user("You can't ban yourself."));
    }
    Ok(Ephemeral(style::embed(style::WARNING).with_description(
        &format!(
            "Would ban {} ({})",
            user.display_name(),
            reason.unwrap_or("no reason")
        ),
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

/// This module's Commands, listed in [`Bot`](crate::Bot)
#[app(handlers(hello, echo, ice_cream, fruit, greet, Mod, avatar, word_count))]
pub struct Basics;
