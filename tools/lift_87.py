#!/usr/bin/env python3
"""
STAGE 87 — FLATTENING DEFECT / EXPOSURE BALANCE LAW

Across all measured weighted runs with nested frames:

    Δ := total_cleared - total_restored

matches

    raw_lane_weight - final_envelope_weight

where:
- raw_lane_weight counts every deposited physical lane multiplicity;
- final_envelope_weight is the L1 norm of the componentwise max envelope.

Thus Δ measures exactly the multiplicity flattened by cross-frame max folding.
It is invariant under the tested AREV path variants even though cleared and
restored separately are path-dependent.
"""

cases = [
    # name, raw, final, cleared, restored
    ("single T,T",              2, 2, 2, 2),
    ("single F,F",              2, 2, 2, 2),
    ("single tf,tf",            4, 4, 4, 4),

    ("nested T|T repeated",     2, 1, 3, 2),
    ("nested F|F repeated",     2, 1, 3, 2),
    ("nested tf|tf repeated",   4, 2, 6, 4),

    ("T|F repeated",            2, 2, 3, 3),
    ("T|F control",             2, 2, 2, 2),
    ("T|tf repeated",           3, 3, 5, 5),
    ("T|tf control",            3, 3, 3, 3),
    ("tf|TF repeated",          4, 4, 6, 6),
    ("tf|TF control",           4, 4, 4, 4),

    ("T2|T1 repeated",          3, 2, 4, 3),
    ("T2|T1 control",           3, 2, 3, 2),
    ("T1|T2 repeated",          3, 2, 5, 4),
    ("T1|T2 control",           3, 2, 3, 2),

    ("tf2|tf1 repeated",        6, 4, 8, 6),
    ("tf2|tf1 control",         6, 4, 6, 4),
    ("tf1|tf2 repeated",        6, 4,10, 8),
    ("tf1|tf2 control",         6, 4, 6, 4),

    ("depth3 T 1,2,3",          6, 3,12, 9),
    ("depth3 T 3,1,2",          6, 3,10, 7),
    ("depth3 vector TF",        6, 4,11, 9),
]

for name, raw, final, cleared, restored in cases:
    assert cleared - restored == raw - final, name

print("STAGE 87 — FLATTENING DEFECT / EXPOSURE BALANCE LAW")
print("="*82)
print("87A measured identity: TRUE across all encoded Stage 83–86 cases")
print("  Δ = cleared - restored = raw_lane_weight - final_envelope_weight")
print()
print("87B interpretation:")
print("  Δ is exactly the multiplicity discarded by cross-frame max folding.")
print("  Δ=0 when frame regions occupy disjoint lane support or all duplicates")
print("  remain inside one frame.")
print()
print("87C path invariance:")
print("  extra AREV operations alter cleared and restored separately")
print("  but preserve Δ in every tested path pair.")
print()
print("87D depth-3 checks:")
print("  T multiplicities (1,2,3): raw 6, final 3, Δ=3; cleared/restored 12/9")
print("  T multiplicities (3,1,2): raw 6, final 3, Δ=3; cleared/restored 10/7")
print("  vector (2,0)|(0,2)|(1,1): raw 6, final 4, Δ=2; cleared/restored 11/9")
print()
print("87E algebraic form:")
print("  let m_i be per-frame lane multiplicity vectors")
print("  raw = ||Σ_i m_i||_1")
print("  envelope = ||max_i m_i||_1  (componentwise max)")
print("  flattening defect:")
print("      Δ = ||Σ_i m_i||_1 - ||max_i m_i||_1")
print()
print("87F consequence:")
print("  exposure accounting factors into")
print("      path-dependent traffic + path-invariant flattening defect.")
print("  The endpoint forgets flattened duplicates, while Δ measures exactly")
print("  how much multiplicity was quotiented away.")
print()
print("PARACONSISTENT LANDING")
print("  cleared/restored traffic changes with path")
print("  AND their defect remains invariant.")
print("  multiplicity is lost under frame folding")
print("  AND the loss is exactly measurable.")
print()
print("STAGE 87 RESULT : True")
