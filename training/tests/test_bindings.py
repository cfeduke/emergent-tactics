import emergent_tactics as et


def test_engine_roundtrip():
    g = et.Game()
    assert g.turn() == 0
    assert len(g.encode()) == et.FEATURE_LEN
    g.apply(g.legal_actions()[0])
    assert g.turn() == 1
