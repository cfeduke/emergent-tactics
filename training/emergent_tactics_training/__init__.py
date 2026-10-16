"""Training pipeline for Emergent Tactics.

Stages, each a CLI entry point:

1. selfplay  -- generate (features, visit distribution, outcome) samples by
                running ISMCTS self-play through the Rust engine.
2. train     -- fit the policy/value network on accumulated samples.
3. export    -- write a checkpoint to ONNX for the Rust inference service.
4. arena     -- pit checkpoints against each other to measure progress and
                calibrate difficulty presets.
"""
