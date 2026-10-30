//! The Dev Loop (ADR 0018): serves the bot, opens the Tunnel, points the Dev Application at it, and
//! on every save rebuilds, swaps the new bot in and syncs Commands to the dev guild.

use std::{net::TcpListener, process::ExitCode, sync::Arc, time::Instant};

use composure::{
    rest::BotClient,
    types::{Application, Snowflake},
};
use composure_adapter_tokio::HttpTransport;
use tokio::sync::{Notify, mpsc, oneshot};

use crate::{
    config::{Kind, Project, Secrets, TunnelSetting},
    repoint::{HttpProbe, client, identify, repoint},
    runtime::{Runtime, Server, Workers},
    stage::{elapsed, info, warning},
    sync::DevSync,
    tunnel,
    watch::{Change, watch},
};

/// The token and its application, once it is known to be the Dev Application
struct Access {
    client: BotClient<HttpTransport>,
    me: Application,
}

pub async fn run(project: Project) -> ExitCode {
    let result = match &project.kind {
        Kind::Workers(config) => match Workers::new(&project, config) {
            Ok(runtime) => serve(&project, runtime).await,
            Err(error) => Err(error),
        },
        Kind::Server(bin) => serve(&project, Server::new(&project, bin)).await,
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("[composure] {error}");
            ExitCode::FAILURE
        }
    }
}

async fn serve(project: &Project, mut runtime: impl Runtime) -> Result<(), String> {
    let secrets = Secrets::load(&project.dir)?;
    if matches!(project.kind, Kind::Workers(_)) && project.dir.join(".dev.vars").exists() {
        warning!(".dev.vars hides .env from the Worker: move its secrets to .env");
    }
    port_is_free(project.port)?;

    let access = access(project.application, secrets.token.as_deref()).await;
    let sync = start_sync(project, &access);

    let (ready, connected) = oneshot::channel();
    let setting = project.tunnel.clone();
    let port = project.port;
    let public_key = secrets.public_key;
    tokio::spawn(async move {
        if connected.await.is_ok() {
            connect(&setting, port, &access, public_key.as_deref()).await;
        }
    });

    let (_watcher, mut changes) = watch(project.sources.clone(), project.dir.clone())
        .map_err(|e| format!("watching the sources: {e}"))?;
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    let on_swap = move || {
        if let Some(requests) = &sync {
            requests.notify_one();
        }
    };
    supervise(&mut runtime, &mut changes, ready, on_swap, shutdown).await;
    Ok(())
}

/// A port something else listens on fails only after the first build otherwise, and quietly
fn port_is_free(port: u16) -> Result<(), String> {
    for host in ["127.0.0.1", "0.0.0.0"] {
        TcpListener::bind((host, port)).map_err(|e| {
            format!("port {port} is not free: {e}. Set `port` in [package.metadata.composure.dev]")
        })?;
    }
    Ok(())
}

/// Whether the tool may write to Discord: `Err` is why not
async fn access(
    application: Option<Snowflake>,
    token: Option<&str>,
) -> Result<Arc<Access>, String> {
    let application = application
        .ok_or("no Dev Application is set (`application` in [package.metadata.composure.dev])")?;
    let token = token.ok_or("DISCORD_TOKEN is not set (in .env)")?;
    let client = client(HttpTransport::new(), token, application);
    let me = identify(&client, application).await?;
    Ok(Arc::new(Access { client, me }))
}

/// Starts dev Sync when it can run: what to notify after each swap
fn start_sync(project: &Project, access: &Result<Arc<Access>, String>) -> Option<Arc<Notify>> {
    let guild = project.guild?;
    let Some(bin) = &project.sync_bin else {
        warning!(
            "no Sync bin in {}: set `sync_bin`. Commands won't be synced",
            project.package
        );
        return None;
    };
    let access = match access {
        Ok(access) => access,
        Err(reason) => {
            warning!("{reason}. Commands won't be synced");
            return None;
        }
    };
    let requests = Arc::new(Notify::new());
    let sync = DevSync {
        package: project.package.clone(),
        bin: bin.clone(),
        dir: project.dir.clone(),
        application: access.me.id,
        guild,
    };
    tokio::spawn(sync.serve(requests.clone()));
    Some(requests)
}

