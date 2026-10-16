import torch
from torch import nn

from .encode import ACTION_SPACE, FEATURE_LEN


class PolicyValueNet(nn.Module):
    """Small MLP with a policy head (logits over the action space) and a
    value head (tanh scalar). Width and depth are configurable so the same
    code scales from a toy baseline to a large model."""

    def __init__(self, width: int = 128, depth: int = 2) -> None:
        super().__init__()
        layers: list[nn.Module] = []
        d = FEATURE_LEN
        for _ in range(depth):
            layers += [nn.Linear(d, width), nn.ReLU()]
            d = width
        self.trunk = nn.Sequential(*layers)
        self.policy = nn.Linear(d, ACTION_SPACE)
        self.value = nn.Sequential(nn.Linear(d, 1), nn.Tanh())

    def forward(self, x: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        h = self.trunk(x)
        return self.policy(h), self.value(h).squeeze(-1)
