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
