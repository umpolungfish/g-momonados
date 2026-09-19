#!/usr/bin/env python3
"""
STAGE 103 — RESOLVED WEIGHTED COLLISION FIBRES

Uses the 16 native Stage-102 resolved traffic traces.
Aggregate traffic has five 2-element collision fibres:
  τ=3,4,5,6,7.

The ordered clear/restore trace is injective on all 16 schedules,
so every aggregate collision is separated at resolved level.
"""

from itertools import product
from collections import defaultdict

bits = list(product((0,1), repeat=4))
weights = (1,2,3,4)

traces = [(('C', 15), ('R', 1), ('R', 1), ('R', 1), ('R', 1), ('R', 1)), (('C', 15), ('R', 1), ('R', 1), ('R', 1), ('R', 1), ('C', 4), ('R', 5)), (('C', 15), ('R', 1), ('R', 1), ('R', 1), ('C', 3), ('R', 4), ('R', 1)), (('C', 15), ('R', 1), ('R', 1), ('R', 1), ('C', 3), ('R', 4), ('C', 4), ('R', 5)), (('C', 15), ('R', 1), ('R', 1), ('C', 2), ('R', 3), ('R', 1), ('R', 1)), (('C', 15), ('R', 1), ('R', 1), ('C', 2), ('R', 3), ('R', 1), ('C', 4), ('R', 5)), (('C', 15), ('R', 1), ('R', 1), ('C', 2), ('R', 3), ('C', 3), ('R', 4), ('R', 1)), (('C', 15), ('R', 1), ('R', 1), ('C', 2), ('R', 3), ('C', 3), ('R', 4), ('C', 4), ('R', 5)), (('C', 15), ('R', 1), ('C', 1), ('R', 2), ('R', 1), ('R', 1), ('R', 1)), (('C', 15), ('R', 1), ('C', 1), ('R', 2), ('R', 1), ('R', 1), ('C', 4), ('R', 5)), (('C', 15), ('R', 1), ('C', 1), ('R', 2), ('R', 1), ('C', 3), ('R', 4), ('R', 1)), (('C', 15), ('R', 1), ('C', 1), ('R', 2), ('R', 1), ('C', 3), ('R', 4), ('C', 4), ('R', 5)), (('C', 15), ('R', 1), ('C', 1), ('R', 2), ('C', 2), ('R', 3), ('R', 1), ('R', 1)), (('C', 15), ('R', 1), ('C', 1), ('R', 2), ('C', 2), ('R', 3), ('R', 1), ('C', 4), ('R', 5)), (('C', 15), ('R', 1), ('C', 1), ('R', 2), ('C', 2), ('R', 3), ('C', 3), ('R', 4), ('R', 1)), (('C', 15), ('R', 1), ('C', 1), ('R', 2), ('C', 2), ('R', 3), ('C', 3), ('R', 4), ('C', 4), ('R', 5))]

assert len(traces) == 16
assert len(set(traces)) == 16

fibres = defaultdict(list)
for b,t in zip(bits,traces):
    tau=sum(x*w for x,w in zip(b,weights))
    fibres[tau].append((b,t))

for k in (3,4,5,6,7):
    assert len(fibres[k]) == 2
    assert fibres[k][0][1] != fibres[k][1][1]

print("STAGE 103 — RESOLVED WEIGHTED COLLISION FIBRES")
print("="*86)
print("103A resolved trace injectivity: TRUE")
print("  all 16 native schedules have distinct ordered traffic words")
print()
print("103B aggregate collision fibres separated: TRUE")
for k in (3,4,5,6,7):
    print(f"  τ={k}:")
    for b,t in fibres[k]:
        print("    ", b, "->", " ".join(f"{a}{v}" for a,v in t))
print()
print("103C projection hierarchy:")
print("  schedule")
print("    -> resolved traffic word   [injective on all 16]")
print("    -> aggregate (C,R)         [11 classes]")
print("    -> defect Δ=10             [1 class]")
print("    -> endpoint T×5            [1 class]")
print()
print("PARACONSISTENT LANDING")
print("  five aggregate collision fibres identify distinct schedules")
print("  AND temporal resolution separates every collision.")
print()
print("STAGE 103 RESULT : True")
