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
devnull = os.open(os.devnull, os.O_WRONLY)
os.dup2(devnull, 2)
os.close(devnull)

import re

import numpy as np
from pywhispercpp.model import Model

SAMPLE_RATE = 16000
MIN_SAMPLES = 1600
MIN_RMS = float(os.environ.get("WHISPER_MIN_RMS", "0.003"))
LOGPROB_THOLD = float(os.environ.get("WHISPER_MIN_LOGPROB", "-1.0"))
NO_SPEECH_THOLD = float(os.environ.get("WHISPER_MAX_NO_SPEECH", "0.6"))
BEAM = int(os.environ.get("WHISPER_BEAM", "2"))
THREADS = int(os.environ.get("WHISPER_THREADS", "4"))
ADAPTIVE_CTX = os.environ.get("WHISPER_ADAPTIVE_CTX", "1") == "1"
CTX_FLOOR = int(os.environ.get("WHISPER_CTX_FLOOR", "384"))


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


def build_params(pcm_len: int) -> dict:
    params = dict(
        n_threads=THREADS,
        no_context=True,
        single_segment=True,
        print_progress=False,
        print_realtime=False,
        print_timestamps=False,
        suppress_blank=True,
        temperature=0.0,
        temperature_inc=0.2,
        logprob_thold=LOGPROB_THOLD,
        no_speech_thold=NO_SPEECH_THOLD,
        entropy_thold=2.4,
    )
    if BEAM > 1:
        params["beam_search"] = {"beam_size": BEAM, "patience": -1.0}
    if ADAPTIVE_CTX:
        secs = pcm_len / SAMPLE_RATE
        params["audio_ctx"] = min(1500, max(CTX_FLOOR, int(secs * 50) + 128))
    return params


def main() -> int:
    model_name = os.environ.get("WHISPER_MODEL", "small.en")
    language = os.environ.get("WHISPER_LANG", "en") or "en"
    if model_name.endswith(".en") or ".en-" in model_name:
        language = "en"

    model = Model(
        model_name,
        models_dir=os.environ.get("WHISPER_DOWNLOAD_ROOT") or None,
        params_sampling_strategy=1 if BEAM > 1 else 0,
        language=language,
        translate=False,
        print_progress=False,
        print_realtime=False,
    )

    warm = np.zeros(SAMPLE_RATE, dtype=np.float32)
    model.transcribe(warm, **build_params(warm.size))

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
        if pcm.size < MIN_SAMPLES or float(np.sqrt(np.mean(pcm * pcm))) < MIN_RMS:
            write_frame("")
            continue

        segments = model.transcribe(pcm, **build_params(pcm.size))
        text = " ".join(s.text.strip() for s in segments if s.text.strip())
        write_frame(text)

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        pass
