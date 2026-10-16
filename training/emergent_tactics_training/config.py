from __future__ import annotations

import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml


@dataclass
class Config:
    raw: dict[str, Any]
    path: Path

    @property
    def root(self) -> Path:
        return self.path.parent.parent

    @property
    def data_dir(self) -> Path:
        return self.root / self.raw.get("data_dir", "data")

    @property
    def models_dir(self) -> Path:
        return (self.root / self.raw.get("models_dir", "../models")).resolve()

    @property
    def selfplay(self) -> dict[str, Any]:
        return dict(self.raw.get("selfplay", {}))

    @property
    def model(self) -> dict[str, Any]:
        return dict(self.raw.get("model", {}))

    @property
    def train(self) -> dict[str, Any]:
        return dict(self.raw.get("train", {}))

    @property
    def arena(self) -> dict[str, Any]:
        return dict(self.raw.get("arena", {}))

    def resolve_model(self, name: str | None) -> Path:
        """'latest' picks the newest .onnx in models_dir; otherwise a path
        relative to training/."""
        if name in (None, "latest"):
            onnx = sorted(self.models_dir.glob("*.onnx"), key=lambda p: p.stat().st_mtime)
            if not onnx:
                raise SystemExit(f"no .onnx models in {self.models_dir}; run et-export first")
            return onnx[-1]
        p = Path(name)
        return p if p.is_absolute() else (self.root / p).resolve()


def load(path: str | os.PathLike[str]) -> Config:
    p = Path(path).resolve()
    with open(p) as f:
        return Config(raw=yaml.safe_load(f) or {}, path=p)


def resolve_device(name: str):
    import torch

    if name == "auto":
        return torch.device("mps") if torch.backends.mps.is_available() else torch.device("cpu")
    if name == "mps" and not torch.backends.mps.is_available():
        raise SystemExit("device: mps requested but Metal is not available in this torch build")
    return torch.device(name)


@dataclass
class Generation:
    """A numbered self-play generation directory under data/."""

    index: int
    dir: Path
    stats: dict[str, Any] = field(default_factory=dict)


def generations(data_dir: Path) -> list[Generation]:
    out: list[Generation] = []
    if not data_dir.exists():
        return out
    for d in sorted(data_dir.glob("gen*")):
        try:
            idx = int(d.name[3:])
        except ValueError:
            continue
        if (d / "samples.npz").exists():
            out.append(Generation(idx, d))
    return out


def next_generation_index(data_dir: Path) -> int:
    gens = generations(data_dir)
    return (gens[-1].index + 1) if gens else 1
