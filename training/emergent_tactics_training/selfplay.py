"""Stage 1: generate training samples by parallel self-play in Rust.

With `evaluator: rollout` leaves are scored by random playouts (generation
1). With `evaluator: network` the ONNX model scores them, batched across
all worker threads, which is how every later generation runs.
"""

from __future__ import annotations

import argparse
import time

import emergent_tactics as et

from . import config as cfg
from .data import decode_samples, save_generation

SEARCH_KEYS = (
    "games",
    "seed",
    "iterations",
    "determinizations",
    "exploration",
    "temperature",
    "temperature_turns",
    "root_noise_alpha",
    "root_noise_weight",
    "threads",
)


def main() -> None:
    parser = argparse.ArgumentParser(description="Emergent Tactics: self-play stage")
    parser.add_argument("--config", default="configs/baseline.yaml")
    parser.add_argument("--games", type=int, help="override games in config")
    parser.add_argument("--evaluator", choices=["rollout", "network"], help="override evaluator in config")
    parser.add_argument("--model", help="ONNX path or 'latest' (network evaluator)")
    parser.add_argument("--generation", type=int, help="generation number to write (default: next)")
    args = parser.parse_args()

    c = cfg.load(args.config)
    sp = c.selfplay
    if args.games is not None:
        sp["games"] = args.games
    evaluator = args.evaluator or sp.get("evaluator", "rollout")
    gen_index = args.generation or cfg.next_generation_index(c.data_dir)
    gen_dir = c.data_dir / f"gen{gen_index}"

    kwargs = {k: sp[k] for k in SEARCH_KEYS if k in sp}
    model_path = None
    if evaluator == "network":
        model_path = c.resolve_model(args.model or sp.get("model"))
        kwargs.update(model=str(model_path), max_batch=sp.get("max_batch", 64), max_wait_us=sp.get("max_wait_us", 500), intra_threads=sp.get("intra_threads", 2))
        fn = et.self_play_network
    else:
        kwargs["rollout_depth"] = sp.get("rollout_depth", 60)
        fn = et.self_play_rollout

    print(
        f"generation {gen_index}: {sp['games']} games, {sp['iterations']} iterations x {sp['determinizations']} determinizations, "
        f"evaluator={evaluator}" + (f" ({model_path.name})" if model_path else "")
    )
    t0 = time.time()
    raw = fn(**kwargs)
    arrays = decode_samples(raw)
    stats = raw["stats"]
    elapsed = time.time() - t0

    n = arrays["features"].shape[0]
    print(f"  {stats['games']} games -> {n} samples in {elapsed:.1f}s ({stats['games'] / elapsed:.1f} games/s, {n / elapsed:.0f} samples/s)")
    print(
        f"  first={stats['first_wins']} second={stats['second_wins']} draws={stats['draws']} "
        f"avg_rounds={stats['total_rounds'] / max(stats['games'], 1):.1f}"
    )
    if "batches" in stats:
        print(f"  model calls: {stats['requests']} leaf evaluations in {stats['batches']} batches (avg {stats['requests'] / max(stats['batches'], 1):.1f} per call)")
    meta = {
        "generation": gen_index,
        "evaluator": evaluator,
        "model": str(model_path) if model_path else None,
        "library_fingerprint": et.LIBRARY_FINGERPRINT,
        "feature_len": et.FEATURE_LEN,
        "action_space": et.ACTION_SPACE,
        "selfplay": {**sp, "evaluator": evaluator},
        "stats": stats,
    }
    save_generation(gen_dir, arrays, meta)
    print(f"  wrote {gen_dir / 'samples.npz'}")


if __name__ == "__main__":
    main()
