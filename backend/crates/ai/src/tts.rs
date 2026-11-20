// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use ort::{session::Session, value::Tensor};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::Command,
    sync::{Mutex, OnceLock},
    time::Instant,
};

use crate::onnx::{Results, model_path, session as build_session};

const STYLE_DIM: usize = 256;
const MAX_TOKENS: usize = 510;
const MODEL_RATE: u32 = 24_000;
const OPUS_RATE: u32 = 48_000;
const OPUS_FRAME: usize = 960;
const SAMPLES_PER_MS: usize = (MODEL_RATE / 1000) as usize;

static SESSION: OnceLock<Mutex<Session>> = OnceLock::new();
static TOKENIZER: OnceLock<KokoroTokenizer> = OnceLock::new();

struct KokoroTokenizer {
    vocab: HashMap<char, i64>,
}

impl KokoroTokenizer {
    fn load(path: impl AsRef<Path>) -> Results<Self> {
        let content = fs::read_to_string(path)?;
        let config: serde_json::Value = serde_json::from_str(&content)?;

        let vocab = config["model"]["vocab"]
            .as_object()
            .ok_or("tokenizer.json tidak memiliki model.vocab")?;

        let mut map = HashMap::new();

        for (symbol, id) in vocab {
            let id = id.as_i64().ok_or("invalid token ID")?;
            let mut chars = symbol.chars();

            if let (Some(ch), None) = (chars.next(), chars.next()) {
                map.insert(ch, id);
            }
        }

        Ok(Self { vocab: map })
    }

    fn encode(&self, phonemes: &str) -> (Vec<i64>, Vec<char>) {
        let mut tokens = Vec::with_capacity(phonemes.len());
        let mut dropped = Vec::new();

        for ch in phonemes.chars() {
            match self.vocab.get(&ch) {
                Some(id) => tokens.push(*id),
                None => dropped.push(ch),
            }
        }

        (tokens, dropped)
    }
}

fn tokenizer(dir: &Path) -> Results<&'static KokoroTokenizer> {
    if let Some(t) = TOKENIZER.get() {
        return Ok(t);
    }

    let t = KokoroTokenizer::load(dir.join("tokenizer.json"))?;
    Ok(TOKENIZER.get_or_init(|| t))
}

fn session(dir: &Path, model: &str) -> Results<&'static Mutex<Session>> {
    if let Some(s) = SESSION.get() {
        return Ok(s);
    }

    let session = build_session(dir.join(model))?;

    Ok(SESSION.get_or_init(|| Mutex::new(session)))
}

#[allow(clippy::chunks_exact_to_as_chunks)]
fn load_style(path: impl AsRef<Path>, n_tokens: usize) -> Results<Vec<f32>> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|e| format!("failed load style {}: {e}", path.display()))?;

    let start = n_tokens * STYLE_DIM * 4;
    let end = start + STYLE_DIM * 4;

    let slice = bytes
        .get(start..end)
        .ok_or_else(|| format!("voice file too small: {n_tokens} token"))?;

    Ok(slice
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect())
}

fn get_phonemes(text: &str) -> Results<String> {
    let output = Command::new("espeak-ng")
        .args(["-q", "-v", "en-us", "--ipa", "--"])
        .arg(text)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "eSpeak NG failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let raw = String::from_utf8(output.stdout)?;
    let joined = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(joined)
}

fn pause_ms(p: char) -> usize {
    match p {
        ',' => 120,
        ';' | ':' => 200,
        '.' => 280,
        '!' | '?' => 320,
        '…' => 450,
        _ => 0,
    }
}

fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut dots = 0usize;

    for c in text.chars() {
        if c == '.' {
            dots += 1;
            continue;
        }

        if dots > 0 {
            out.push(if dots >= 2 { '…' } else { '.' });
            dots = 0;
        }

        out.push(c);
    }

    if dots > 0 {
        out.push(if dots >= 2 { '…' } else { '.' });
    }

    out
}

fn split_segments(text: &str) -> Vec<(String, Option<char>)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        let boundary =
            pause_ms(c) > 0 && (c == '…' || chars.peek().map_or(true, |n| n.is_whitespace()));

        if boundary {
            out.push((cur.trim().to_string(), Some(c)));
            cur.clear();
        } else {
            cur.push(c);
        }
    }

    let tail = cur.trim();
    if !tail.is_empty() {
        out.push((tail.to_string(), None));
    }

    out
}

fn trim_silence(pcm: &mut Vec<f32>) {
    const THRESH: f32 = 0.01;
    const KEEP: usize = 15 * SAMPLES_PER_MS;

    let first = pcm.iter().position(|s| s.abs() > THRESH).unwrap_or(0);
    let last = pcm
        .iter()
        .rposition(|s| s.abs() > THRESH)
        .map_or(pcm.len(), |i| i + 1);

    let start = first.saturating_sub(KEEP);
    let end = (last + KEEP).min(pcm.len());

    pcm.truncate(end);
    pcm.drain(..start);
}

/// Fade in/out singkat supaya sambungan antar chunk tidak klik.
fn fade_edges(pcm: &mut [f32], ms: usize) {
    let n = (ms * SAMPLES_PER_MS).min(pcm.len() / 2);

    for i in 0..n {
        let g = i as f32 / n as f32;
        let j = pcm.len() - 1 - i;
        pcm[i] *= g;
        pcm[j] *= g;
    }
}

