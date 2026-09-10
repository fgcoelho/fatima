use std::{env, fs, io, path::Path, process::Command};

use inquire::Select;
use time::OffsetDateTime;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

const WASM_ASSET: &str = "wasm";
const RELEASE_DIR: &str = ".release";
const RAW_DIR: &str = ".release/raw";
const DOCKER_CACHE_DIR: &str = "target/fatima-build-cache";

const VERSION_FILES: &[&str] = &["packages/js/package.json"];
const COMMIT_TAG_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "packages/js/package.json",
    "CHANGELOG.md",
];
const RELEASE_PACKAGES: &[ReleasePackage] = &[
    ReleasePackage {
        name: "fatima-core",
        path: "crates/core",
    },
    ReleasePackage {
        name: "fatima-wasm",
        path: "crates/wasm",
    },
    ReleasePackage {
        name: "fatima-cli",
        path: "crates/cli",
    },
    ReleasePackage {
        name: "@fatima.dev/js",
        path: "packages/js",
    },
];
const STATIC_ASSETS: &[StaticAsset] = &[
    StaticAsset {
        path: "static/install.sh",
        content_type: "text/x-shellscript; charset=utf-8",
        versioned_key: "install/v{version}/install.sh",
        alias_keys: &[
            "install",
            "install.sh",
            "install/latest",
            "install/latest/install.sh",
        ],
    },
    StaticAsset {
        path: "static/install.ps1",
        content_type: "text/plain; charset=utf-8",
        versioned_key: "install/v{version}/install.ps1",
        alias_keys: &["install.ps1", "install/latest/install.ps1"],
    },
    StaticAsset {
        path: "static/schema.json",
        content_type: "application/schema+json",
        versioned_key: "schema/v{version}/schema.json",
        alias_keys: &["schema.json", "schema/latest/schema.json"],
    },
];

#[derive(Clone, Copy)]
struct ReleasePackage {
    name: &'static str,
    path: &'static str,
}

#[derive(Clone, Copy)]
struct StaticAsset {
    path: &'static str,
    content_type: &'static str,
    versioned_key: &'static str,
    alias_keys: &'static [&'static str],
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UploadPolicy {
    Missing,
    Force,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AliasPolicy {
    Never,
    Always,
    IfLatest,
}

struct UploadStaticOptions {
    tag: Option<String>,
    version: Option<String>,
    bucket: Option<String>,
    endpoint: Option<String>,
    versioned_policy: UploadPolicy,
    alias_policy: AliasPolicy,
    check_release: bool,
    dry_run: bool,
}

#[derive(Clone, Copy, Debug)]
enum BumpKind {
    Patch,
    Minor,
    Major,
}

impl std::fmt::Display for BumpKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            BumpKind::Patch => "patch",
            BumpKind::Minor => "minor",
            BumpKind::Major => "major",
        })
    }
}

#[derive(Clone, Copy)]
struct Target {
    asset: &'static str,
    triple: &'static str,
    binary: &'static str,
    archive: Archive,
    docker_builder: DockerBuilder,
}

#[derive(Clone, Copy)]
enum BuildTarget {
    Native(Target),
    Wasm,
}

#[derive(Clone, Copy)]
enum Archive {
    TarGz,
    Zip,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DockerBuilder {
    Zigbuild,
    Xwin,
}

const TARGETS: &[Target] = &[
    Target {
        asset: "linux-x64",
        triple: "x86_64-unknown-linux-gnu",
        binary: "fatima",
        archive: Archive::TarGz,
        docker_builder: DockerBuilder::Zigbuild,
    },
    Target {
        asset: "linux-arm64",
        triple: "aarch64-unknown-linux-gnu",
        binary: "fatima",
        archive: Archive::TarGz,
        docker_builder: DockerBuilder::Zigbuild,
    },
    Target {
        asset: "linux-x64-musl",
        triple: "x86_64-unknown-linux-musl",
        binary: "fatima",
        archive: Archive::TarGz,
        docker_builder: DockerBuilder::Zigbuild,
    },
    Target {
        asset: "linux-arm64-musl",
        triple: "aarch64-unknown-linux-musl",
        binary: "fatima",
        archive: Archive::TarGz,
        docker_builder: DockerBuilder::Zigbuild,
    },
    Target {
        asset: "darwin-x64",
        triple: "x86_64-apple-darwin",
        binary: "fatima",
        archive: Archive::Zip,
        docker_builder: DockerBuilder::Zigbuild,
    },
    Target {
        asset: "darwin-arm64",
        triple: "aarch64-apple-darwin",
        binary: "fatima",
        archive: Archive::Zip,
        docker_builder: DockerBuilder::Zigbuild,
    },
    Target {
        asset: "windows-x64",
        triple: "x86_64-pc-windows-msvc",
        binary: "fatima.exe",
        archive: Archive::Zip,
        docker_builder: DockerBuilder::Xwin,
    },
    Target {
        asset: "windows-arm64",
        triple: "aarch64-pc-windows-msvc",
        binary: "fatima.exe",
        archive: Archive::Zip,
        docker_builder: DockerBuilder::Xwin,
    },
];

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("failed to resolve workspace root")?;
    env::set_current_dir(workspace).map_err(|error| error.to_string())?;

    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        usage();
        return Err("missing command".into());
    };

    match command.as_str() {
        "build-binaries" => build_binaries(args.collect()),
        "set-version" => {
            let version = parse_version_arg(args.collect())?;
            set_version(&version)
        }
        "bump" => bump(),
        "revert-bump" => revert_bump(),
        "commit-release" => commit_release(),
        "verify-release" => {
            let version = parse_version_arg(args.collect())?;
            verify_release(&version)
        }
        "upload-static" => upload_static(args.collect()),
        "help" | "--help" | "-h" => {
            usage();
            Ok(())
        }
        _ => Err(format!("unknown command `{command}`")),
    }
}

