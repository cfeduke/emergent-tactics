import argparse


def main() -> None:
    parser = argparse.ArgumentParser(description="Emergent Tactics: selfplay stage")
    parser.add_argument("--config", default="configs/baseline.yaml")
    args = parser.parse_args()
    raise SystemExit(f"selfplay: not implemented yet (config={args.config})")


if __name__ == "__main__":
    main()
