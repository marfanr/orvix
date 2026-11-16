// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use std::env;

use ai::stt;
use api::metrics::init_metrics;
use api::server::{ServerState, run_server};
use common::utils::{init_keys, resolve_announce_ip};
use common::{logger, telemetry};
use models::conn::{self, prewarm};

fn init_rustls() {
    let provider = rustls::crypto::ring::default_provider();
    let _ = provider.install_default();
}

#[derive(Default)]
struct ArgsParsed {
    is_running_migration: bool,
}

fn handle_args() -> ArgsParsed {
    let mut parsed = ArgsParsed::default();
    let args: Vec<String> = env::args().collect();
    for item in &args {
        if item.contains("migration") {
            parsed.is_running_migration = true;
        }
    }
    parsed
}

#[tokio::main(worker_threads = 8)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_rustls();
    dotenvy::dotenv().ok();

    if let Err(e) = telemetry::init_telemetry() {
        eprintln!("Failed to initialize telemetry: {}", e);
    }

    logger::init_logger();

    stt::warm_up();

    init_keys();
    let pool = conn::init_db()?;
    prewarm(&pool, 4).await;

    let state = ServerState::new(
        pool,
        vec![
            "http://localhost:5173".to_string(),
            "http://127.0.0.1:5173".to_string(),
        ],
        resolve_announce_ip().await,
    );

    // let args = handle_args();
    // if args.is_running_migration {
    //     // TODO: create migration
    //     todo!("run migrations");
    //     return Ok(());
    // }

    init_metrics();

    run_server(state, 8080).await?;

    telemetry::shutdown_telemetry();
    Ok(())
}

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
