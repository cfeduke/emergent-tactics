"""Stage 4: pit controllers against each other to rank checkpoints and
calibrate difficulty presets. Runs the Rust arena in parallel."""

from __future__ import annotations

import argparse

import emergent_tactics as et

from . import config as cfg


def main() -> None:
    parser = argparse.ArgumentParser(description="Emergent Tactics: arena stage")
    parser.add_argument("--config", default="configs/baseline.yaml")
    parser.add_argument("--model", help="ONNX path or 'latest' for net-* presets")
    parser.add_argument("--games", type=int)
    parser.add_argument("matchup", nargs="*", help="pairs like net-adept adept (default: config arena.matchups)")
    args = parser.parse_args()

    c = cfg.load(args.config)
    ar = c.arena
    games = args.games or int(ar.get("games", 100))
    if args.matchup:
        if len(args.matchup) % 2:
            raise SystemExit("matchups come in pairs")
        matchups = [args.matchup[i : i + 2] for i in range(0, len(args.matchup), 2)]
    else:
        matchups = ar.get("matchups", [["net-adept", "adept"]])
    needs_model = any(name.startswith("net-") for pair in matchups for name in pair)
    model = str(c.resolve_model(args.model)) if needs_model else None
    if model:
        print(f"model: {model}")
    print(f"{'matchup':28s} {'wins':>9s} {'losses':>7s} {'draws':>6s} {'win%':>6s} {'rounds':>7s} {'s/game':>7s}")
    for a, b in matchups:
        s = et.arena_match(a, b, games, seed=int(ar.get("seed", 0)), model=model)
        decided = s["first_wins"] + s["second_wins"]
        pct = 100.0 * s["first_wins"] / decided if decided else 0.0
        print(
            f"{a + ' vs ' + b:28s} {s['first_wins']:9d} {s['second_wins']:7d} {s['draws']:6d} {pct:6.1f} "
            f"{s['total_rounds'] / games:7.1f} {s['elapsed_secs'] / games:7.3f}"
        )


if __name__ == "__main__":
    main()
