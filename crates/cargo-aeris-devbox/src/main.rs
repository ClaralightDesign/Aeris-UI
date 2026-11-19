//! 从应用的 Cargo 依赖启动匹配版本的 DevBox。

use serde_json::Value;
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const TARGET: &str = "wasm32-unknown-emscripten";
const CONFIG_NAME: &str = "aeris.config.toml";
const USAGE: &str = "用法：cargo aeris-devbox [aeris.config.toml | --demo]\n      cargo aeris-devbox agent --help";
const AGENT_CLIENT: &str = include_str!("agent-client.mjs");
const RUNTIME_FILES: &[&str] = &[
    "cli.mjs",
    "index.html",
    "package.json",
    "pnpm-lock.yaml",
    "vite.config.ts",
];
const RUNTIME_DIRS: &[&str] = &["native", "web"];

fn main() -> ExitCode {
    match launch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn launch() -> Result<()> {
    let mut args = env::args_os()
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .peekable();
    if args.peek().is_some_and(|arg| arg == "aeris-devbox") {
        args.next();
    }
    let agent_action = if args.peek().is_some_and(|arg| arg == "agent") {
        args.next();
        let starts_runtime = matches!(
            args.peek().and_then(|arg| arg.to_str()),
            Some("serve" | "install-browser")
        );
        if starts_runtime && !args.clone().any(|arg| arg == "--help" || arg == "-h") {
            args.next()
        } else {
            let mut node = Command::new("node");
            node.args(["--input-type=module", "--eval", AGENT_CLIENT, "--"])
                .args(args);
            return run_node(node);
        }
    } else {
        None
    };
    let agent = agent_action.is_some();
    let config = match args.next() {
        Some(arg) if arg == "--help" || arg == "-h" => {
            println!("{USAGE}");
            return Ok(());
        }
        Some(arg) if arg == "--version" || arg == "-V" => {
            println!("cargo-aeris-devbox {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some(arg) => PathBuf::from(arg),
        None if !Path::new(CONFIG_NAME).is_file() && Path::new("devbox.json").is_file() => {
            PathBuf::from("devbox.json")
        }
        None => PathBuf::from(CONFIG_NAME),
    };
    if args.next().is_some() {
        return Err(USAGE.into());
    }
    let demo = config == Path::new("--demo");
    let config = if demo { gallery_config()? } else { config };
    let config = config.canonicalize().map_err(|error| {
        format!(
            "无法打开 {}：{error}。请在应用目录运行，或传入配置文件路径。",
            config.display()
        )
    })?;
    let project = config.parent().ok_or("配置文件需要位于应用目录。")?;
    let contents = fs::read_to_string(&config)?;
    let settings: Value = if config.extension().is_some_and(|ext| ext == "json") {
        serde_json::from_str(&contents)?
    } else {
        let document: toml::value::Table = toml::from_str(&contents)?;
        let devbox = document
            .get("devbox")
            .and_then(toml::Value::as_table)
            .ok_or("配置文件需要 [devbox] 表。")?;
        serde_json::to_value(devbox)?
    };
    let manifest = settings["manifest"]
        .as_str()
        .filter(|path| !path.trim().is_empty())
        .ok_or("DevBox 配置需要 manifest，指向应用的 Cargo.toml。")?;
    let manifest = project.join(manifest).canonicalize()?;
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(&cargo);
    command
        .current_dir(project)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--filter-platform",
            TARGET,
            "--manifest-path",
        ])
        .arg(&manifest)
        .stderr(Stdio::inherit());
    if let Some(features) = settings.get("features") {
        let features = features
            .as_array()
            .ok_or("features 必须是字符串数组。")?
            .iter()
            .map(|value| value.as_str().ok_or("features 必须是字符串数组。"))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if !features.is_empty() {
            command.args(["--features", &features.join(",")]);
        }
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err("Cargo 无法解析应用的依赖，请检查上方输出。".into());
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)?;
    let dependency = aeris_dependency(&metadata, &manifest)?;
    let source = Path::new(
        dependency["manifest_path"]
            .as_str()
            .ok_or("无法定位 Aeris UI 的 Cargo.toml。")?,
    )
    .ancestors()
    .nth(3)
    .ok_or("无法定位 Aeris UI 源码目录。")?;
    let devbox = source.join("devbox");
    if !devbox.join("cli.mjs").is_file() {
        return Err("当前 Aeris UI 修订没有 DevBox，请更新项目的 Aeris UI 依赖。".into());
    }
    let runtime = project.join(if agent {
        if !devbox.join("agent.mjs").is_file() {
            return Err("当前 Aeris UI 修订没有 Agent CLI，请更新项目的 Aeris UI 依赖。".into());
        }
        "target/aeris-devbox/agent-tool"
    } else {
        "target/aeris-devbox/tool"
    });
    prepare_runtime(&devbox, &runtime)?;
    install_web_dependencies(&runtime, agent)?;

    let mut node = Command::new("node");
    node.current_dir(project).arg(runtime.join("cli.mjs"));
    if let Some(action) = agent_action {
        node.arg("agent").arg(action);
    }
    if demo {
        node.arg("--demo").env("AERIS_GALLERY_DEVBOX_CONFIG", config);
    } else {
        node.arg(config);
    }
    node.env("AERIS_DEVBOX_SOURCE_ROOT", source)
        .env("CARGO", cargo);
    run_node(node)
}

fn gallery_config() -> Result<PathBuf> {
    if let Some(config) = env::var_os("AERIS_GALLERY_DEVBOX_CONFIG") {
        return Ok(config.into());
    }
    let cwd = env::current_dir()?;
    cwd.ancestors()
        .flat_map(|parent| {
            [
                parent.join("examples/gallery/aeris.config.toml"),
                parent.join("Aeris-UI/examples/gallery/aeris.config.toml"),
            ]
        })
        .find(|candidate| candidate.is_file())
        .or_else(|| {
            let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/gallery/aeris.config.toml");
            bundled.is_file().then_some(bundled)
        })
        .ok_or_else(|| "找不到 examples/gallery/aeris.config.toml。请设置 AERIS_GALLERY_DEVBOX_CONFIG。".into())
}

fn aeris_dependency<'a>(metadata: &'a Value, manifest: &Path) -> Result<&'a Value> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("Cargo 没有返回包信息。")?;
    let app = packages
        .iter()
        .find(|package| {
            package["manifest_path"]
                .as_str()
                .is_some_and(|path| Path::new(path) == manifest)
        })
        .ok_or("manifest 必须指向应用包的 Cargo.toml。")?;
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("Cargo 没有返回依赖信息。")?;
    let dependencies = nodes
        .iter()
        .find(|node| node["id"] == app["id"])
        .and_then(|node| node["deps"].as_array())
        .ok_or("找不到应用的依赖。")?;

    let mut aeris = packages.iter().filter(|package| {
        package["name"] == "aeris-ui"
            && dependencies.iter().any(|dependency| {
                dependency["pkg"] == package["id"]
                    && dependency["dep_kinds"]
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind["kind"].is_null()))
            })
    });
    let dependency = aeris
        .next()
        .ok_or("应用需要直接依赖 aeris-ui，并在浏览器构建中启用它。")?;
    if aeris.next().is_some() {
        return Err("应用的浏览器构建解析到了多个 Aeris UI 版本。".into());
    }
    let enabled = nodes
        .iter()
        .find(|node| node["id"] == dependency["id"])
        .and_then(|node| node["features"].as_array())
        .is_some_and(|features| features.iter().any(|feature| feature == "devbox"));
    if !enabled {
        return Err("请在 DevBox 配置中选用启用了 aeris-ui/devbox 的应用 feature。".into());
    }
    Ok(dependency)
}