fn usage() {
    eprintln!(
        "Usage:\n  cargo run -p task -- bump\n  cargo run -p task -- revert-bump\n  cargo run -p task -- commit-release\n  cargo run -p task -- build-binaries [--target <asset>|--host|--all]\n  cargo run -p task -- set-version --version <version>\n  cargo run -p task -- verify-release --version <version>\n  cargo run -p task -- upload-static (--version <version>|--tag <tag>) [--versioned-policy missing|force] [--aliases never|always|if-latest] [--check-release] [--dry-run]\n\nTargets: linux-x64, linux-arm64, linux-x64-musl, linux-arm64-musl, darwin-x64, darwin-arm64, windows-x64, windows-arm64, wasm"
    );
}

fn parse_version_arg(args: Vec<String>) -> Result<String, String> {
    let mut iter = args.into_iter();
    match iter.next() {
        None => Err("missing version".into()),
        Some(arg) => match arg.as_str() {
            "--version" | "-v" => iter.next().ok_or("--version requires a value".into()),
            value if !value.starts_with('-') => Ok(value.to_string()),
            _ => Err(format!("unknown argument `{arg}`")),
        },
    }
}

fn build_binaries(args: Vec<String>) -> Result<(), String> {
    let selected = parse_targets(args)?;
    if Path::new(RELEASE_DIR).exists() {
        fs::remove_dir_all(RELEASE_DIR).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(RAW_DIR).map_err(|error| error.to_string())?;

    for target in selected {
        match target {
            BuildTarget::Native(target) => build_native_binary(target)?,
            BuildTarget::Wasm => build_wasm_binary()?,
        }
    }

    Ok(())
}

fn build_native_binary(target: Target) -> Result<(), String> {
    println!("building fatima for {} ({})", target.asset, target.triple);
    build_target(target)?;

    let raw_dir = Path::new(RAW_DIR).join(target.asset);
    if raw_dir.exists() {
        fs::remove_dir_all(&raw_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&raw_dir).map_err(|error| error.to_string())?;

    let built = Path::new("target")
        .join(target.triple)
        .join("release")
        .join(target.binary);
    let raw_binary = raw_dir.join(target.binary);
    fs::copy(&built, &raw_binary).map_err(|error| {
        format!(
            "failed to copy `{}` to `{}`: {error}",
            built.display(),
            raw_binary.display()
        )
    })?;

    package_target(target, &raw_dir)
}

fn build_wasm_binary() -> Result<(), String> {
    println!("building fatima for wasm");

    let raw_dir = Path::new(RAW_DIR).join(WASM_ASSET);
    let wasm_pack_dir = Path::new(RAW_DIR).join("wasm-pack");
    if raw_dir.exists() {
        fs::remove_dir_all(&raw_dir).map_err(|error| error.to_string())?;
    }
    if wasm_pack_dir.exists() {
        fs::remove_dir_all(&wasm_pack_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&raw_dir).map_err(|error| error.to_string())?;
    fs::create_dir_all(&wasm_pack_dir).map_err(|error| error.to_string())?;

    run_command(
        "wasm-pack",
        &[
            "build",
            "crates/wasm",
            "--target",
            "web",
            "--out-dir",
            "../../.release/raw/wasm-pack",
            "--out-name",
            "fatima_wasm",
        ],
    )?;

    fs::copy(
        wasm_pack_dir.join("fatima_wasm_bg.wasm"),
        raw_dir.join("fatima_wasm_bg.wasm"),
    )
    .map_err(|error| error.to_string())?;

    package_wasm(&raw_dir)
}

fn build_target(target: Target) -> Result<(), String> {
    if should_use_docker(target)? {
        return build_target_with_docker(target);
    }

    run_command("rustup", &["target", "add", target.triple])?;
    run_command(
        "cargo",
        &[
            "build",
            "--release",
            "-p",
            "fatima-cli",
            "--bin",
            "fatima",
            "--target",
            target.triple,
        ],
    )
}

fn should_use_docker(target: Target) -> Result<bool, String> {
    match env::var("FATIMA_BINARY_BUILDER").as_deref() {
        Ok("cargo") => return Ok(false),
        Ok("docker") => return Ok(true),
        Ok(value) => return Err(format!("unknown FATIMA_BINARY_BUILDER `{value}`")),
        Err(_) => {}
    }

    if env::consts::OS == "linux" && !is_current_host_target(target) {
        if command_exists("docker") {
            return Ok(true);
        }
        return Err(format!(
            "cannot build {} from Linux with plain cargo. Install Docker or set FATIMA_BINARY_BUILDER=cargo and provide the target linker yourself. Rust still needs a cross linker/toolchain.",
            target.asset
        ));
    }

    Ok(false)
}

fn build_target_with_docker(target: Target) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let workspace = cwd.to_string_lossy().into_owned();
    let user = docker_user_flag();
    let cache_root = Path::new(DOCKER_CACHE_DIR);
    let cargo_registry = cache_root.join("cargo-registry");
    let cargo_git = cache_root.join("cargo-git");
    let zigbuild_cache = cache_root.join("cargo-zigbuild");
    let xwin_cache = cache_root.join("xwin");
    for dir in [&cargo_registry, &cargo_git, &zigbuild_cache, &xwin_cache] {
        fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }

    let command = match target.docker_builder {
        DockerBuilder::Zigbuild => format!(
            "export PATH=/usr/local/cargo/bin:$PATH; cargo zigbuild --release -p fatima-cli --bin fatima --target {triple}",
            triple = target.triple
        ),
        DockerBuilder::Xwin => format!(
            "export PATH=/usr/local/cargo/bin:$PATH; XWIN_CROSS_COMPILER=clang cargo xwin build --release -p fatima-cli --bin fatima --target {triple}",
            triple = target.triple
        ),
    };
    let image = match target.docker_builder {
        DockerBuilder::Zigbuild => "ghcr.io/rust-cross/cargo-zigbuild",
        DockerBuilder::Xwin => "messense/cargo-xwin",
    };

    let mut args = vec!["run", "--rm"];
    let volume = format!("{workspace}:/io");
    let cargo_registry_volume = docker_volume(&cargo_registry, "/usr/local/cargo/registry")?;
    let cargo_git_volume = docker_volume(&cargo_git, "/usr/local/cargo/git")?;
    let zigbuild_cache_volume = docker_volume(&zigbuild_cache, "/tmp/cargo-zigbuild")?;
    let xwin_cache_volume = docker_volume(&xwin_cache, "/tmp/xwin")?;
    if let Some(user) = user.as_deref() {
        args.extend(["--user", user]);
    }
    args.extend([
        "-e",
        "HOME=/tmp",
        "-e",
        "CARGO_ZIGBUILD_CACHE_DIR=/tmp/cargo-zigbuild",
        "-e",
        "XWIN_CACHE_DIR=/tmp/xwin",
        "-v",
        volume.as_str(),
        "-v",
        cargo_registry_volume.as_str(),
        "-v",
        cargo_git_volume.as_str(),
        "-v",
        zigbuild_cache_volume.as_str(),
        "-v",
        xwin_cache_volume.as_str(),
        "-w",
        "/io",
        image,
        "sh",
        "-lc",
        command.as_str(),
    ]);

    run_command("docker", &args)
}

fn docker_volume(host: &Path, container: &str) -> Result<String, String> {
    let host = host
        .canonicalize()
        .map_err(|error| format!("failed to resolve `{}`: {error}", host.display()))?;
    Ok(format!("{}:{container}", host.to_string_lossy()))
}

fn parse_targets(args: Vec<String>) -> Result<Vec<BuildTarget>, String> {
    if args.is_empty() {
        return Ok(all_targets());
    }

    let mut iter = args.into_iter();
    let mut targets = Vec::new();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--all" => return Ok(all_targets()),
            "--host" => {
                targets.push(BuildTarget::Native(current_host_target().ok_or(
                    "could not infer current host target; use --target <asset> or --all",
                )?));
            }
            "--target" | "-t" => {
                let asset = iter.next().ok_or("--target requires a value")?;
                targets.push(find_target(&asset)?);
            }
            value if !value.starts_with('-') => targets.push(find_target(value)?),
            _ => return Err(format!("unknown argument `{arg}`")),
        }
    }
    Ok(targets)
}

