//! Commands that call Discord's REST API: inline with the bot token, and as Background Work after
//! a Deferred Response. `Ctx<impl Platform>` keeps them testable off any platform.

use std::time::Duration;

use composure::{
    Ctx, Mentionable, autocomplete, command,
    command::{Deferred, Ephemeral, HandlerError, Platform},
    definition::ApplicationCommand,
    rest::{CommandScope, EditMessage},
    types::{File, InteractionResponse, Message},
};

/// Ask the bot who it is
#[command]
pub async fn whoami(ctx: Ctx<impl Platform>) -> Result<String, HandlerError> {
    // Inline REST: fine while it stays well inside Discord's 3 seconds
    let me = ctx.bot()?.get_current_user().await?.model()?;
    Ok(format!("I'm {}.", me.display_name()))
}

/// Remind you about something in a few seconds
#[command]
pub fn remind(
    ctx: Ctx<impl Platform>,
    #[option("What to remind you of", max_len = 200)] text: &str,
    #[option("Seconds to wait", min = 1, max = 10)] seconds: u8,
) -> Ephemeral<Deferred> {
    // Background Work outlives the request, so it owns what it needs
    let text = text.to_owned();
    Ephemeral(ctx.defer(async move |ctx| {
        ctx.sleep(Duration::from_secs(seconds.into())).await;
        let mention = ctx.user().map(|user| user.to_mention()).unwrap_or_default();
        ctx.client()
            .followup(&format!("\u{23F0} {mention} {text}"))
            .await?;
        Ok::<_, HandlerError>(format!("Reminded you about \"{text}\"."))
    }))
}

/// Count down, editing the response every second
#[command]
pub fn countdown(ctx: Ctx<impl Platform>) -> Deferred {
    ctx.defer(async |ctx| {
        let client = ctx.client();
        for n in (1..=3).rev() {
            ctx.sleep(Duration::from_secs(1)).await;
            let content = format!("{n}...");
            client
                .edit_original(&EditMessage {
                    content: Some(&content),
                    ..Default::default()
                })
                .await?;
        }
        ctx.sleep(Duration::from_secs(1)).await;
        Ok::<_, HandlerError>("Liftoff! \u{1F680}")
    })
}

/// Post a message as the bot (dev guild only)
#[command(scope = "dev", permissions = Permissions::ManageMessages)]
pub async fn announce(
    ctx: Ctx<impl Platform>,
    #[option("What to announce", max_len = 2000)] text: &str,
) -> Result<Ephemeral<&'static str>, HandlerError> {
    let channel = ctx
        .channel_id()
        .ok_or_else(|| HandlerError::user("Use this in a channel."))?;
    // The body is never read, so it is never parsed
    ctx.bot()?.send_message(*channel, text).await?;
    Ok(Ephemeral("Posted."))
}

/// Reacts to a message as the bot
#[command(message, name = "Thumbs up")]
pub async fn thumbs_up(
    ctx: Ctx<impl Platform>,
    target: &Message,
) -> Result<Ephemeral<&'static str>, HandlerError> {
    ctx.bot()?
        .create_reaction(target.channel_id, target.id, "\u{1F44D}")
        .await?;
    Ok(Ephemeral("Reacted."))
}

/// Have the bot send you a DM
#[command(name = "dm-me")]
pub async fn dm_me(
    ctx: Ctx<impl Platform>,
    #[option("What to send", max_len = 2000)] text: &str,
) -> Result<Ephemeral<&'static str>, HandlerError> {
    let user = ctx
        .user()
        .ok_or_else(|| HandlerError::user("No user to DM."))?;
    let bot = ctx.bot()?;
    let dm = bot.create_dm(user.id).await?.model()?;
    bot.send_message(dm.id, text)
        .await
        .map_err(|_| HandlerError::user("I can't DM you; check your privacy settings."))?;
    Ok(Ephemeral("Check your DMs."))
}

/// Export this bot's global commands as a CSV file
#[command]
pub fn report(ctx: Ctx<impl Platform>) -> Ephemeral<Deferred> {
    Ephemeral(ctx.defer(async |ctx| {
        let commands = ctx
            .bot()?
            .get_commands(CommandScope::Global)
            .await?
            .model()?;
        let mut csv = String::from("name,type\n");
        for command in &commands {
            let (name, kind) = match command {
                ApplicationCommand::ChatInputCommand(c) => (&c.details.name, "slash"),
                ApplicationCommand::UserCommand(c) => (&c.name, "user"),
                ApplicationCommand::MessageCommand(c) => (&c.name, "message"),
            };
            csv.push_str(&format!("{name},{kind}\n"));
        }
        // Background Work stays in the Interaction's span: this line has its id, user, ...
        tracing::info!(count = commands.len(), "exported global commands");
        // Files in a deferred result go out with the edit as multipart
        Ok::<_, HandlerError>(
            InteractionResponse::respond_with_message(format!(
                "{} global commands",
                commands.len()
            ))
            .with_file(File::new("commands.csv", csv).content_type("text/csv")),
        )
    }))
}

/// Suggests this bot's global command names, fetched with the bot token while the user types
#[autocomplete]
pub async fn command_names(
    ctx: Ctx<impl Platform>,
    input: &str,
) -> Result<Vec<String>, HandlerError> {
    let commands = ctx
        .bot()?
        .get_commands(CommandScope::Global)
        .await?
        .model()?;
    Ok(commands
        .iter()
        .map(command_name)
        .filter(|name| name.starts_with(input))
        .map(str::to_string)
        .collect())
}

/// Describe one of this bot's commands
#[command(name = "command-info")]
pub async fn command_info(
    ctx: Ctx<impl Platform>,
    #[option("Command", autocomplete = command_names)] name: &str,
) -> Result<Ephemeral<String>, HandlerError> {
    let commands = ctx
        .bot()?
        .get_commands(CommandScope::Global)
        .await?
        .model()?;
    let command = commands
        .iter()
        .find(|command| command_name(command) == name)
        .ok_or_else(|| HandlerError::user(format!("No command named `{name}`.")))?;
    Ok(Ephemeral(match command {
        ApplicationCommand::ChatInputCommand(c) => format!("`/{name}`: {}", c.description),
        ApplicationCommand::UserCommand(_) => format!("`{name}` is a user command"),
        ApplicationCommand::MessageCommand(_) => format!("`{name}` is a message command"),
    }))
}

fn command_name(command: &ApplicationCommand) -> &str {
    match command {
        ApplicationCommand::ChatInputCommand(c) => &c.details.name,
        ApplicationCommand::UserCommand(c) => &c.name,
        ApplicationCommand::MessageCommand(c) => &c.name,
    }
}

/// Get your text back as a file
#[command]
pub fn export(
    #[option("What to put in the file", max_len = 2000)] text: &str,
) -> InteractionResponse {
    // Files in the initial response: the Adapter sends it through the callback endpoint
    InteractionResponse::respond_with_message("Here's your file:".to_string())
        .with_file(File::new("export.txt", text.as_bytes().to_vec()).content_type("text/plain"))
}
