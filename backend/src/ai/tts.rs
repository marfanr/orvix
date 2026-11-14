use ort::{
    ep,
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use std::{
    collections::HashMap,
    error::Error as StdError,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    time::Instant,
};

type Error = Box<dyn StdError + Send + Sync>;
type Results<T> = std::result::Result<T, Error>;

const STYLE_DIM: usize = 256;
const MAX_TOKENS: usize = 510;
const MODEL_RATE: u32 = 24_000;
const OPUS_RATE: u32 = 48_000;
const OPUS_FRAME: usize = 960;
const GAP_SAMPLES: usize = 2_400;

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

fn model_path() -> Results<PathBuf> {
    if let Ok(path) = std::env::var("MODEL_DIR") {
        return Ok(PathBuf::from(path));
    }

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cargo manifest has no parent")?
        .to_path_buf();

    Ok(workspace.join("models"))
}

fn tokenizer(dir: &Path) -> Results<&'static KokoroTokenizer> {
    if let Some(t) = TOKENIZER.get() {
        return Ok(t);
    }

    let t = KokoroTokenizer::load(dir.join("tokenizer.json"))?;
    Ok(TOKENIZER.get_or_init(|| t))
}

fn opt_level() -> GraphOptimizationLevel {
    match std::env::var("ORT_OPT").as_deref() {
        Ok("level1") => GraphOptimizationLevel::Level1,
        Ok("level2") => GraphOptimizationLevel::Level2,
        Ok("level3") => GraphOptimizationLevel::Level3,
        _ => GraphOptimizationLevel::Level2,
    }
}

fn session(dir: &Path) -> Results<&'static Mutex<Session>> {
    if let Some(s) = SESSION.get() {
        return Ok(s);
    }

    let env = ort::init()
        .with_execution_providers([ep::CPU::default().build()])
        .build()?;

    let model_file = std::env::var("MODEL_FILE").unwrap_or_else(|_| "model.onnx".into());

    let session = Session::builder(&env)?
        .with_optimization_level(opt_level())
        .map_err(|e| format!("{e}"))?
        .commit_from_file(dir.join("onnx").join(model_file))?;

    std::mem::forget(env);

    Ok(SESSION.get_or_init(|| Mutex::new(session)))
}

fn load_style(path: impl AsRef<Path>, n_tokens: usize) -> Results<Vec<f32>> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|e| format!("gagal baca {}: {e}", path.display()))?;

    let start = n_tokens * STYLE_DIM * 4;
    let end = start + STYLE_DIM * 4;

    let slice = bytes
        .get(start..end)
        .ok_or_else(|| format!("voice file terlalu kecil untuk {n_tokens} token"))?;

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
    Ok(raw.trim().to_string())
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();

    for ch in text.chars() {
        cur.push(ch);

        if matches!(ch, '.' | '!' | '?' | ';' | '\n') {
            let t = cur.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
            cur.clear();
        }
    }

    let t = cur.trim();
    if !t.is_empty() {
        out.push(t.to_string());
    }

    out
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

    let mut session = session(dir)?.lock().map_err(|_| "session mutex poisoned")?;

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

    Ok(samples.iter().copied().collect())
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

fn write_wav(path: &str, rate: u32, pcm: &[f32]) -> Results<()> {
    let data_len = (pcm.len() * 2) as u32;
    let mut f = fs::File::create(path)?;

    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&rate.to_le_bytes())?;
    f.write_all(&(rate * 2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;

    for s in pcm {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        f.write_all(&v.to_le_bytes())?;
    }

    Ok(())
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

fn dump_debug(raw: &[f32], pcm48: &[f32], packets: &[Vec<u8>]) -> Results<()> {
    write_wav("/tmp/tts_raw_24k.wav", MODEL_RATE, raw)?;
    write_wav("/tmp/tts_up_48k.wav", OPUS_RATE, pcm48)?;

    let mut decoder = opus::Decoder::new(OPUS_RATE, opus::Channels::Mono)?;
    let mut decoded = Vec::with_capacity(packets.len() * OPUS_FRAME);
    let mut out = [0.0f32; OPUS_FRAME];

    for p in packets {
        let n = decoder.decode_float(p, &mut out, false)?;
        decoded.extend_from_slice(&out[..n]);
    }

    write_wav("/tmp/tts_opus_roundtrip_48k.wav", OPUS_RATE, &decoded)?;

    Ok(())
}

pub fn synthesize(text: &str, voice: &str) -> Results<Vec<Vec<u8>>> {
    let dir = model_path()?;
    let tokenizer = tokenizer(&dir)?;

    let mut pcm: Vec<f32> = Vec::new();

    for sentence in split_sentences(text) {
        let phonemes = get_phonemes(&sentence)?;
        let (tokens, dropped) = tokenizer.encode(&phonemes);

        println!("sentence: {sentence:?}");
        println!("phonemes: {phonemes}");
        println!("tokens: {} dropped: {dropped:?}", tokens.len());

        for chunk in tokens.chunks(MAX_TOKENS) {
            let wave = infer(&dir, chunk, voice)?;
            pcm.extend_from_slice(&wave);
        }

        pcm.extend(std::iter::repeat(0.0f32).take(GAP_SAMPLES));
    }

    if pcm.is_empty() {
        return Err("tidak ada audio yang dihasilkan".into());
    }

    let (bad, peak, rms) = sanitize(&mut pcm);
    println!(
        "pcm: {} samples ({:.2}s) non_finite={bad} peak={peak:.4} rms={rms:.4}",
        pcm.len(),
        pcm.len() as f32 / MODEL_RATE as f32
    );

    let pcm48 = upsample_2x(&pcm);
    let packets = encode_opus(&pcm48)?;

    if std::env::var("TTS_DUMP").is_ok() {
        dump_debug(&pcm, &pcm48, &packets)?;
        println!("dumped wav ke /tmp/tts_*.wav");
    }

    Ok(packets)
}

pub fn test_onnx() -> Results<Vec<Vec<u8>>> {
    let start = Instant::now();

    let packets = synthesize("hello world! whats going? huh???", "af_aoede")?;

    println!("opus packets: {}", packets.len());
    println!("finish tts on {} ms", start.elapsed().as_millis());

    Ok(packets)
}
