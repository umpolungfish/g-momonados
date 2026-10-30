#!/usr/bin/env python3
"""
STAGE 96 — B4 BINOMIAL TRAFFIC CUBE / HAMMING-WEIGHT QUOTIENT

Executable prediction audit only. Native Stage-96 measurements are still pending.

Depth 5 with one T deposit in each frame gives:
    W=5
    g1=g2=g3=g4=g5=1

Optional re-clears correspond to E subset {Γ2,Γ3,Γ4,Γ5}, so:
    τ(E)=|E|
    C_E=5+|E|
    R_E=1+|E|
    Δ=4

Therefore:
    Q(x)=(1+x)^4=1+4x+6x^2+4x^3+x^4

Predicted traffic fibre sizes:
    1,4,6,4,1
"""

from itertools import product
from math import comb

cube = list(product((0,1), repeat=4))
fibres = {}
for b in cube:
    k = sum(b)
    fibres.setdefault(k, []).append(b)

assert [len(fibres[k]) for k in range(5)] == [1,4,6,4,1]
assert all(len(fibres[k]) == comb(4,k) for k in range(5))

for b in cube:
    k = sum(b)
    C, R = 5+k, 1+k
    assert C-R == 4

print("STAGE 96 — B4 BINOMIAL TRAFFIC CUBE / HAMMING-WEIGHT QUOTIENT")
print("="*86)
print("96A executable combinatorial prediction: TRUE")
print("  schedule cube B4 has 16 schedules")
print("  τ(E)=|E|")
print("  C_E=5+|E|, R_E=1+|E|, Δ=4")
print()
print("96B predicted fibre sizes:")
for k in range(5):
    print(f"  |E|={k}: {len(fibres[k])}=C(4,{k})")
print()
print("96C generating polynomial:")
print("  Q(x)=(1+x)^4=1+4x+6x^2+4x^3+x^4")
print()
print("96D predicted aggregate traffic classes:")
for k in range(5):
    print(f"  rank {k}: C/R={5+k}/{1+k}")
print()
print("96E symmetry:")
print("  S4 permutes the four optional re-clear coordinates.")
print("  Aggregate traffic is constant on each Boolean rank layer.")
print()
print("STAGE 96 NATIVE STATUS : PENDING")
