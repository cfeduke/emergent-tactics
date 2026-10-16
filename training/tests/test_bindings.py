import json

import emergent_tactics as et


def test_engine_roundtrip():
    g = et.Game(seed=1)
    assert g.turn() == 0
    assert g.to_move() == 0
    assert len(g.encode()) == et.FEATURE_LEN
    assert len(et.ACTION_NAMES) == et.ACTION_SPACE
    assert len(et.LIBRARY_FINGERPRINT) == 16
    assert "Vulnerable" in et.STATUS_NAMES
    assert et.ARCHETYPE_NAMES == ["Warrior"]
    legal = g.legal_actions()
    assert et.ACTION_SPACE - 1 in legal  # end turn always legal
    g.apply(et.ACTION_SPACE - 1)
    assert g.turn() == 1
    assert g.to_move() == 1


def test_random_game_terminates():
    import random

    rnd = random.Random(0)
    g = et.Game(seed=2)
    while not g.is_over():
        g.apply(rnd.choice(g.legal_actions()))
    assert g.outcome() in (-1.0, 0.0, 1.0)
    obs = json.loads(g.observation_json())
    assert "hand" in obs


def test_search_returns_visit_distribution():
    g = et.Game(seed=3)
    chosen, root_value, cands = g.search(iterations=50, determinizations=5)
    assert chosen in g.legal_actions()
    assert sum(v for _, _, v, _ in cands) == 50
    assert -1.0 <= root_value <= 1.0


def test_self_play_rollout_produces_consistent_samples():
    import numpy as np

    from emergent_tactics_training.data import decode_samples

    raw = et.self_play_rollout(games=4, seed=7, iterations=20, determinizations=3, threads=2)
    a = decode_samples(raw)
    n = a["features"].shape[0]
    assert n > 0 and raw["stats"]["games"] == 4
    assert a["policy"].shape == (n, et.ACTION_SPACE)
    np.testing.assert_allclose(a["policy"].sum(axis=1), 1.0, atol=1e-5)
    assert ((a["policy"] > 0) <= (a["legal"] > 0)).all()
    assert set(np.unique(a["value"])) <= {-1.0, 0.0, 1.0}
    # Within one game, both players' values are opposite unless a draw.
    g0 = a["meta"][:, 1] == a["meta"][0, 1]
    players = a["meta"][g0, 0]
    vals = a["value"][g0]
    if 0.0 not in vals:
        assert len({(p, v) for p, v in zip(players, vals)}) == 2


def test_network_self_play_and_arena_if_model_present():
    from pathlib import Path

    model = Path(__file__).resolve().parents[2] / "models" / "gen1.onnx"
    if not model.exists():
        return
    raw = et.self_play_network(str(model), games=4, seed=1, iterations=20, determinizations=3, threads=2)
    assert raw["stats"]["games"] == 4 and raw["stats"]["requests"] > 0
    s = et.arena_match("net-novice", "random", 4, model=str(model), threads=2)
    assert s["games"] == 4