fn infer(dir: &Path, tokens: &[i64], voice: &str) -> Results<Vec<f32>> {
    if tokens.is_empty() {
        return Err("phoneme tokens empty".into());
    }

    if tokens.len() > MAX_TOKENS {
        return Err(format!("token too long: {} (max {MAX_TOKENS})", tokens.len()).into());
    }

    let style = load_style(
        dir.join("voices").join(format!("{voice}.bin")),
        tokens.len(),
    )?;

    let mut input_ids = Vec::with_capacity(tokens.len() + 2);
    input_ids.push(0i64);
    input_ids.extend_from_slice(tokens);
    input_ids.push(0);

    let mut session = session(dir, "model.onnx")?
        .lock()
        .map_err(|_| "session mutex poisoned")?;

    let outputs = session.run(ort::inputs![
        "input_ids" => Tensor::from_array((
            [1usize, input_ids.len()],
            input_ids
        ))?,
        "style" => Tensor::from_array((
            [1usize, STYLE_DIM],
            style
        ))?,
        "speed" => Tensor::from_array((
            [1usize],
            vec![1.20f32]
        ))?,
    ])?;

    let (_, samples) = outputs["waveform"].try_extract_tensor::<f32>()?;

    Ok(samples.to_vec())
}

fn sanitize(pcm: &mut [f32]) -> (usize, f32, f32) {
    let mut bad = 0usize;
    let mut peak = 0.0f32;
    let mut sum = 0.0f64;

    for s in pcm.iter_mut() {
        if !s.is_finite() {
            *s = 0.0;
            bad += 1;
        }

        peak = peak.max(s.abs());
        sum += (*s as f64) * (*s as f64);
    }

    let rms = if pcm.is_empty() {
        0.0
    } else {
        (sum / pcm.len() as f64).sqrt() as f32
    };

    (bad, peak, rms)
}

fn upsample_2x(pcm: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(pcm.len() * 2);

    for i in 0..pcm.len() {
        let a = pcm[i];
        let b = if i + 1 < pcm.len() { pcm[i + 1] } else { a };
        out.push(a);
        out.push((a + b) * 0.5);
    }

    out
}

fn encode_opus(pcm: &[f32]) -> Results<Vec<Vec<u8>>> {
    let mut encoder = opus::Encoder::new(OPUS_RATE, opus::Channels::Mono, opus::Application::Voip)?;
    encoder.set_bitrate(opus::Bitrate::Bits(32_000))?;

    let mut packets = Vec::with_capacity(pcm.len() / OPUS_FRAME + 1);
    let mut frame = [0.0f32; OPUS_FRAME];
    let mut buf = [0u8; 1500];

    for chunk in pcm.chunks(OPUS_FRAME) {
        frame.fill(0.0);

        for (dst, src) in frame.iter_mut().zip(chunk) {
            *dst = src.clamp(-1.0, 1.0);
        }

        let n = encoder.encode_float(&frame, &mut buf)?;
        packets.push(buf[..n].to_vec());
    }

    Ok(packets)
}

fn silence_packets(ms: usize) -> Results<Vec<Vec<u8>>> {
    if ms == 0 {
        return Ok(Vec::new());
    }

    let silence = vec![0.0f32; ms * SAMPLES_PER_MS];
    encode_opus(&upsample_2x(&silence))
}

pub fn synthesize(text: &str, voice: &str) -> Results<Vec<Vec<u8>>> {
    let dir = model_path()?.join("kokoro");
    let tokenizer = tokenizer(&dir)?;

    let text = normalize(text);
    let segments = split_segments(&text);
    let trailing = segments.last().and_then(|(_, p)| *p);

    // Fonemisasi per segmen, tanda baca ditempel kembali ke fonem.
    let mut parts: Vec<String> = Vec::with_capacity(segments.len());
    let mut spoken = false;

    for (body, punct) in &segments {
        let mut ph = if body.is_empty() {
            String::new()
        } else {
            get_phonemes(body)?
        };

        if !ph.is_empty() {
            spoken = true;
        }

        if let Some(p) = punct {
            ph.push(*p);
        }

        if !ph.is_empty() {
            parts.push(ph);
        }
    }

    if !spoken {
        return silence_packets(trailing.map_or(0, pause_ms));
    }

    let phonemes = parts.join(" ");
    let (mut tokens, dropped) = tokenizer.encode(&phonemes);
    tokens.truncate(MAX_TOKENS);

    println!("text: {text:?}");
    println!("phonemes: {phonemes}");
    println!("tokens: {} dropped: {dropped:?}", tokens.len());

    let mut pcm = infer(&dir, &tokens, voice)?;

    if pcm.is_empty() {
        return Err("tidak ada audio yang dihasilkan".into());
    }

    let (bad, peak, rms) = sanitize(&mut pcm);
    println!(
        "pcm: {} samples ({:.2}s) non_finite={bad} peak={peak:.4} rms={rms:.4}",
        pcm.len(),
        pcm.len() as f32 / MODEL_RATE as f32
    );

    trim_silence(&mut pcm);
    fade_edges(&mut pcm, 4);

    if let Some(p) = trailing {
        pcm.resize(pcm.len() + pause_ms(p) * SAMPLES_PER_MS, 0.0);
    }

    let pcm48 = upsample_2x(&pcm);
    let packets = encode_opus(&pcm48)?;

    Ok(packets)
}

pub fn speak(text: String) -> Results<Vec<Vec<u8>>> {
    let start = Instant::now();

    let packets = synthesize(text.as_str(), "af")?;

    println!("opus packets: {}", packets.len());
    println!("finish tts on {} ms", start.elapsed().as_millis());

    Ok(packets)
}

pub fn warmup() -> Result<(), String> {
    let start = Instant::now();

    synthesize("Hello, this is a warm up sentence.", "af").map_err(|e| e.to_string())?;

    println!("tts warmup done in {} ms", start.elapsed().as_millis());

    Ok(())
}
