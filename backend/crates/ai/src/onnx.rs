use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ort::{
    environment::{Environment, EnvironmentBuilder},
    ep,
    session::{Session, builder::GraphOptimizationLevel},
};

pub type Error = Box<dyn StdError + Send + Sync>;
pub type Results<T> = std::result::Result<T, Error>;

static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();

/// Satu environment ORT untuk seluruh proses. `ort::init().build()` hanya boleh
/// dipanggil sekali; TTS dan STT berbagi environment ini.
pub(crate) fn environment() -> Results<&'static Environment> {
    if let Some(env) = ENVIRONMENT.get() {
        return Ok(env);
    }

    let env = environment_builder()?
        .with_execution_providers([ep::CPU::default().build()])
        .build()?;

    Ok(ENVIRONMENT.get_or_init(|| env))
}

fn environment_builder() -> Results<EnvironmentBuilder> {
    if let Some(path) = std::env::var("ORT_DYLIB_PATH")
        .ok()
        .filter(|path| !path.is_empty())
    {
        return Ok(ort::init_from(&path).map_err(|e| format!("load ORT dylib {path}: {e}"))?);
    }

    for path in dylib_candidates() {
        if let Ok(builder) = ort::init_from(&path) {
            tracing::info!("ORT: loaded dylib {}", path.display());
            return Ok(builder);
        }
    }

    Err("ONNX Runtime dylib not found; set ORT_DYLIB_PATH to libonnxruntime.so".into())
}

fn dylib_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "onnxruntime.dll"
    } else if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else {
        "libonnxruntime.so"
    }
}

fn dylib_candidates() -> Vec<PathBuf> {
    let name = dylib_name();
    let mut dirs: Vec<PathBuf> = Vec::new();

    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        dirs.push(dir);
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd);
    }
    if let Ok(dir) = std::env::var("ORT_LIB_DIR") {
        dirs.push(PathBuf::from(dir));
    }

    for dir in [
        "/usr/lib",
        "/usr/local/lib",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/opt/homebrew/lib",
    ] {
        dirs.push(PathBuf::from(dir));
    }

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        // venv / uv cache yang menyertakan paket onnxruntime
        let uv = home.join(".cache/uv/archive-v0");
        if let Ok(entries) = fs::read_dir(uv) {
            for entry in entries.flatten() {
                dirs.push(entry.path().join("onnxruntime/capi"));
            }
        }
        let local = home.join(".local/lib");
        if let Ok(entries) = fs::read_dir(local) {
            for entry in entries.flatten() {
                dirs.push(entry.path().join("site-packages/onnxruntime/capi"));
            }
        }
    }

    let mut out = Vec::new();
    for dir in dirs {
        let direct = dir.join(name);
        if direct.is_file() {
            out.push(direct);
        }
        // libonnxruntime.so.1.32.0
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let file = entry.file_name();
                if file.to_string_lossy().starts_with(name) {
                    out.push(entry.path());
                }
            }
        }
    }
    out
}

/// Bangun session dari file model memakai environment bersama.
pub(crate) fn session(path: impl AsRef<Path>) -> Results<Session> {
    let mut builder = Session::builder(environment()?)?
        .with_optimization_level(opt_level())
        .map_err(|e| format!("{e}"))?;

    let threads = std::env::var("ORT_INTRA")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or_else(physical_cores);

    builder = builder
        .with_intra_threads(threads)
        .map_err(|e| format!("{e}"))?;

    let session = builder.commit_from_file(path)?;

    Ok(session)
}

/// ORT sering default ke 1 thread; pakai jumlah core fisik agar inferensi CPU cepat.
fn physical_cores() -> usize {
    let logical = std::thread::available_parallelism().map_or(1, |n| n.get());

    #[cfg(target_os = "linux")]
    {
        use std::collections::HashSet;

        let mut siblings = HashSet::new();
        if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                let is_cpu = name
                    .strip_prefix("cpu")
                    .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
                if !is_cpu {
                    continue;
                }
                let path = entry.path().join("topology/thread_siblings_list");
                if let Ok(list) = std::fs::read_to_string(path) {
                    siblings.insert(list.trim().to_owned());
                }
            }
        }

        if !siblings.is_empty() {
            return siblings.len().min(logical);
        }
    }

    logical
}

pub fn model_path() -> Results<PathBuf> {
    if let Ok(path) = std::env::var("MODEL_DIR") {
        return Ok(PathBuf::from(path));
    }

    let root = common::utils::project_root();

    Ok(root.join("models"))
}

pub(crate) fn opt_level() -> GraphOptimizationLevel {
    match std::env::var("ORT_OPT").as_deref() {
        Ok("level1") => GraphOptimizationLevel::Level1,
        Ok("level2") => GraphOptimizationLevel::Level2,
        Ok("level3") => GraphOptimizationLevel::Level3,
        _ => GraphOptimizationLevel::Level2,
    }
}