fn all_targets() -> Vec<BuildTarget> {
    TARGETS
        .iter()
        .copied()
        .map(BuildTarget::Native)
        .chain(std::iter::once(BuildTarget::Wasm))
        .collect()
}

fn current_host_target() -> Option<Target> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let asset = match (os, arch) {
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", "x86_64") => "darwin-x64",
        ("macos", "aarch64") => "darwin-arm64",
        ("windows", "x86_64") => "windows-x64",
        ("windows", "aarch64") => "windows-arm64",
        _ => return None,
    };
    TARGETS.iter().copied().find(|target| target.asset == asset)
}

fn is_current_host_target(target: Target) -> bool {
    current_host_target()
        .map(|host| host.asset == target.asset)
        .unwrap_or(false)
}

fn find_target(asset: &str) -> Result<BuildTarget, String> {
    if asset == WASM_ASSET {
        return Ok(BuildTarget::Wasm);
    }

    TARGETS
        .iter()
        .copied()
        .find(|target| target.asset == asset)
        .map(BuildTarget::Native)
        .ok_or_else(|| format!("unknown target `{asset}`"))
}

fn package_target(target: Target, raw_dir: &Path) -> Result<(), String> {
    let archive_name = match target.archive {
        Archive::TarGz => format!("fatima-{}.tar.gz", target.asset),
        Archive::Zip => format!("fatima-{}.zip", target.asset),
    };
    let archive = Path::new(RELEASE_DIR).join(archive_name);
    remove_file_if_exists(&archive).map_err(|error| error.to_string())?;

    match target.archive {
        Archive::TarGz => run_command_in(
            raw_dir,
            "tar",
            &["-czf", archive_path(&archive)?.as_str(), target.binary],
        ),
        Archive::Zip => {
            write_zip(&archive, &raw_dir.join(target.binary), target.binary)?;
            Ok(())
        }
    }?;

    ensure_archive_created(&archive)
}