fn prepare_runtime(source: &Path, runtime: &Path) -> io::Result<()> {
    fs::create_dir_all(runtime)?;
    for name in RUNTIME_FILES {
        copy_file(&source.join(name), &runtime.join(name))?;
    }
    for name in ["pnpm-workspace.yaml", "agent.mjs", "agent-mcp.mjs"] {
        let from = source.join(name);
        let to = runtime.join(name);
        if from.is_file() {
            copy_file(&from, &to)?;
        } else if to.is_file() {
            fs::remove_file(to)?;
        }
    }
    for name in RUNTIME_DIRS {
        copy_directory(&source.join(name), &runtime.join(name))?;
    }
    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> io::Result<()> {
    let data = fs::read(source)?;
    if fs::read(destination).ok().as_deref() != Some(&data) {
        fs::write(destination, data)?;
    }
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else {
            copy_file(&entry.path(), &target)?;
        }
    }
    Ok(())
}

fn install_web_dependencies(runtime: &Path, agent: bool) -> Result<()> {
    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    let mut command = Command::new(pnpm);
    command
        .args(["install", "--frozen-lockfile", "--reporter=append-only"])
        .current_dir(runtime);
    if agent {
        let output = command
            .output()
            .map_err(|error| format!("无法运行 pnpm：{error}"))?;
        io::stderr().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        if !output.status.success() {
            return Err("安装 DevBox 网页依赖失败。".into());
        }
        return Ok(());
    }
    let status = command
        .status()
        .map_err(|error| format!("无法运行 pnpm：{error}"))?;
    if !status.success() {
        return Err("安装 DevBox 网页依赖失败。".into());
    }
    Ok(())
}

#[cfg(unix)]
fn run_node(mut command: Command) -> Result<()> {
    use std::os::unix::process::CommandExt;
    Err(format!("无法启动 Node.js：{}", command.exec()).into())
}

#[cfg(not(unix))]
fn run_node(mut command: Command) -> Result<()> {
    let status = command
        .status()
        .map_err(|error| format!("无法启动 Node.js：{error}"))?;
    std::process::exit(status.code().unwrap_or(1))
}
