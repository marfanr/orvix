// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ort::{
    environment::Environment,
    ep,
    session::{Session, builder::GraphOptimizationLevel},
};

pub type Error = Box<dyn StdError + Send + Sync>;
pub type Results<T> = std::result::Result<T, Error>;

static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();

pub(crate) fn environment() -> Results<&'static Environment> {
    if let Some(env) = ENVIRONMENT.get() {
        return Ok(env);
    }

    let env = ort::init()
        .with_execution_providers([ep::CPU::default().build()])
        .build()?;

    Ok(ENVIRONMENT.get_or_init(|| env))
}

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

    let model = fs::read(path)?;
    let session = builder.commit_from_memory(&model)?;

    Ok(session)
}

fn physical_cores() -> usize {
    let logical = std::thread::available_parallelism().map_or(1, |n| n.get());
    tracing::info!("logical {}", logical);
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