fn write_zip(archive: &Path, source: &Path, name: &str) -> Result<(), String> {
    let file = fs::File::create(archive)
        .map_err(|error| format!("failed to create `{}`: {error}", archive.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);
    zip.start_file(name, options)
        .map_err(|error| error.to_string())?;

    let mut source = fs::File::open(source)
        .map_err(|error| format!("failed to open `{}`: {error}", source.display()))?;
    io::copy(&mut source, &mut zip).map_err(|error| error.to_string())?;
    zip.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn package_wasm(raw_dir: &Path) -> Result<(), String> {
    let archive = Path::new(RELEASE_DIR).join("fatima-wasm.zip");
    remove_file_if_exists(&archive).map_err(|error| error.to_string())?;
    write_zip(
        &archive,
        &raw_dir.join("fatima_wasm_bg.wasm"),
        "fatima_wasm_bg.wasm",
    )?;

    ensure_archive_created(&archive)
}

fn ensure_archive_created(archive: &Path) -> Result<(), String> {
    let metadata = archive
        .metadata()
        .map_err(|error| format!("archive was not created `{}`: {error}", archive.display()))?;
    if metadata.len() == 0 {
        return Err(format!("archive is empty `{}`", archive.display()));
    }
    println!("created {} ({} bytes)", archive.display(), metadata.len());
    Ok(())
}

fn archive_path(path: &Path) -> Result<String, String> {
    path.canonicalize()
        .or_else(|_| Ok::<_, io::Error>(env::current_dir()?.join(path)))
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| error.to_string())
}

fn set_version(version: &str) -> Result<(), String> {
    replace_in_file("Cargo.toml", |content| {
        content
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("version = ") {
                    format!("version = \"{version}\"")
                } else if line.trim_start().starts_with("fatima-core = ") {
                    format!("fatima-core = {{ path = \"crates/core\", version = \"{version}\", default-features = false }}")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    })?;

    for file in VERSION_FILES {
        replace_in_file(file, |content| replace_json_version(&content, version))?;
    }

    Ok(())
}

fn replace_json_version(content: &str, version: &str) -> String {
    content
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("\"version\":") {
                let indent = &line[..line.len() - trimmed.len()];
                let comma = if trimmed.ends_with(',') { "," } else { "" };
                format!("{indent}\"version\": \"{version}\"{comma}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn replace_in_file(path: &str, replace: impl FnOnce(String) -> String) -> Result<(), String> {
    let content =
        fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let next = replace(content);
    fs::write(path, next).map_err(|error| format!("failed to write {path}: {error}"))
}

fn bump() -> Result<(), String> {
    ensure_clean_release_files()?;

    let kind = Select::new(
        "Select version bump",
        vec![BumpKind::Patch, BumpKind::Minor, BumpKind::Major],
    )
    .with_help_message("↑↓ to move, enter to select")
    .without_filtering()
    .prompt()
    .map_err(|error| error.to_string())?;

    let current = current_version()?;
    let latest = latest_release_tag()?;
    let base = latest
        .as_deref()
        .and_then(|tag| tag.strip_prefix('v'))
        .unwrap_or("0.0.0");
    let next = bump_version(base, kind)?;
    let tag = format!("v{next}");
    if git_tag_exists(&tag)? {
        return Err(format!("tag `{tag}` already exists"));
    }

    let changed = match latest.as_deref() {
        Some(previous) => changed_release_packages(previous)?,
        None => Vec::new(),
    };
    if latest.is_some() && changed.is_empty() {
        return Err("no versioned package changes found since the last release".into());
    }

    set_version(&next)?;
    refresh_cargo_lock()?;
    if latest.is_some() {
        update_changelog(&next, &changed)?;
    }

    run_command("git", &["tag", &tag])?;
    println!("bumped {current} -> {next}");
    println!("created local tag {tag}");
    Ok(())
}

fn current_version() -> Result<String, String> {
    let cargo = fs::read_to_string("Cargo.toml").map_err(|error| error.to_string())?;
    let mut in_workspace_package = false;
    for line in cargo.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace_package = trimmed == "[workspace.package]";
            continue;
        }
        if in_workspace_package && trimmed.starts_with("version = ") {
            return trimmed
                .split_once('"')
                .and_then(|(_, rest)| rest.split_once('"'))
                .map(|(version, _)| version.to_string())
                .ok_or_else(|| "failed to parse workspace version".to_string());
        }
    }
    Err("missing workspace package version".into())
}

fn bump_version(current: &str, kind: BumpKind) -> Result<String, String> {
    let mut parts = current.split('.');
    let major = parts
        .next()
        .ok_or("missing major version")?
        .parse::<u64>()
        .map_err(|_| format!("invalid major version in `{current}`"))?;
    let minor = parts
        .next()
        .ok_or("missing minor version")?
        .parse::<u64>()
        .map_err(|_| format!("invalid minor version in `{current}`"))?;
    let patch = parts
        .next()
        .ok_or("missing patch version")?
        .parse::<u64>()
        .map_err(|_| format!("invalid patch version in `{current}`"))?;
    if parts.next().is_some() {
        return Err(format!("unsupported version `{current}`"));
    }

    let (major, minor, patch) = match kind {
        BumpKind::Patch => (major, minor, patch + 1),
        BumpKind::Minor => (major, minor + 1, 0),
        BumpKind::Major => (major + 1, 0, 0),
    };
    Ok(format!("{major}.{minor}.{patch}"))
}

fn latest_release_tag() -> Result<Option<String>, String> {
    let output = command_output("git", &["tag", "--list", "v[0-9]*", "--sort=-v:refname"])?;
    Ok(output.lines().next().map(str::to_string))
}

fn release_tags() -> Result<Vec<String>, String> {
    let output = command_output("git", &["tag", "--list", "v[0-9]*", "--sort=-v:refname"])?;
    Ok(output.lines().map(str::to_string).collect())
}

fn git_tag_exists(tag: &str) -> Result<bool, String> {
    let output = command_output("git", &["tag", "--list", tag])?;
    Ok(output.lines().any(|line| line == tag))
}

fn changed_release_packages(tag: &str) -> Result<Vec<ReleasePackage>, String> {
    let mut changed = Vec::new();
    for package in RELEASE_PACKAGES {
        let output = command_output(
            "git",
            &["diff", "--name-only", tag, "HEAD", "--", package.path],
        )?;
        if !output.trim().is_empty() {
            changed.push(*package);
        }
    }
    Ok(changed)
}

fn refresh_cargo_lock() -> Result<(), String> {
    command_output("cargo", &["metadata", "--format-version", "1", "--no-deps"]).map(|_| ())
}

fn update_changelog(version: &str, packages: &[ReleasePackage]) -> Result<(), String> {
    let path = Path::new("CHANGELOG.md");
    let existing = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => "# Changelog\n".to_string(),
        Err(error) => return Err(error.to_string()),
    };
    let date = today();
    let package_lines = packages
        .iter()
        .map(|package| format!("- {}", package.name))
        .collect::<Vec<_>>()
        .join("\n");
    let entry = format!("## v{version} - {date}\n\n### Changed packages\n\n{package_lines}\n");

    let next = if let Some(rest) = existing.strip_prefix("# Changelog\n") {
        format!("# Changelog\n\n{entry}\n{}", rest.trim_start())
    } else {
        format!("# Changelog\n\n{entry}\n{}", existing.trim_start())
    };
    fs::write(path, next).map_err(|error| error.to_string())
}

fn remove_changelog_entry(version: &str) -> Result<(), String> {
    let path = Path::new("CHANGELOG.md");
    let Ok(existing) = fs::read_to_string(path) else {
        return Ok(());
    };

    let heading = format!("## v{version}");
    let Some(start) = existing.find(&heading) else {
        return Ok(());
    };
    let end = existing[start + heading.len()..]
        .find("\n## ")
        .map(|offset| start + heading.len() + offset + 1)
        .unwrap_or(existing.len());

    let mut next = format!("{}{}", &existing[..start], &existing[end..]);
    while next.contains("\n\n\n") {
        next = next.replace("\n\n\n", "\n\n");
    }
    if next.trim() == "# Changelog" {
        if !git_file_tracked("CHANGELOG.md") {
            return fs::remove_file(path).map_err(|error| error.to_string());
        }
        next = "# Changelog\n".to_string();
    }

    fs::write(path, next).map_err(|error| error.to_string())
}

fn git_file_tracked(path: &str) -> bool {
    Command::new("git")
        .args(["ls-files", "--error-unmatch", path])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn today() -> String {
    let date = OffsetDateTime::now_utc().date();
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

fn ensure_clean_release_files() -> Result<(), String> {
    let mut dirty = Vec::new();
    for file in COMMIT_TAG_FILES {
        if !Path::new(file).exists() {
            continue;
        }
        let output = command_output("git", &["status", "--porcelain", "--", file])?;
        if !output.trim().is_empty() {
            dirty.push(*file);
        }
    }
    if dirty.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "release files have uncommitted changes; commit or stash before bumping: {}",
            dirty.join(", ")
        ))
    }
}

