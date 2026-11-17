// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use crate::constant::{MAX_SAMPLES, MIN_SAMPLES, PAUSE_MS, SAMPLE_RATE, SILENCE, SPEECH_RMS};
use crate::onnx::Results;
use bytes::Bytes;
use common::utils::python_path;
use std::{
    env,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use tokio::{sync::mpsc, time::timeout};

struct WhisperWorker {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl WhisperWorker {
    fn spawn() -> Results<Self> {
        let python = python_path();
        let script = env::var("ASR_SCRIPT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| common::utils::workspace_root().join("asr_worker.py"));
        if !script.is_file() {
            return Err(format!("asr worker not found: {}", script.display()).into());
        }

        let mut child = Command::new(&python)
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("spawn {}: {e}", python.display()))?;

        let stdin = child.stdin.take().ok_or("asr worker: no stdin")?;
        let stdout = child.stdout.take().ok_or("asr worker: no stdout")?;
        let mut worker = Self {
            child,
            stdin,
            stdout,
        };

        if worker.read_frame(Duration::from_secs(120))? != "READY" {
            return Err("asr worker: bad handshake".into());
        }

        Ok(worker)
    }

    fn transcribe(&mut self, pcm: &[f32]) -> Results<String> {
        if pcm.is_empty() {
            return Ok(String::new());
        }

        let mut bytes = Vec::with_capacity(pcm.len() * 4);
        for sample in pcm {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        let len = u32::try_from(bytes.len()).map_err(|_| "asr: audio too long")?;
        self.stdin.write_all(&len.to_le_bytes())?;
        self.stdin.write_all(&bytes)?;
        self.stdin.flush()?;

        self.read_frame(Duration::from_secs(20))
    }

    fn read_frame(&mut self, timeout: Duration) -> Results<String> {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;

            let mut pfd = libc::pollfd {
                fd: self.stdout.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
            let rc = unsafe { libc::poll(&mut pfd, 1, ms) };
            if rc < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            if rc == 0 {
                return Err(format!("asr worker timeout after {timeout:?}").into());
            }
        }

        let mut header = [0u8; 4];
        self.stdout.read_exact(&mut header)?;
        let len = u32::from_le_bytes(header) as usize;
        if len > 1_000_000 {
            return Err(format!("asr worker response too large: {len}").into());
        }
        let mut buf = vec![0u8; len];
        self.stdout.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf)?)
    }
}

impl Drop for WhisperWorker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

enum AsrInput {
    Audio(Bytes),
    Flush,
}

enum Wake {
    Packet,
    Flush,
    Idle,
    Closed,
}

struct AsrJob {
    pcm: Vec<f32>,
    on_text: Arc<dyn Fn(String) + Send + Sync>,
}

struct AsrPool {
    tx: mpsc::UnboundedSender<AsrJob>,
}

static ASR_POOL: OnceLock<AsrPool> = OnceLock::new();

fn start_worker(id: usize) -> Option<WhisperWorker> {
    match WhisperWorker::spawn() {
        Ok(w) => {
            tracing::info!("asr-worker-{id}: ready");
            Some(w)
        }
        Err(e) => {
            tracing::error!("asr-worker-{id}: spawn: {e}");
            None
        }
    }
}

fn run_worker(id: usize, rx: Arc<Mutex<mpsc::UnboundedReceiver<AsrJob>>>) {
    let mut worker = start_worker(id);

    loop {
        let job = {
            let mut guard = rx.lock().unwrap_or_else(|p| p.into_inner());
            guard.blocking_recv()
        };
        let Some(job) = job else { break };

        if worker.is_none() {
            worker = start_worker(id);
        }
        let Some(w) = worker.as_mut() else { continue };

        match w.transcribe(&job.pcm) {
            Ok(text) if !text.trim().is_empty() => (job.on_text)(text),
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("asr-worker-{id}: {e}");
                worker = None;
            }
        }
    }
}

fn asr_pool() -> &'static AsrPool {
    ASR_POOL.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded_channel::<AsrJob>();
        let rx = Arc::new(Mutex::new(rx));

        let spawned = std::thread::Builder::new()
            .name(format!("asr-worker"))
            .spawn(move || run_worker(0, rx));

        if let Err(e) = spawned {
            tracing::error!("asr: spawn thread : {e}");
        }

        AsrPool { tx }
    })
}

pub fn warm_up() {
    let _ = asr_pool();
}

pub struct AsrHandle {
    tx: mpsc::UnboundedSender<AsrInput>,
}

impl AsrHandle {
    pub fn spawn(on_text: impl Fn(String) + Send + Sync + 'static) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        spawn_asr(rx, on_text);
        Self { tx }
    }

    pub fn push(&self, payload: Bytes) {
        let _ = self.tx.send(AsrInput::Audio(payload));
    }

    pub fn flush(&self) {
        let _ = self.tx.send(AsrInput::Flush);
    }
}

fn spawn_asr(
    mut rx: mpsc::UnboundedReceiver<AsrInput>,
    on_text: impl Fn(String) + Send + Sync + 'static,
) {
    let on_text: Arc<dyn Fn(String) + Send + Sync> = Arc::new(on_text);

    tokio::spawn(async move {
        let mut decoder = match opus::Decoder::new(SAMPLE_RATE, opus::Channels::Mono) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("asr: decoder: {e}");
                return;
            }
        };
        let mut frame = vec![0.0f32; SAMPLE_RATE as usize * 120 / 1000];
        let mut pcm: Vec<f32> = Vec::new();
        let mut speech = false;
        let mut silent_ms: u32 = 0;

        loop {
            let wake = match timeout(SILENCE, rx.recv()).await {
                Ok(Some(AsrInput::Audio(payload))) => {
                    if !payload.is_empty() {
                        match decoder.decode_float(&payload, &mut frame, false) {
                            Ok(n) if n > 0 => {
                                let samples = &frame[..n];
                                let rms =
                                    (samples.iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt();
                                let ms = (n as u32 * 1000) / SAMPLE_RATE;

                                if rms >= SPEECH_RMS {
                                    speech = true;
                                    silent_ms = 0;
                                } else if speech {
                                    silent_ms = silent_ms.saturating_add(ms);
                                }

                                pcm.extend_from_slice(samples);
                            }
                            Ok(_) => {}
                            Err(e) => tracing::warn!("asr: decode: {e}"),
                        }
                    }
                    Wake::Packet
                }
                Ok(Some(AsrInput::Flush)) => Wake::Flush,
                Ok(None) => Wake::Closed,
                Err(_) => Wake::Idle,
            };

            if !speech && pcm.len() >= MIN_SAMPLES {
                pcm.clear();
            }

            let finish = matches!(wake, Wake::Flush | Wake::Closed) && !pcm.is_empty();
            let idle = matches!(wake, Wake::Idle) && speech;
            let pause = speech && silent_ms >= PAUSE_MS;
            let enough = pcm.len() >= MAX_SAMPLES;
            let should_send = (finish || pause || enough || idle) && pcm.len() >= MIN_SAMPLES;

            if should_send {
                let _ = asr_pool().tx.send(AsrJob {
                    pcm: std::mem::take(&mut pcm),
                    on_text: Arc::clone(&on_text),
                });
                speech = false;
                silent_ms = 0;
            } else if finish || enough || idle {
                pcm.clear();
                speech = false;
                silent_ms = 0;
            }

            if matches!(wake, Wake::Closed) {
                break;
            }
        }
    });
}