/// Opens the Tunnel and repoints. Everything here only logs: the bot keeps running without it.
async fn connect(
    setting: &TunnelSetting,
    port: u16,
    access: &Result<Arc<Access>, String>,
    public_key: Option<&str>,
) {
    let since = Instant::now();
    let tunnel = match tunnel::open(setting, port).await {
        Ok(Some(tunnel)) => tunnel,
        Ok(None) => return,
        Err(error) => return warning!("{error}"),
    };
    info!("tunnel connected {} ({})", tunnel.url, elapsed(since));

    match access {
        Ok(access) => {
            let result = repoint(
                &access.client,
                &HttpProbe,
                &access.me,
                &tunnel,
                public_key,
                since,
            )
            .await;
            if let Err(error) = result {
                warning!("{error}");
            }
        }
        Err(reason) => warning!(
            "{reason}. Set the Interactions Endpoint URL by hand: {}",
            tunnel.url
        ),
    }
}

/// Builds and swaps until `shutdown`: first now, then per batch of saves. A failed build leaves
/// the last good bot serving. `ready` fires at the first swap; `on_swap` after each that served
/// something new, once the bot can answer what it adds.
async fn supervise<R: Runtime>(
    runtime: &mut R,
    changes: &mut mpsc::Receiver<Change>,
    ready: oneshot::Sender<()>,
    on_swap: impl Fn(),
    shutdown: impl Future<Output = ()>,
) {
    tokio::pin!(shutdown);
    let mut ready = Some(ready);
    let mut pending = Some(Change::default());
    loop {
        let change = match pending.take() {
            Some(change) => change,
            None => tokio::select! {
                // First: Ctrl-C reaches the bot too, and that exit is expected
                biased;
                () = &mut shutdown => break,
                change = changes.recv() => match change {
                    Some(change) => change,
                    None => break,
                },
                ended = runtime.exited() => {
                    warning!("{ended}; waiting for the next save");
                    continue;
                }
            },
        };
        // Saves made while building, one more build between them
        let mut change = change;
        while let Ok(next) = changes.try_recv() {
            change = change.merge(next);
        }

        let since = Instant::now();
        let built = tokio::select! {
            built = runtime.build() => built,
            () = &mut shutdown => break,
        };
        if !built {
            warning!("build failed; the last good build keeps serving");
            continue;
        }
        match runtime.swap(change.env).await {
            Ok(true) => {
                info!("bot serving ({})", elapsed(since));
                if let Some(ready) = ready.take() {
                    let _ = ready.send(());
                }
                on_swap();
            }
            Ok(false) => {}
            Err(error) => warning!("{error}"),
        }
    }
    runtime.stop().await;
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use super::*;

    /// Records the calls; builds succeed unless scripted
    #[derive(Default)]
    struct Fake {
        calls: Arc<Mutex<Vec<String>>>,
        builds: VecDeque<bool>,
        swaps: VecDeque<Result<bool, String>>,
        /// A build that takes this long, for saves to arrive during it
        build_time: Duration,
    }

    impl Fake {
        fn call(&self, call: impl Into<String>) {
            self.calls.lock().unwrap().push(call.into());
        }
    }

    impl Runtime for Fake {
        async fn build(&mut self) -> bool {
            self.call("build");
            tokio::time::sleep(self.build_time).await;
            self.builds.pop_front().unwrap_or(true)
        }

        async fn swap(&mut self, restart: bool) -> Result<bool, String> {
            self.call(format!("swap restart={restart}"));
            self.swaps.pop_front().unwrap_or(Ok(true))
        }

        async fn exited(&mut self) -> String {
            std::future::pending().await
        }

        async fn stop(&mut self) {
            self.call("stop");
        }
    }

    /// Runs `supervise` until the saves run out, one batch every [`GAP`] from the start: what the
    /// bot was asked to do, and whether the tunnel may open
    async fn run(mut fake: Fake, batches: &[&[Change]]) -> (Vec<String>, bool) {
        let calls = fake.calls.clone();
        let (sender, mut changes) = mpsc::channel(8);
        let (ready, connected) = oneshot::channel();
        let record = calls.clone();
        let on_swap = move || record.lock().unwrap().push("sync".to_string());
        let running = tokio::spawn(async move {
            let shutdown = std::future::pending();
            supervise(&mut fake, &mut changes, ready, on_swap, shutdown).await;
        });
        for batch in batches {
            tokio::time::sleep(GAP).await;
            for save in *batch {
                sender.send(*save).await.unwrap();
            }
        }
        tokio::time::sleep(GAP).await;
        drop(sender);
        running.await.unwrap();

        let ready = connected.await.is_ok();
        let calls = calls.lock().unwrap().clone();
        (calls, ready)
    }

    const GAP: Duration = Duration::from_millis(5);
    const SAVE: Change = Change { env: false };

    #[tokio::test(start_paused = true)]
    async fn starts_with_a_build_and_syncs_only_after_the_swap() {
        let (calls, ready) = run(Fake::default(), &[]).await;
        assert_eq!(calls, ["build", "swap restart=false", "sync", "stop"]);
        assert!(ready);
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_build_swaps_nothing_and_the_next_save_recovers() {
        let fake = Fake {
            builds: [true, false, true].into(),
            ..Fake::default()
        };
        let (calls, _) = run(fake, &[&[SAVE], &[SAVE]]).await;
        assert_eq!(
            calls,
            [
                "build",
                "swap restart=false",
                "sync",
                "build",
                "build",
                "swap restart=false",
                "sync",
                "stop"
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn the_tunnel_waits_for_a_bot_to_serve() {
        let fake = Fake {
            builds: [false].into(),
            ..Fake::default()
        };
        let (calls, ready) = run(fake, &[]).await;
        assert_eq!(calls, ["build", "stop"]);
        assert!(!ready);

        let fake = Fake {
            builds: [false, true].into(),
            ..Fake::default()
        };
        let (calls, ready) = run(fake, &[&[SAVE]]).await;
        assert_eq!(
            calls,
            ["build", "build", "swap restart=false", "sync", "stop"]
        );
        assert!(ready);
    }

    #[tokio::test(start_paused = true)]
    async fn an_unchanged_build_doesnt_sync() {
        let fake = Fake {
            swaps: [Ok(true), Ok(false)].into(),
            ..Fake::default()
        };
        let (calls, _) = run(fake, &[&[SAVE]]).await;
        assert_eq!(
            calls,
            [
                "build",
                "swap restart=false",
                "sync",
                "build",
                "swap restart=false",
                "stop"
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn saves_during_a_build_collapse_into_one_more() {
        let fake = Fake {
            build_time: GAP * 4,
            ..Fake::default()
        };
        let (calls, _) = run(fake, &[&[SAVE, Change { env: true }, SAVE]]).await;
        assert_eq!(
            calls,
            [
                "build",
                "swap restart=false",
                "sync",
                "build",
                "swap restart=true",
                "sync",
                "stop"
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_during_a_build_stops_the_bot() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut fake = Fake {
            calls: calls.clone(),
            build_time: Duration::from_secs(3600),
            ..Fake::default()
        };
        let (_sender, mut changes) = mpsc::channel(1);
        let (ready, _connected) = oneshot::channel();
        let shutdown = tokio::time::sleep(GAP);
        supervise(&mut fake, &mut changes, ready, || {}, shutdown).await;
        assert_eq!(*calls.lock().unwrap(), ["build", "stop"]);
    }

    #[tokio::test]
    async fn a_busy_port_is_an_error() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port_is_free(port).unwrap_err().contains("is not free"));
    }

    #[tokio::test]
    async fn nothing_is_written_without_a_dev_application_or_token() {
        let reason = access(None, Some("t")).await.err().unwrap();
        assert!(reason.contains("no Dev Application"), "{reason}");
        let id = Some("1100146565504565411".parse().unwrap());
        let reason = access(id, None).await.err().unwrap();
        assert!(reason.contains("DISCORD_TOKEN"), "{reason}");
    }
}