fn revert_bump() -> Result<(), String> {
    let current = current_version()?;
    let tag = format!("v{current}");
    let tags = release_tags()?;
    let Some(index) = tags.iter().position(|candidate| candidate == &tag) else {
        return Err(format!("missing local bump tag `{tag}`"));
    };
    let previous = tags
        .get(index + 1)
        .and_then(|tag| tag.strip_prefix('v'))
        .unwrap_or("0.0.0")
        .to_string();

    run_command("git", &["tag", "-d", &tag])?;
    set_version(&previous)?;
    refresh_cargo_lock()?;
    remove_changelog_entry(&current)?;

    println!("deleted local tag {tag}");
    println!("reverted {current} -> {previous}");
    Ok(())
}

fn commit_release() -> Result<(), String> {
    let version = current_version()?;
    let tag = format!("v{version}");
    verify_release(&version)?;
    if !git_tag_exists(&tag)? {
        return Err(format!(
            "missing local tag `{tag}`; run `cargo run -p task -- bump` first"
        ));
    }

    let existing_files = COMMIT_TAG_FILES
        .iter()
        .copied()
        .filter(|file| Path::new(file).exists())
        .collect::<Vec<_>>();
    let has_changes = existing_files.iter().try_fold(false, |has_changes, file| {
        let output = command_output("git", &["status", "--porcelain", "--", file])?;
        Ok::<_, String>(has_changes || !output.trim().is_empty())
    })?;

    if has_changes {
        let mut add_args = vec!["add"];
        add_args.extend(existing_files.iter().copied());
        run_command("git", &add_args)?;
        run_command("git", &["commit", "-m", &format!("release: {tag}")])?;
    } else {
        println!("no release file changes to commit");
    }

    run_command("git", &["tag", "-f", &tag])?;
    run_command("git", &["push"])?;
    run_command("git", &["push", "origin", &tag])?;
    Ok(())
}

