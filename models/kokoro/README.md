# Kokoro-82M (local model files)

Text-to-speech model files used by the backend. These files are **third-party
assets** and are **not** covered by this repository's AGPL-3.0 license.

## License

- Kokoro-82M model weights, tokenizer, and voice data: **Apache-2.0**
  ([LICENSE.md](LICENSE.md), [NOTICE](NOTICE))
- G2P engine used upstream: [misaki](https://github.com/hexgrad/misaki) (Apache-2.0)
- ONNX conversion tooling: [kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx) (MIT)

Apache-2.0 permits use, modification, and redistribution, including commercially,
provided that the license text and attribution notices are kept and any changes
are stated.

## Contents

| File                 | Description                                      |
| -------------------- | ------------------------------------------------ |
| `model.onnx`         | Kokoro-82M graph converted to ONNX               |
| `voices/*.bin`       | Voice style embeddings as raw float32 arrays     |
| `tokenizer.json`     | Phoneme tokenizer used by the backend            |
| `tokens.txt`         | Token/vocabulary list                            |
| `config.json`        | Model type metadata                              |
| `LICENSE.md`         | Apache License 2.0                               |
| `NOTICE`             | Attribution and list of changes                  |

## Provenance and changes

Source: [hexgrad/Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M) v1.0.

Changes made for this project:

- The PyTorch checkpoint was converted to ONNX (`model.onnx`).
- Voice style tensors were extracted from the upstream voicepacks and stored as
  raw float32 `.bin` files.
- Tokenizer files were repackaged for local inference.

## Trademark

"Kokoro" is a project of hexgrad. This copy is not affiliated with, sponsored
by, or endorsed by hexgrad.
