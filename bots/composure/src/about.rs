//! `/about` (what Composure is, and what this bot runs) and `/invite` (install links).

use composure::{Ctx, command, command::View, rsx, types::Permissions};

use crate::{
    Services,
    build_info::{COMPOSURE_VERSION, GIT_SHA},
    ui,
};

const GUIDE: &str = "https://bluefrog130.github.io/composure-rs/";

/// What the bot asks for when added to a server: following the announcement channel creates a
/// webhook
pub const INSTALL_PERMISSIONS: Permissions = Permissions::ManageWebhooks;

/// About Composure and this bot
#[command(
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall)
)]
pub fn about(ctx: Ctx<impl Services>) -> View {
    let repo = ctx.state().data().repo_url();
    let commit = format!("[`{GIT_SHA}`]({repo}/commit/{GIT_SHA})");
    rsx! {
        <ui::Primary>
            <Text>"## Composure"</Text>
            <Text>
                "A Rust framework for Discord bots that answer over HTTP only (no gateway), "
                "on Cloudflare Workers or a tokio server. This bot is built with it and runs on Workers: "
                "`/changelog`, `/docs`, `/crate` and `/notify` are its own source in `bots/composure`."
            </Text>
            <Separator/>
            <Text>"-# Running composure " {COMPOSURE_VERSION} " at " {commit}</Text>
            <Row>
                <Button url=GUIDE>"Guide"</Button>
                <Button url=repo.as_str()>"GitHub"</Button>
                <Button url="https://crates.io/crates/composure">"crates.io"</Button>
                <Button url="https://docs.rs/composure">"docs.rs"</Button>
            </Row>
        </ui::Primary>
    }
}

/// Add this bot to a server, or to your apps to use it anywhere
#[command(
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall)
)]
pub fn invite(ctx: Ctx) -> View {
    let app = ctx.interaction().common.application_id;
    let base = format!("https://discord.com/oauth2/authorize?client_id={app}");
    let server = format!(
        "{base}&scope=bot+applications.commands&permissions={}&integration_type=0",
        INSTALL_PERMISSIONS.bits()
    );
    let user = format!("{base}&scope=applications.commands&integration_type=1");
    rsx! {
        <ui::Primary>
            <Text>"## Add Composure"</Text>
            <Text>
                "**To a server**: release announcements with `/notify`, and every command for everyone there.\n"
                "**To your apps**: `/changelog`, `/docs`, `/crate` and more in any server or DM."
            </Text>
            <Row>
                <Button url=server>"Add to a server"</Button>
                <Button url=user>"Add to my apps"</Button>
            </Row>
        </ui::Primary>
    }
}