fn verify_release(version: &str) -> Result<(), String> {
    let cargo = fs::read_to_string("Cargo.toml").map_err(|error| error.to_string())?;
    ensure_contains(
        &cargo,
        &format!("version = \"{version}\""),
        "Cargo workspace version",
    )?;
    ensure_contains(
        &cargo,
        &format!(
            "fatima-core = {{ path = \"crates/core\", version = \"{version}\", default-features = false }}"
        ),
        "fatima-core workspace dependency version",
    )?;

    for file in VERSION_FILES {
        let content = fs::read_to_string(file).map_err(|error| error.to_string())?;
        ensure_contains(&content, "\"name\": \"@fatima.dev/js\"", file)?;
        ensure_contains(&content, "\"license\": \"MIT\"", file)?;
        ensure_contains(
            &content,
            &format!("\"version\": \"{version}\""),
            &format!("{file} version"),
        )?;
    }

    for file in [
        "crates/core/Cargo.toml",
        "crates/cli/Cargo.toml",
        "crates/wasm/Cargo.toml",
    ] {
        let content = fs::read_to_string(file).map_err(|error| error.to_string())?;
        for needle in [
            "version.workspace = true",
            "edition.workspace = true",
            "license.workspace = true",
            "repository.workspace = true",
            "description.workspace = true",
        ] {
            ensure_contains(&content, needle, file)?;
        }
    }

    let wasm = fs::read_to_string("crates/wasm/Cargo.toml").map_err(|error| error.to_string())?;
    ensure_contains(
        &wasm,
        "fatima-core = { workspace = true, features = [\"wasm\"] }",
        "crates/wasm fatima-core dependency",
    )?;

    let cli = fs::read_to_string("crates/cli/Cargo.toml").map_err(|error| error.to_string())?;
    ensure_contains(
        &cli,
        "fatima-core = { workspace = true, features = [\"native\"] }",
        "crates/cli fatima-core dependency",
    )?;

    let task = fs::read_to_string("crates/task/Cargo.toml").map_err(|error| error.to_string())?;
    for needle in [
        "edition.workspace = true",
        "license.workspace = true",
        "repository.workspace = true",
        "publish = false",
    ] {
        ensure_contains(&task, needle, "crates/task/Cargo.toml")?;
    }
    ensure_package_key_absent(&task, "version", "crates/task/Cargo.toml")?;
    ensure_package_key_absent(&task, "description", "crates/task/Cargo.toml")?;

    let web = fs::read_to_string("packages/web/package.json").map_err(|error| error.to_string())?;
    ensure_contains(&web, "\"name\": \"@fatimajs/web\"", "packages/web name")?;
    ensure_contains(&web, "\"private\": true", "packages/web private flag")?;
    ensure_not_contains(&web, "\"version\"", "packages/web version")?;

    let root_package = fs::read_to_string("package.json").map_err(|error| error.to_string())?;
    ensure_contains(
        &root_package,
        "\"private\": true",
        "root package private flag",
    )?;
    ensure_not_contains(&root_package, "\n\t\"version\":", "root package version")?;

    let lock = fs::read_to_string("Cargo.lock").map_err(|error| error.to_string())?;
    for package in ["fatima-core", "fatima-cli", "fatima-wasm"] {
        ensure_lock_package_version(&lock, package, version)?;
    }
    ensure_lock_package_version(&lock, "task", "0.0.0")?;

    Ok(())
}

