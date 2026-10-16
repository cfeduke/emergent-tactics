"""Stage 2: fit the policy/value network on recent self-play generations."""

from __future__ import annotations

import argparse
import json
import os
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

import emergent_tactics as et

from . import config as cfg
from .data import load_generations
from .model import PolicyValueNet


def masked_policy_loss(logits: torch.Tensor, target: torch.Tensor, legal: torch.Tensor) -> torch.Tensor:
    """Cross-entropy against the visit distribution, restricted to legal actions."""
    logits = logits.masked_fill(legal == 0, -1e9)
    logp = F.log_softmax(logits, dim=-1)
    return -(target * logp).sum(dim=-1).mean()


def run_epoch(model, data, idx, batch_size, device, value_weight, opt=None):
    total = {"loss": 0.0, "policy": 0.0, "value": 0.0, "n": 0}
    for start in range(0, len(idx), batch_size):
        b = idx[start : start + batch_size]
        x = data["features"][b].to(device)
        p = data["policy"][b].to(device)
        m = data["legal"][b].to(device)
        v = data["value"][b].to(device)
        logits, value = model(x)
        lp = masked_policy_loss(logits, p, m)
        lv = F.mse_loss(value, v)
        loss = lp + value_weight * lv
        if opt is not None:
            opt.zero_grad(set_to_none=True)
            loss.backward()
            opt.step()
        n = len(b)
        total["loss"] += loss.item() * n
        total["policy"] += lp.item() * n
        total["value"] += lv.item() * n
        total["n"] += n
    return {k: total[k] / total["n"] for k in ("loss", "policy", "value")}


def main() -> None:
    parser = argparse.ArgumentParser(description="Emergent Tactics: train stage")
    parser.add_argument("--config", default="configs/baseline.yaml")
    parser.add_argument("--name", help="checkpoint name (default: gen<N> of the latest generation)")
    parser.add_argument("--epochs", type=int)
    parser.add_argument("--device")
    args = parser.parse_args()

    c = cfg.load(args.config)
    tr = c.train
    if args.epochs is not None:
        tr["epochs"] = args.epochs
    if args.device is not None:
        tr["device"] = args.device

    gens = cfg.generations(c.data_dir)
    if not gens:
        raise SystemExit(f"no generations under {c.data_dir}; run et-selfplay first")
    use = gens[-int(tr.get("generations", 1)) :]
    name = args.name or f"gen{use[-1].index}"

    for g in use:
        meta = json.loads((g.dir / "meta.json").read_text())
        if meta["library_fingerprint"] != et.LIBRARY_FINGERPRINT:
            raise SystemExit(f"{g.dir} was generated with library {meta['library_fingerprint']}, engine is {et.LIBRARY_FINGERPRINT}")

    torch.manual_seed(int(tr.get("seed", 0)))
    threads = int(tr.get("threads", 0)) or (os.cpu_count() or 1)
    torch.set_num_threads(threads)
    device = cfg.resolve_device(str(tr.get("device", "auto")))

    arrays = load_generations([g.dir for g in use])
    n = arrays["features"].shape[0]
    rng = np.random.default_rng(int(tr.get("seed", 0)))
    perm = rng.permutation(n)
    n_val = int(n * float(tr.get("val_fraction", 0.1)))
    val_idx, train_idx = perm[:n_val], perm[n_val:]
    data = {k: torch.from_numpy(arrays[k]) for k in ("features", "policy", "legal", "value")}

    model = PolicyValueNet(width=int(c.model["width"]), depth=int(c.model["depth"])).to(device)
    params = sum(p.numel() for p in model.parameters())
    opt = torch.optim.AdamW(model.parameters(), lr=float(tr["lr"]), weight_decay=float(tr.get("weight_decay", 0.0)))
    epochs = int(tr["epochs"])
    bs = int(tr["batch_size"])
    vw = float(tr.get("value_weight", 1.0))

    print(f"training {name}: {n} samples from generations {[g.index for g in use]}, {params} params, device={device}, threads={threads}")
    t0 = time.time()
    history = []
    for epoch in range(1, epochs + 1):
        model.train()
        rng.shuffle(train_idx)
        tr_m = run_epoch(model, data, train_idx, bs, device, vw, opt)
        model.eval()
        with torch.no_grad():
            va_m = run_epoch(model, data, val_idx, bs, device, vw) if n_val else tr_m
        history.append({"epoch": epoch, "train": tr_m, "val": va_m})
        print(
            f"  epoch {epoch:3d}  train loss {tr_m['loss']:.4f} (p {tr_m['policy']:.4f} v {tr_m['value']:.4f})"
            f"  val loss {va_m['loss']:.4f} (p {va_m['policy']:.4f} v {va_m['value']:.4f})  {time.time() - t0:.1f}s"
        )

    c.models_dir.mkdir(parents=True, exist_ok=True)
    ckpt = c.models_dir / f"{name}.pt"
    torch.save(
        {
            "state_dict": {k: v.cpu() for k, v in model.state_dict().items()},
            "model": {"width": int(c.model["width"]), "depth": int(c.model["depth"])},
            "library_fingerprint": et.LIBRARY_FINGERPRINT,
            "feature_len": et.FEATURE_LEN,
            "action_space": et.ACTION_SPACE,
            "generations": [g.index for g in use],
            "history": history,
            "name": name,
        },
        ckpt,
    )
    print(f"  wrote {ckpt}")


if __name__ == "__main__":
    main()
