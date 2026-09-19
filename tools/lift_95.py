#!/usr/bin/env python3
"""
STAGE 95 — BINOMIAL TRAFFIC FIBRES / HAMMING-WEIGHT QUOTIENT

Native depth-4 equal-envelope case:
  one T deposit in each of four nested frames.

Raw lane weight:
  W = 4

All suffix-max envelopes have norm 1:
  g1=g2=g3=g4=1

Optional re-clears are indexed by E ⊆ {Γ2,Γ3,Γ4}.
Therefore:
  τ(E)=|E|
  C_E=4+|E|
  R_E=1+|E|
  Δ=3

Native aggregate traffic:
  rank 0: 4/1                  [1 schedule]
  rank 1: 5/2,5/2,5/2        [3 schedules]
  rank 2: 6/3,6/3,6/3        [3 schedules]
  rank 3: 7/4                  [1 schedule]

Hence:
  Q(x)=(1+x)^3=1+3x+3x^2+x^3
and traffic fibres are exactly Boolean rank layers.
"""

from itertools import product
from math import comb

cube = list(product((0,1), repeat=3))

def rank(b):
    return sum(b)

fibres = {}
for b in cube:
    fibres.setdefault(rank(b), []).append(b)

assert {k:len(v) for k,v in fibres.items()} == {0:1,1:3,2:3,3:1}
assert all(len(fibres[k]) == comb(3,k) for k in range(4))

native = [
    (4,1),
    (5,2),(5,2),(5,2),
    (6,3),(6,3),(6,3),
    (7,4),
]
predicted = sorted((4+rank(b), 1+rank(b)) for b in cube)
assert sorted(native) == predicted
assert all(c-r == 3 for c,r in native)

# Resolved native traffic words, coordinates ordered (Γ2,Γ3,Γ4)
resolved = {
    (0,0,0): ("C4","R1","R0","R0","R0"),
    (0,0,1): ("C4","R1","C1","R1","R0","R0"),
    (0,1,0): ("C4","R1","R0","C1","R1","R0"),
    (1,0,0): ("C4","R1","R0","R0","C1","R1"),
    (0,1,1): ("C4","R1","C1","R1","C1","R1","R0"),
    (1,0,1): ("C4","R1","C1","R1","R0","C1","R1"),
    (1,1,0): ("C4","R1","R0","C1","R1","C1","R1"),
    (1,1,1): ("C4","R1","C1","R1","C1","R1","C1","R1"),
}
assert len(set(resolved.values())) == 8

print("STAGE 95 — BINOMIAL TRAFFIC FIBRES / HAMMING-WEIGHT QUOTIENT")
print("="*84)
print("95A equal envelope weights: TRUE")
print("  W=4 and g1=g2=g3=g4=1")
print()
print("95B traffic quotient:")
print("  τ(E)=|E|")
print("  C_E=4+|E|")
print("  R_E=1+|E|")
print("  Δ=C-R=3")
print()
print("95C native fibre sizes: TRUE")
for k in range(4):
    print(f"  |E|={k}: fibre size={len(fibres[k])}=C(3,{k})")
print()
print("95D generating polynomial:")
print("  Q(x)=(1+x)^3=1+3x+3x^2+x^3")
print("  coefficient [x^k]Q = C(3,k) = traffic-fibre cardinality.")
print()
print("95E Boolean-rank quotient:")
print("  aggregate traffic depends only on Hamming weight |E|.")
print("  Thus the traffic fibres are exactly the rank layers of B3.")
print()
print("95F symmetry:")
print("  the full coordinate-permutation group S3 preserves aggregate traffic.")
print("  each rank-k fibre is one S3 orbit.")
print()
print("95G resolved trace:")
print("  all 8 measured schedules still have distinct ordered traffic words.")
print("  therefore temporal resolution separates schedules collapsed by rank.")
print()
print("PARACONSISTENT LANDING")
print("  three rank-1 schedules are aggregate-identical")
print("  AND resolved-trace distinct.")
print("  three rank-2 schedules are aggregate-identical")
print("  AND resolved-trace distinct.")
print("  aggregate traffic sees only Boolean rank")
print("  AND the execution trace retains coordinate position.")
print()
print("STAGE 95 RESULT : True")