fn upload_static(args: Vec<String>) -> Result<(), String> {
    let options = parse_upload_static_args(args)?;
    let tag = match (&options.tag, &options.version) {
        (Some(tag), None) => tag.clone(),
        (None, Some(version)) => format!("v{}", normalize_version(version)),
        (Some(_), Some(_)) => return Err("pass only one of --tag or --version".into()),
        (None, None) => return Err("missing --tag or --version".into()),
    };
    if !tag.starts_with('v') {
        return Err(format!("tag must start with v, got: {tag}"));
    }
    let version = tag.trim_start_matches('v').to_string();
    if version.is_empty() {
        return Err("version is empty".into());
    }

    if options.check_release && !options.dry_run {
        run_command("gh", &["release", "view", &tag])?;
    }

    let bucket = options
        .bucket
        .or_else(|| env::var("R2_BUCKET").ok())
        .ok_or("missing --bucket or R2_BUCKET")?;
    let endpoint = options
        .endpoint
        .or_else(|| env::var("R2_ENDPOINT").ok())
        .ok_or("missing --endpoint or R2_ENDPOINT")?;

    for asset in STATIC_ASSETS {
        if !Path::new(asset.path).is_file() {
            return Err(format!("missing static asset `{}`", asset.path));
        }

        let key = asset.versioned_key.replace("{version}", &version);
        upload_static_asset(
            &bucket,
            &endpoint,
            &key,
            asset,
            options.versioned_policy,
            options.dry_run,
        )?;
    }

    if should_upload_aliases(options.alias_policy, &tag, options.dry_run)? {
        for asset in STATIC_ASSETS {
            for key in asset.alias_keys {
                upload_static_asset(
                    &bucket,
                    &endpoint,
                    key,
                    asset,
                    UploadPolicy::Force,
                    options.dry_run,
                )?;
            }
        }
    }

    Ok(())
}

fn parse_upload_static_args(args: Vec<String>) -> Result<UploadStaticOptions, String> {
    let mut options = UploadStaticOptions {
        tag: None,
        version: None,
        bucket: None,
        endpoint: None,
        versioned_policy: UploadPolicy::Missing,
        alias_policy: AliasPolicy::Never,
        check_release: false,
        dry_run: false,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tag" => options.tag = Some(iter.next().ok_or("--tag requires a value")?),
            "--version" | "-v" => {
                options.version = Some(iter.next().ok_or("--version requires a value")?)
            }
            "--bucket" => options.bucket = Some(iter.next().ok_or("--bucket requires a value")?),
            "--endpoint" => {
                options.endpoint = Some(iter.next().ok_or("--endpoint requires a value")?)
            }
            "--versioned-policy" => {
                let value = iter.next().ok_or("--versioned-policy requires a value")?;
                options.versioned_policy = match value.as_str() {
                    "missing" => UploadPolicy::Missing,
                    "force" => UploadPolicy::Force,
                    _ => return Err(format!("unknown versioned policy `{value}`")),
                };
            }
            "--aliases" => {
                let value = iter.next().ok_or("--aliases requires a value")?;
                options.alias_policy = match value.as_str() {
                    "never" => AliasPolicy::Never,
                    "always" => AliasPolicy::Always,
                    "if-latest" => AliasPolicy::IfLatest,
                    _ => return Err(format!("unknown alias policy `{value}`")),
                };
            }
            "--check-release" => options.check_release = true,
            "--dry-run" => options.dry_run = true,
            _ => return Err(format!("unknown argument `{arg}`")),
        }
    }
    Ok(options)
}

