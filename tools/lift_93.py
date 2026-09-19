#!/usr/bin/env python3
"""
STAGE 93 — RESOLVED TRAFFIC WORD / SCHEDULE RECOVERY

Native depth-4 case:
  local multiplicities m=(4,3,2,1)
  raw clear = 10
  suffix envelope norms:
      g4=1, g3=2, g2=3, g1=4

There are 2^3=8 binary AREV schedules, according to whether the
reconstructed envelopes Γ4, Γ3, Γ2 are cleared again before the next pop.

Stage 92 showed that aggregate traffic
    (C_total, R_total)
is NOT injective:
    E={Γ2} and E={Γ3,Γ4}
both give (13,7).

Here we retain the ordered traffic word:
    Ck = CLEAR loses k
    Rk = FFUSE3 restores k

Native measured words:
  ∅            : C10 R1 R1 R1 R1
  {Γ4}         : C10 R1 C1 R2 R1 R1
  {Γ3}         : C10 R1 R1 C2 R3 R1
  {Γ2}         : C10 R1 R1 R1 C3 R4
  {Γ4,Γ3}      : C10 R1 C1 R2 C2 R3 R1
  {Γ4,Γ2}      : C10 R1 C1 R2 R1 C3 R4
  {Γ3,Γ2}      : C10 R1 R1 C2 R3 C3 R4
  {Γ4,Γ3,Γ2}   : C10 R1 C1 R2 C2 R3 C3 R4

The resolved traffic word is injective on the full measured cube.
"""

schedules = {
    frozenset():                  ("C10","R1","R1","R1","R1"),
    frozenset({4}):               ("C10","R1","C1","R2","R1","R1"),
    frozenset({3}):               ("C10","R1","R1","C2","R3","R1"),
    frozenset({2}):               ("C10","R1","R1","R1","C3","R4"),
    frozenset({4,3}):             ("C10","R1","C1","R2","C2","R3","R1"),
    frozenset({4,2}):             ("C10","R1","C1","R2","R1","C3","R4"),
    frozenset({3,2}):             ("C10","R1","R1","C2","R3","C3","R4"),
    frozenset({4,3,2}):           ("C10","R1","C1","R2","C2","R3","C3","R4"),
}

def aggregate(word):
    c = sum(int(tok[1:]) for tok in word if tok.startswith("C"))
    r = sum(int(tok[1:]) for tok in word if tok.startswith("R"))
    return c, r

# Injective resolved words.
assert len(set(schedules.values())) == 8

# Aggregate Stage-92 collision.
agg = {E: aggregate(w) for E,w in schedules.items()}
assert agg[frozenset({2})] == (13,7)
assert agg[frozenset({3,4})] == (13,7)
assert schedules[frozenset({2})] != schedules[frozenset({3,4})]

# All defects remain 6.
assert all(c-r == 6 for c,r in agg.values())

# Recover schedule by presence of the re-clear event Cg_i immediately
# after the envelope Γ_i has been reconstructed.
recover = {w:E for E,w in schedules.items()}
assert len(recover) == 8

print("STAGE 93 — RESOLVED TRAFFIC WORD / SCHEDULE RECOVERY")
print("="*82)
print("93A full depth-4 resolved-word audit: TRUE")
for E,w in schedules.items():
    label = "∅" if not E else "{" + ",".join(f"Γ{i}" for i in sorted(E)) + "}"
    print(f"  E={label:<12} -> {' '.join(w)} -> C/R={agg[E][0]}/{agg[E][1]}")
print()
print("93B aggregate collision persists: TRUE")
print("  E={Γ2}       -> C/R=13/7")
print("  E={Γ3,Γ4}    -> C/R=13/7")
print()
print("93C resolved traffic separates the collision: TRUE")
print("  {Γ2}    : C10 R1 R1 R1 C3 R4")
print("  {Γ3,Γ4} : C10 R1 C1 R2 C2 R3 R1")
print()
print("93D injectivity on measured schedule cube: TRUE")
print("  all 8 schedules have distinct resolved traffic words.")
print()
print("93E hierarchy of projections:")
print("  schedule")
print("    -> resolved traffic word       [injective here]")
print("    -> aggregate traffic (C,R)     [non-injective]")
print("    -> defect Δ=C-R                [constant 6]")
print("    -> endpoint envelope Γ1        [constant T×4]")
print("    -> SIXTEEN_3 support           [constant T]")
print()
print("93F schedule recovery:")
print("  the positions of re-clear tokens C1,C2,C3 recover exactly which")
print("  reconstructed envelopes Γ4,Γ3,Γ2 were cleared again.")
print()
print("PARACONSISTENT LANDING")
print("  aggregate traffic identifies distinct schedules")
print("  AND ordered traffic restores their distinction.")
print("  endpoint and defect collapse the entire cube")
print("  AND the resolved trace retains its full measured geometry.")
print()
print("STAGE 93 RESULT : True")
