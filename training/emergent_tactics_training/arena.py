"""Stage 4: pit checkpoints against each other. Network-vs-network play
needs the Rust network evaluator (ort), which lands with the inference
wiring; until then use `cargo run -p et-cli -- arena` for rollout presets."""

import argparse


def main() -> None:
    parser = argparse.ArgumentParser(description="Emergent Tactics: arena stage")
    parser.add_argument("--config", default="configs/baseline.yaml")
    parser.parse_args()
    raise SystemExit("arena: network evaluator not wired yet; use `cargo run --release -p et-cli -- arena`")


if __name__ == "__main__":
    main()