fn normalize_version(version: &str) -> String {
    version
        .trim_start_matches("fatima@")
        .trim_start_matches('v')
        .to_string()
}

fn should_upload_aliases(policy: AliasPolicy, tag: &str, dry_run: bool) -> Result<bool, String> {
    match policy {
        AliasPolicy::Never => Ok(false),
        AliasPolicy::Always => Ok(true),
        AliasPolicy::IfLatest => {
            if dry_run {
                println!("dry-run: assuming {tag} is latest for alias upload");
                return Ok(true);
            }
            let latest = command_output(
                "gh",
                &[
                    "release",
                    "list",
                    "--limit",
                    "1",
                    "--json",
                    "tagName",
                    "--jq",
                    ".[0].tagName",
                ],
            )?;
            if tag == latest {
                println!("{tag} is latest release; uploading aliases");
                Ok(true)
            } else {
                println!("{tag} is not latest release ({latest}); leaving aliases unchanged");
                Ok(false)
            }
        }
    }
}

fn upload_static_asset(
    bucket: &str,
    endpoint: &str,
    key: &str,
    asset: &StaticAsset,
    policy: UploadPolicy,
    dry_run: bool,
) -> Result<(), String> {
    if dry_run {
        println!(
            "dry-run: upload {} to {} ({})",
            asset.path, key, asset.content_type
        );
        return Ok(());
    }

    if policy == UploadPolicy::Missing && s3_object_exists(bucket, endpoint, key)? {
        println!("{key} already exists in bucket, skipping");
        return Ok(());
    }

    println!("Uploading {} to {key}", asset.path);
    run_dynamic_command(
        "aws",
        &[
            "s3api",
            "put-object",
            "--bucket",
            bucket,
            "--key",
            key,
            "--body",
            asset.path,
            "--content-type",
            asset.content_type,
            "--endpoint-url",
            endpoint,
        ],
    )
}

fn s3_object_exists(bucket: &str, endpoint: &str, key: &str) -> Result<bool, String> {
    let status = Command::new("aws")
        .args([
            "s3api",
            "head-object",
            "--bucket",
            bucket,
            "--key",
            key,
            "--endpoint-url",
            endpoint,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|error| format!("failed to run aws: {error}"))?;
    Ok(status.success())
}

fn ensure_contains(content: &str, needle: &str, label: &str) -> Result<(), String> {
    if content.contains(needle) {
        Ok(())
    } else {
        Err(format!("{label} is not synchronized; missing `{needle}`"))
    }
}

fn ensure_not_contains(content: &str, needle: &str, label: &str) -> Result<(), String> {
    if content.contains(needle) {
        Err(format!("{label} should not contain `{needle}`"))
    } else {
        Ok(())
    }
}

fn ensure_package_key_absent(content: &str, key: &str, label: &str) -> Result<(), String> {
    let mut in_package = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if in_package
            && (trimmed.starts_with(&format!("{key} = "))
                || trimmed.starts_with(&format!("{key}.workspace = ")))
        {
            return Err(format!("{label} package section should not define `{key}`"));
        }
    }
    Ok(())
}

fn ensure_lock_package_version(content: &str, package: &str, version: &str) -> Result<(), String> {
    let expected_name = format!("name = \"{package}\"");
    let expected_version = format!("version = \"{version}\"");

    for block in content.split("\n[[package]]\n") {
        if block.contains(&expected_name) {
            return ensure_contains(block, &expected_version, &format!("Cargo.lock {package}"));
        }
    }

    Err(format!("Cargo.lock is missing package `{package}`"))
}

fn run_command(command: &str, args: &[&str]) -> Result<(), String> {
    run_command_in(Path::new("."), command, args)
}

fn run_dynamic_command(command: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|error| format!("failed to run {command}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command} exited with {status}"))
    }
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn docker_user_flag() -> Option<String> {
    if !cfg!(unix) {
        return None;
    }
    let uid = command_output("id", &["-u"]).ok()?;
    let gid = command_output("id", &["-g"]).ok()?;
    Some(format!("{uid}:{gid}"))
}

fn command_output(command: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run {command}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{command} exited with {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_command_in(cwd: &Path, command: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(command)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|error| format!("failed to run {command}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command} exited with {status}"))
    }
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
