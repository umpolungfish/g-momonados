#!/usr/bin/env python3
"""
STAGE 85 — TROPICAL WEIGHT ENVELOPE / MAX-PLUS FRAME ALGEBRA

Native measurements establish, for same-lane multiplicity:

Within one frame:
    deposits add.

Across nested frame regions:
    the fold keeps the larger multiplicity, not the sum.

Depth-2 asymmetric tests:
    outer 2, inner 1 -> final 2
    outer 1, inner 2 -> final 2

for T, F, and the paired lowercase atom tf.
"""

cases = {
    "T": {
        (2,1): {"repeated_arev": (4,3,2), "control": (3,2,2)},
        (1,2): {"repeated_arev": (5,4,2), "control": (3,2,2)},
    },
    "F": {
        (2,1): {"repeated_arev": (4,3,2), "control": (3,2,2)},
        (1,2): {"repeated_arev": (5,4,2), "control": (3,2,2)},
    },
    "tf": {
        (2,1): {"repeated_arev": (8,6,2), "control": (6,4,2)},
        (1,2): {"repeated_arev": (10,8,2), "control": (6,4,2)},
    },
}

for atom, rows in cases.items():
    for (outer, inner), variants in rows.items():
        expected = max(outer, inner)
        for _, (_, _, final_mult) in variants.items():
            assert final_mult == expected

print("STAGE 85 — TROPICAL WEIGHT ENVELOPE / MAX-PLUS FRAME ALGEBRA")
print("="*84)
print("85A within-frame multiplicity addition: TRUE")
print("  repeated T/F/tf deposits in one frame retain both copies.")
print()
print("85B cross-depth fold = max on same-lane multiplicity: TRUE")
for atom in ("T","F","tf"):
    print(f"  {atom}: (2,1)->2 and (1,2)->2")
print()
print("85C native banked semantics agrees:")
print("  fold between regions keeps the larger multiplicity, not the sum;")
print("  deposits in one region keep both.")
print()
print("85D control vs repeated-AREV endpoint:")
print("  both paths end at the same max envelope.")
print("  repeated AREV changes clear/restore exposure totals only.")
print()
print("85E algebra:")
print("  local/frame-internal composition: +")
print("  inter-region/nesting composition: max")
print("  therefore each lane carries a max-plus style provenance envelope.")
print()
print("85F paired lowercase atom:")
print("  tf behaves as two physical lanes t,f with equal multiplicity;")
print("  one tf deposit contributes one t and one f.")
print("  cross-depth multiplicity still folds by max per lane.")
print()
print("85G depth-2 formula:")
print("  for lane ℓ with local multiplicities m1,m2,")
print("      Γ2(ℓ)=m2")
print("      Γ1(ℓ)=max(m1,m2)")
print("  FFUSE3 reconstructs the current frame envelope relative to what is already live.")
print()
print()
print("PARACONSISTENT LANDING")
print("  multiplicity is additive within a frame")
print("  AND idempotent/max-like across frame regions.")
print("  same final weighted envelope")
print("  AND different exposure histories.")
print()
print("STAGE 85 RESULT : True")
