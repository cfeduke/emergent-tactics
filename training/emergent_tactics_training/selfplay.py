"""Stage 1: generate training samples by parallel self-play in Rust."""

from __future__ import annotations

import argparse
import time

import emergent_tactics as et

from . import config as cfg
from .data import decode_samples, save_generation


def main() -> None:
    parser = argparse.ArgumentParser(description="Emergent Tactics: self-play stage")
    parser.add_argument("--config", default="configs/baseline.yaml")
    parser.add_argument("--games", type=int, help="override games in config")
    parser.add_argument("--generation", type=int, help="generation number to write (default: next)")
    args = parser.parse_args()

    c = cfg.load(args.config)
    sp = c.selfplay
    if args.games is not None:
        sp["games"] = args.games
    gen_index = args.generation or cfg.next_generation_index(c.data_dir)
    gen_dir = c.data_dir / f"gen{gen_index}"

    print(f"generation {gen_index}: {sp['games']} games, {sp['iterations']} iterations x {sp['determinizations']} determinizations, rollout evaluator")
    t0 = time.time()
    raw = et.self_play_rollout(**sp)
    arrays = decode_samples(raw)
    stats = raw["stats"]
    elapsed = time.time() - t0

    n = arrays["features"].shape[0]
    print(
        f"  {stats['games']} games -> {n} samples in {elapsed:.1f}s "
        f"({stats['games'] / elapsed:.1f} games/s, {n / elapsed:.0f} samples/s)"
    )
    print(
        f"  first={stats['first_wins']} second={stats['second_wins']} draws={stats['draws']} "
        f"avg_rounds={stats['total_rounds'] / max(stats['games'], 1):.1f}"
    )
    meta = {
        "generation": gen_index,
        "evaluator": "rollout",
        "library_fingerprint": et.LIBRARY_FINGERPRINT,
        "feature_len": et.FEATURE_LEN,
        "action_space": et.ACTION_SPACE,
        "selfplay": sp,
        "stats": stats,
    }
    save_generation(gen_dir, arrays, meta)
    print(f"  wrote {gen_dir / 'samples.npz'}")


if __name__ == "__main__":
    main()
