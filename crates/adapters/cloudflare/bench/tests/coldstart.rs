//! Cold start benchmark: the example bot (`examples/cloudflare`) built several ways, each started
//! fresh on workerd (Miniflare) and timed by `coldstart/coldstart.mjs`, which documents the
//! columns.
//!
//! A variant is a cargo build (opt-level, toolchain, panic strategy) plus the `wasm-opt` arguments
//! run on its output, each changing one thing from the one before:
//!
//! - `worker-build default`: what a project without wasm-opt settings gets (`-O`, names kept)
//! - `-O`: names dropped
//! - `-Oz no-inline`: binaryen keeps functions with one caller apart
//! - `example`: worker-build with the example's own settings (Cargo.toml metadata and
//!   `.cargo/config.toml`), which should match `-Oz no-inline`
//! - `s` / `3`: other opt-levels; `nightly-abort`: std rebuilt with `panic=immediate-abort`
//!
//! Needs Node and `worker-build` (`cargo install worker-build@0.8.7`); `nightly-abort` also needs
//! a nightly toolchain with `rust-src`. Run with
//! `cargo test -p composure_bench_cloudflare --release --test coldstart -- --ignored --nocapture`.
//! `COLDSTART_SAMPLES` (default 15), `COLDSTART_WARM` (default 50), and `COLDSTART_VARIANTS`
//! (comma-separated names, default all) override the defaults.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

/// How the crate is compiled
struct Build {
    name: &'static str,
    /// `cargo build` arguments, after worker-build's own
    cargo: &'static [&'static str],
    toolchain: Option<&'static str>,
    /// `RUSTFLAGS`, which replace the example's `.cargo/config.toml` flags (name stripping);
    /// `None` keeps them
    rustflags: Option<&'static str>,
}

/// The example as configured: the workspace profile (opt-level "z", curve25519-dalek and sha2 at
/// 3) and its `.cargo/config.toml`
const EXAMPLE: Build = Build {
    name: "example",
    cargo: &[],
    toolchain: None,
    rustflags: None,
};

/// The workspace profile, names kept for wasm-opt to keep or drop
const Z: Build = Build {
    name: "z",
    cargo: &[],
    toolchain: None,
    rustflags: Some(""),
};

const S: Build = Build {
    name: "s",
    cargo: &[
        "--config",
        "profile.release.opt-level=\"s\"",
        "--config",
        "profile.release.package.\"*\".opt-level=\"s\"",
    ],
    toolchain: None,
    rustflags: Some(""),
};

const O3: Build = Build {
    name: "3",
    cargo: &[
        "--config",
        "profile.release.opt-level=3",
        "--config",
        "profile.release.package.\"*\".opt-level=3",
    ],
    toolchain: None,
    rustflags: Some(""),
};

/// std rebuilt so a panic is a bare trap: no panic messages, formatting or location tables
const NIGHTLY_ABORT: Build = Build {
    name: "nightly-abort",
    cargo: &["-Z", "build-std=std,panic_abort"],
    toolchain: Some("nightly"),
    rustflags: Some("-Zunstable-options -Cpanic=immediate-abort"),
};

struct Variant {
    name: &'static str,
    build: &'static Build,
    /// `None`: worker-build's own wasm-opt step, as the example's Cargo.toml configures it
    wasm_opt: Option<&'static [&'static str]>,
}

/// The example's `wasm-opt` arguments
const NO_INLINE: &[&str] = &["-Oz", "--one-caller-inline-max-function-size", "0"];

const VARIANTS: &[Variant] = &[
    Variant {
        name: "worker-build default",
        build: &Z,
        wasm_opt: Some(&["-O", "--debuginfo"]),
    },
    Variant {
        name: "-O",
        build: &Z,
        wasm_opt: Some(&["-O"]),
    },
    Variant {
        name: "-Oz no-inline",
        build: &Z,
        wasm_opt: Some(NO_INLINE),
    },
    Variant {
        name: "example",
        build: &EXAMPLE,
        wasm_opt: None,
    },
    Variant {
        name: "s -Oz no-inline",
        build: &S,
        wasm_opt: Some(NO_INLINE),
    },
    Variant {
        name: "3 -O3 no-inline",
        build: &O3,
        wasm_opt: Some(&["-O3", "--one-caller-inline-max-function-size", "0"]),
    },
    Variant {
        name: "nightly-abort -Oz no-inline",
        build: &NIGHTLY_ABORT,
        wasm_opt: Some(NO_INLINE),
    },
];

/// What worker-build enables for workerd, appended to every explicit argument list
const WASM_FEATURES: &[&str] = &[
    "--enable-mutable-globals",
    "--enable-sign-ext",
    "--enable-nontrapping-float-to-int",
    "--enable-bulk-memory",
    "--enable-multivalue",
    "--enable-reference-types",
    "--enable-exception-handling",
];

fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// worker-build's output for `build` in `out`, wasm-opt skipped unless `optimize`
fn worker_build(example: &Path, target: &Path, build: &Build, out: &Path, optimize: bool) {
    let mut command = Command::new("worker-build");
    command.arg("--release");
    if !optimize {
        command.arg("--no-opt");
    }
    command
        .arg("-d")
        .arg(out)
        .arg(".")
        .arg("--target-dir")
        .arg(target.join(format!("coldstart-{}", build.name)))
        .args(build.cargo)
        .current_dir(example);
    if let Some(toolchain) = build.toolchain {
        // `cargo test` points these at its own toolchain's binaries, past rustup
        command
            .env("RUSTUP_TOOLCHAIN", toolchain)
            .env_remove("CARGO")
            .env_remove("RUSTC")
            .env_remove("RUSTDOC");
    }
    match build.rustflags {
        Some(flags) => command.env("RUSTFLAGS", flags),
        None => command.env_remove("RUSTFLAGS"),
    };
    let status = command.status().expect("failed to run worker-build");
    assert!(status.success(), "worker-build failed for {}", build.name);
}

/// `WASM_OPT_BIN`, else the binaryen worker-build downloaded into its cache
fn wasm_opt() -> PathBuf {
    if let Some(bin) = env::var_os("WASM_OPT_BIN") {
        return bin.into();
    }
    let cache = if cfg!(windows) {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env::var_os("HOME").map(|home| Path::new(&home).join("Library/Caches"))
    } else {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".cache")))
    }
    .expect("no cache directory")
    .join("worker-build");
    let exe = if cfg!(windows) {
        "wasm-opt.exe"
    } else {
        "wasm-opt"
    };
    fs::read_dir(&cache)
        .unwrap_or_else(|e| panic!("{}: {e}", cache.display()))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("wasm-opt-"))
        .map(|entry| entry.path().join("bin").join(exe))
        .find(|path| path.exists())
        .expect("no wasm-opt in worker-build's cache; set WASM_OPT_BIN")
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn npm(dir: &Path, args: &[&str]) {
    let mut command = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "npm"]);
        c
    } else {
        Command::new("npm")
    };
    let status = command
        .args(args)
        .current_dir(dir)
        .status()
        .expect("failed to run npm");
    assert!(status.success(), "npm {args:?} failed");
}

#[test]
#[ignore = "benchmark; builds the example several ways and starts workerd, needs Node and worker-build"]
fn cold_start() {
    let bench = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = bench.join("../../../..");
    let example = root.join("examples/cloudflare");
    let target = root.join("target");
    let out = target.join("coldstart");
    let samples: u32 = env_or("COLDSTART_SAMPLES", 15);
    let warm: u32 = env_or("COLDSTART_WARM", 50);
    let only = env::var("COLDSTART_VARIANTS").ok();
    let selected: Vec<&Variant> = VARIANTS
        .iter()
        .filter(|v| {
            only.as_deref()
                .is_none_or(|only| only.split(',').any(|n| n.trim() == v.name))
        })
        .collect();
    assert!(
        !selected.is_empty(),
        "COLDSTART_VARIANTS matches no variant"
    );

    // Each build once, unoptimized; `default` gets worker-build's own wasm-opt instead
    let mut raw = Vec::new();
    let mut dirs = Vec::new();
    for variant in &selected {
        let dir = out.join(variant.name.replace(' ', "_").trim_start_matches('-'));
        let _ = fs::remove_dir_all(&dir);
        match variant.wasm_opt {
            None => worker_build(&example, &target, variant.build, &dir, true),
            Some(args) => {
                let unoptimized = out.join(format!("raw-{}", variant.build.name));
                if !raw.contains(&variant.build.name) {
                    let _ = fs::remove_dir_all(&unoptimized);
                    worker_build(&example, &target, variant.build, &unoptimized, false);
                    raw.push(variant.build.name);
                }
                copy_dir(&unoptimized, &dir);
                let wasm = dir.join("index_bg.wasm");
                let optimized = dir.join("index_bg.opt.wasm");
                let status = Command::new(wasm_opt())
                    .arg(&wasm)
                    .arg("-o")
                    .arg(&optimized)
                    .args(args)
                    .args(WASM_FEATURES)
                    .status()
                    .expect("failed to run wasm-opt");
                assert!(status.success(), "wasm-opt failed for {}", variant.name);
                fs::rename(&optimized, &wasm).unwrap();
            }
        }
        dirs.push(format!("{}={}", variant.name, dir.display()));
    }

    let harness = bench.join("coldstart");
    if !harness.join("node_modules").exists() {
        npm(&harness, &["ci", "--no-audit", "--no-fund"]);
    }
    let status = Command::new("node")
        .arg("coldstart.mjs")
        .args([
            "--samples",
            &samples.to_string(),
            "--warm",
            &warm.to_string(),
        ])
        .arg("--json")
        .arg(out.join("results.json"))
        .args(&dirs)
        .current_dir(&harness)
        .status()
        .expect("failed to run node");
    assert!(status.success(), "coldstart.mjs failed");
}
