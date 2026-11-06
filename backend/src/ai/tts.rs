use ort::{
    ep,
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use tokio::time::Instant;
use std::{
    error::Error as StdError,
    fs,
    path::{Path, PathBuf},
};

type Error = Box<dyn StdError + Send + Sync>;
type Results<T> = std::result::Result<T, Error>;

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

fn opt_level() -> GraphOptimizationLevel {
    match std::env::var("ORT_OPT").as_deref() {
        Ok("disable") => GraphOptimizationLevel::Disable,
        Ok("level2") => GraphOptimizationLevel::Level2,
        Ok("level3") => GraphOptimizationLevel::Level3,
        // default Level1: segfault terjadi di pass Level3 (NCHWc/NHWC)
        _ => GraphOptimizationLevel::Level1,
    }
}

fn synthesize_tokens(
    dir: &Path,
    tokens: &[i64],
    voice: &str,
    out_path: &str,
) -> Result<(), String> {
    if tokens.len() > 510 {
        return Err(format!("token terlalu panjang: {} (maks 510)", tokens.len()).into());
    }

    let style = load_style(
        dir.join("voices").join(format!("{voice}.bin")),
        tokens.len(),
    )
    .map_err(|e| e.to_string())?;

    // pad token 0 di awal dan akhir
    let mut input_ids = Vec::with_capacity(tokens.len() + 2);
    input_ids.push(0i64);
    input_ids.extend(tokens);
    input_ids.push(0);

    let env = ort::init()
        .with_execution_providers([ep::CPU::default().build()])
        .build()
        .map_err(|e| e.to_string())?;

    let model_file = std::env::var("MODEL_FILE").unwrap_or_else(|_| "model.onnx".into());
    let mut session = Session::builder(&env)
        .map_err(|e| e.to_string())?
        .with_optimization_level(opt_level())
        .map_err(|e| e.to_string())?
        .commit_from_file(dir.join("onnx").join(model_file))
        .map_err(|e| e.to_string())?;

    let outputs = session.run(ort::inputs![
        "input_ids" => Tensor::from_array(([1usize, input_ids.len()], input_ids)).map_err(|e| e.to_string())?,
        "style"     => Tensor::from_array(([1usize, STYLE_DIM], style)).map_err(|e| e.to_string())?,
        "speed"     => Tensor::from_array(([1usize], vec![1.0f32])).map_err(|e| e.to_string())?,
    ]).map_err(|e| e.to_string())?;

    let (_, samples) = outputs["waveform"]
        .try_extract_tensor::<f32>()
        .map_err(|e| e.to_string())?;

    let mut w = hound::WavWriter::create(
        out_path,
        hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .map_err(|e| e.to_string())?;
    for &s in samples {
        w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())?;
    Ok(())
}

// for demo purposes
const STYLE_DIM: usize = 256;
const SAMPLE_RATE: u32 = 24_000;

const TOKENS: &[i64] = &[
    50, 157, 43, 135, 16, 53, 135, 46, 16, 43, 102, 16, 56, 156, 57, 135, 6, 16, 102, 62, 61, 16,
    70, 56, 16, 138, 56, 156, 72, 56, 61, 85, 123, 83, 44, 83, 54, 16, 53, 65, 156, 86, 61, 62,
    131, 83, 56, 4, 16, 54, 156, 43, 102, 53, 16, 156, 72, 61, 53, 102, 112, 16, 70, 56, 16, 138,
    56, 44, 156, 76, 158, 123, 56, 16, 62, 131, 156, 43, 102, 54, 46, 16, 102, 48, 16, 81, 47, 102,
    54, 16, 54, 156, 51, 158, 46, 16, 70, 16, 92, 156, 135, 46, 16, 54, 156, 43, 102, 48, 4, 16,
    81, 47, 102, 16, 50, 156, 72, 64, 83, 56, 62, 16, 156, 51, 158, 64, 83, 56, 16, 44, 157, 102,
    56, 16, 44, 156, 76, 158, 123, 56, 4,
];

pub fn test_onnx() {
    let start = Instant::now();
    let dir = model_path().unwrap();
    synthesize_tokens(&dir, TOKENS, "af_aoede", "out.wav")
        .map_err(|e| println!("err: {}", e))
        .unwrap();
    let duration = start.elapsed();
    println!("finish tts on {} ms", duration.as_millis());
}
