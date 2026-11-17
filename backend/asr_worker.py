#!/usr/bin/env python3
"""
SPDX-FileCopyrightText: 2026 Mohammad Arfan

SPDX-License-Identifier: AGPL-3.0
"""

import os
import struct
import sys

OUT = os.fdopen(os.dup(1), "wb")
os.dup2(2, 1)

import numpy as np
from faster_whisper import WhisperModel

MIN_AVG_LOGPROB = float(os.environ.get("WHISPER_MIN_LOGPROB", "-1.0"))
MAX_NO_SPEECH = float(os.environ.get("WHISPER_MAX_NO_SPEECH", "0.6"))

def read_exact(n: int):
    buf = bytearray()
    while len(buf) < n:
        chunk = sys.stdin.buffer.read(n - len(buf))
        if not chunk:
            return None
        buf.extend(chunk)
    return bytes(buf)


def write_frame(text: str) -> None:
    data = text.encode("utf-8")
    OUT.write(struct.pack("<I", len(data)) + data)
    OUT.flush()


def main() -> int:
    model_name = os.environ.get("WHISPER_MODEL", "base.en")
    language = os.environ.get("WHISPER_LANG", "en") or None
    if language == "auto":
        language = None
    if model_name.endswith(".en"):
        language = "en"

    model = WhisperModel(
        model_name,
        device=os.environ.get("WHISPER_DEVICE", "auto"),
        compute_type=os.environ.get("WHISPER_COMPUTE", "int8"),
        cpu_threads=int(os.environ.get("WHISPER_THREADS", "8")),
        download_root=os.environ.get("WHISPER_DOWNLOAD_ROOT") or None,
    )
    beam_size = int(os.environ.get("WHISPER_BEAM", "5"))

    # warm-up
    list(
        model.transcribe(
            np.zeros(16000, dtype=np.float32),
            language=language,
            beam_size=1,
            without_timestamps=True,
        )[0]
    )

    write_frame("READY")

    while True:
        header = read_exact(4)
        if header is None:
            break

        (length,) = struct.unpack("<I", header)
        if length == 0:
            break

        data = read_exact(length)
        if data is None:
            break

        pcm = np.frombuffer(data, dtype=np.float32)
        if pcm.size < 1600:
            write_frame("")
            continue

        segments, _ = model.transcribe(
            pcm,
            language=language,
            task="transcribe",
            beam_size=beam_size,
            best_of=3,
            temperature=(0.0, 0.2, 0.4),
            condition_on_previous_text=False,
            without_timestamps=True,
            vad_filter=True,
            no_speech_threshold=MAX_NO_SPEECH,
            log_prob_threshold=-1.0,
            compression_ratio_threshold=2.4,
        )

        parts = []
        for seg in segments:
            text = seg.text.strip()
            if not text:
                continue
            if seg.avg_logprob < MIN_AVG_LOGPROB or (
                seg.no_speech_prob > MAX_NO_SPEECH and seg.avg_logprob < -0.7
            ):
                continue
            parts.append(text)

        write_frame(" ".join(parts).strip())

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        pass
