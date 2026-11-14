//! 从应用的 Cargo 依赖启动匹配版本的 DevBox。

use serde_json::Value;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const TARGET: &str = "wasm32-unknown-emscripten";
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
    let mut args = env::args_os().skip(1).peekable();
    if args.peek().is_some_and(|arg| arg == "aeris-devbox") {
        args.next();
    }
    let config = match args.next() {
        Some(arg) if arg == "--help" || arg == "-h" => {
            println!("用法：cargo aeris-devbox [devbox.json]");
            return Ok(());
        }
        Some(arg) if arg == "--version" || arg == "-V" => {
            println!("cargo-aeris-devbox {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some(arg) => PathBuf::from(arg),
        None => PathBuf::from("devbox.json"),
    };
    if args.next().is_some() {
        return Err("用法：cargo aeris-devbox [devbox.json]".into());
    }
    let config = config.canonicalize().map_err(|error| {
        format!(
            "无法打开 {}：{error}。请在应用目录运行，或传入配置文件路径。",
            config.display()
        )
    })?;
    let project = config.parent().ok_or("配置文件需要位于应用目录。")?;
    let settings: Value = serde_json::from_slice(&fs::read(&config)?)?;
    let manifest = settings["manifest"]
        .as_str()
        .filter(|path| !path.trim().is_empty())
        .ok_or("devbox.json 需要 manifest，指向应用的 Cargo.toml。")?;
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
    let runtime = project.join("target/aeris-devbox/tool");
    prepare_runtime(&devbox, &runtime)?;
    install_web_dependencies(&runtime)?;

    let mut node = Command::new("node");
    node.current_dir(project)
        .arg(runtime.join("cli.mjs"))
        .arg(config)
        .env("AERIS_DEVBOX_SOURCE_ROOT", source)
        .env("CARGO", cargo);
    run_node(node)
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
        return Err("请在 devbox.json 选用启用了 aeris-ui/devbox 的应用 feature。".into());
    }
    Ok(dependency)
}

fn prepare_runtime(source: &Path, runtime: &Path) -> io::Result<()> {
    fs::create_dir_all(runtime)?;
    for name in RUNTIME_FILES {
        copy_file(&source.join(name), &runtime.join(name))?;
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

fn install_web_dependencies(runtime: &Path) -> Result<()> {
    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    let status = Command::new(pnpm)
        .args(["install", "--frozen-lockfile"])
        .current_dir(runtime)
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
