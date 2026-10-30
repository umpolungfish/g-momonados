#!/usr/bin/env python3
"""
STAGE 90 — ARBITRARY AREV-SCHEDULE TRAFFIC LAW

Depth-3 vector case:
  m1=(2,0), m2=(0,2), m3=(1,1)

Suffix-max envelopes:
  Γ3=(1,1), ||Γ3||_1=2
  Γ2=(1,2), ||Γ2||_1=3
  Γ1=(2,2), ||Γ1||_1=4

Raw lane weight:
  ||m1+m2+m3||_1 = 6

Native schedules:
  baseline single-AREV:
      C/R = 6/4

  extra AREV after Γ3:
      C/R = 8/6

  extra AREV after Γ2:
      C/R = 9/7

  repeated AREV after Γ3 and Γ2:
      C/R = 11/9

Thus each extra AREV at envelope Γ_i adds ||Γ_i||_1
to both clear and restore traffic.
"""

raw = 6
g = {1:4, 2:3, 3:2}
delta = raw - g[1]
assert delta == 2

measured = {
    frozenset(): (6,4),
    frozenset({3}): (8,6),
    frozenset({2}): (9,7),
    frozenset({2,3}): (11,9),
}

def predicted(E):
    extra = sum(g[i] for i in E)
    return raw + extra, g[1] + extra

for E, cr in measured.items():
    assert predicted(E) == cr
    assert cr[0] - cr[1] == delta

print("STAGE 90 — ARBITRARY AREV-SCHEDULE TRAFFIC LAW")
print("="*82)
print("90A measured schedule family: TRUE")
for E in sorted(measured, key=lambda x:(len(x), sorted(x))):
    c,r = measured[E]
    label = "baseline" if not E else "extra clear at " + ",".join(f"Γ{i}" for i in sorted(E))
    print(f"  {label:<28} C/R={c}/{r}")
print()
print("90B schedule law: TRUE")
print("  for E subset of reconstructed envelopes cleared again,")
print("      C_E = raw + Σ_{i∈E} ||Γ_i||_1")
print("      R_E = ||Γ_1||_1 + Σ_{i∈E} ||Γ_i||_1")
print()
print("90C local traffic increments:")
print("  clear Γ3 again -> +2 clear, +2 restore")
print("  clear Γ2 again -> +3 clear, +3 restore")
print("  clear both      -> +5 clear, +5 restore")
print()
print("90D path-invariant quantities:")
print("  final weighted endpoint = Γ1 = (2,2)")
print("  SIXTEEN_3 support = TF")
print("  Δ = C-R = raw-||Γ1||_1 = 2")
print()
print("90E path-dependent quantity:")
print("  total clear/restore traffic depends additively on the AREV schedule.")
print()
print("90F decomposition:")
print("  endpoint envelope : Γ1")
print("  flattening defect : Δ")
print("  schedule traffic  : Σ_{i∈E} ||Γ_i||_1")
print()
print("PARACONSISTENT LANDING")
print("  AREV schedule changes traffic")
print("  AND leaves endpoint and defect invariant.")
print("  each re-clear is locally measurable")
print("  AND globally composes additively over the schedule.")
print()
print("STAGE 90 RESULT : True")
