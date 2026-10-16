"""Stage 3: export a checkpoint to ONNX with library metadata, and verify
that ONNX Runtime reproduces PyTorch's outputs."""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
import onnx
import onnxruntime as ort
import torch

import emergent_tactics as et

from . import config as cfg
from .model import PolicyValueNet


def main() -> None:
    parser = argparse.ArgumentParser(description="Emergent Tactics: export stage")
    parser.add_argument("--config", default="configs/baseline.yaml")
    parser.add_argument("--checkpoint", help="path to .pt (default: newest in models dir)")
    parser.add_argument("--opset", type=int, default=18)
    args = parser.parse_args()

    c = cfg.load(args.config)
    if args.checkpoint:
        ckpt_path = Path(args.checkpoint)
    else:
        pts = sorted(c.models_dir.glob("*.pt"), key=lambda p: p.stat().st_mtime)
        if not pts:
            raise SystemExit(f"no checkpoints in {c.models_dir}; run et-train first")
        ckpt_path = pts[-1]

    ckpt = torch.load(ckpt_path, map_location="cpu")
    if ckpt["library_fingerprint"] != et.LIBRARY_FINGERPRINT:
        raise SystemExit(f"checkpoint library {ckpt['library_fingerprint']} != engine {et.LIBRARY_FINGERPRINT}")

    model = PolicyValueNet(**ckpt["model"])
    model.load_state_dict(ckpt["state_dict"])
    model.eval()

    out_path = ckpt_path.with_suffix(".onnx")
    example = torch.zeros(1, et.FEATURE_LEN)
    torch.onnx.export(
        model,
        (example,),
        str(out_path),
        input_names=["features"],
        output_names=["policy", "value"],
        dynamic_axes={"features": {0: "batch"}, "policy": {0: "batch"}, "value": {0: "batch"}},
        opset_version=args.opset,
        dynamo=False,
    )

    m = onnx.load(str(out_path))
    del m.metadata_props[:]
    for k, v in {
        "library_fingerprint": et.LIBRARY_FINGERPRINT,
        "feature_len": str(et.FEATURE_LEN),
        "action_space": str(et.ACTION_SPACE),
        "model_name": ckpt["name"],
        "width": str(ckpt["model"]["width"]),
        "depth": str(ckpt["model"]["depth"]),
        "generations": ",".join(map(str, ckpt["generations"])),
    }.items():
        m.metadata_props.add(key=k, value=v)
    onnx.checker.check_model(m)
    onnx.save(m, str(out_path))

    # Parity: ONNX Runtime must match PyTorch on random and real-looking input.
    sess = ort.InferenceSession(str(out_path), providers=["CPUExecutionProvider"])
    x = np.random.default_rng(0).standard_normal((64, et.FEATURE_LEN)).astype(np.float32)
    g = et.Game(seed=1)
    x[0] = np.array(g.encode(), dtype=np.float32)
    with torch.no_grad():
        tp, tv = model(torch.from_numpy(x))
    op, ov = sess.run(None, {"features": x})
    dp = float(np.abs(tp.numpy() - op).max())
    dv = float(np.abs(tv.numpy() - ov).max())
    tol = 1e-4
    status = "ok" if dp < tol and dv < tol else "MISMATCH"
    print(f"exported {out_path} ({out_path.stat().st_size} bytes), opset {args.opset}")
    print(f"  parity vs torch on 64 rows: policy max|Δ|={dp:.2e} value max|Δ|={dv:.2e} -> {status}")
    print(f"  metadata: fingerprint={et.LIBRARY_FINGERPRINT} feature_len={et.FEATURE_LEN} action_space={et.ACTION_SPACE}")
    if status != "ok":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
