from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from .encode import ACTION_SPACE, FEATURE_LEN


def decode_samples(d: dict) -> dict[str, np.ndarray]:
    """Turn the raw buffers returned by the Rust self-play into arrays."""
    n = d["n"]
    f = np.frombuffer(d["features"], dtype=np.float32).reshape(n, FEATURE_LEN)
    p = np.frombuffer(d["policy"], dtype=np.float32).reshape(n, ACTION_SPACE)
    legal = np.frombuffer(d["legal"], dtype=np.float32).reshape(n, ACTION_SPACE)
    v = np.frombuffer(d["value"], dtype=np.float32)
    meta = np.array(d["meta"], dtype=np.int64).reshape(n, 3)
    return {"features": f.copy(), "policy": p.copy(), "legal": legal.copy(), "value": v.copy(), "meta": meta}


def save_generation(gen_dir: Path, arrays: dict[str, np.ndarray], meta: dict) -> None:
    gen_dir.mkdir(parents=True, exist_ok=True)
    np.savez_compressed(gen_dir / "samples.npz", **arrays)
    with open(gen_dir / "meta.json", "w") as f:
        json.dump(meta, f, indent=2)


def load_generations(gen_dirs: list[Path]) -> dict[str, np.ndarray]:
    parts = [np.load(d / "samples.npz") for d in gen_dirs]
    keys = ["features", "policy", "legal", "value", "meta"]
    return {k: np.concatenate([p[k] for p in parts]) for k in keys}
