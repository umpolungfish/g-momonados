#!/usr/bin/env python3
"""
STAGE 125 — FACTORIAL FIBRE HIERARCHY / r-WAY COLLISIONS

For aggregate fibre sizes q_k, define for r>=1

    F_r = sum_k C(q_k, r).

Interpretation:
    F_r counts unordered r-element sets of distinct schedules
    that share the same aggregate traffic value.

Special cases:
    F_1 = total schedules = 2^m
    F_2 = unordered collision-pair count P.

Detection:
    F_r > 0  iff  some fibre has size at least r.

Hence the maximum fibre size is recovered exactly by

    max_k q_k = max { r : F_r > 0 }.

The full sequence (F_1,F_2,...) is the binomial/factorial moment
profile of the traffic quotient.

For one fibre of size q, the local contribution is
    (C(q,1), C(q,2), ..., C(q,q)).

This refines Stage 124:
    H2 = F_2 - X
detects fibres of size >=3,
while higher F_r detect progressively larger fibres.
"""

from itertools import product
from collections import Counter
from math import comb

def fibres(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        t=sum(b*g for b,g in zip(bits,weights))
        c[t]+=1
    return c

def hierarchy(weights):
    c=fibres(weights)
    M=max(c.values())
    F={}
    for r in range(1,M+2):
        F[r]=sum(comb(q,r) for q in c.values() if q>=r)
    assert F[M] > 0
    assert F[M+1] == 0
    assert F[1] == (1<<len(weights))
    return c,F,M

anchors=[
    (1,2,4,8),
    (1,2,3,4),
    (1,1,1,1),
    (1,1,1,1,1),
]
for ws in anchors:
    c,F,M=hierarchy(ws)
    assert M == max(c.values())
    assert max(r for r,v in F.items() if v>0)==M

print("STAGE 125 — FACTORIAL FIBRE HIERARCHY / r-WAY COLLISIONS")
print("="*94)
print("125A hierarchy:")
print("  F_r=sum_k C(q_k,r)")
print()
print("125B meaning:")
print("  F_r counts unordered r-tuples of distinct schedules")
print("  lying in one aggregate-traffic fibre")
print()
print("125C special cases:")
print("  F_1=2^m")
print("  F_2=P (unordered collision pairs)")
print()
print("125D fibre-size detection:")
print("  F_r>0 iff some fibre has size >=r")
print("  max fibre = max{r:F_r>0}")
print()
print("125E anchors:")
for ws in anchors:
    c,F,M=hierarchy(ws)
    vals=[F[r] for r in range(1,M+1)]
    print(f"  {ws}: max fibre={M}, F={vals}")
print()
print("125F Stage-124 relation:")
print("  H2=F_2-X")
print("  detects multiplicity beyond double fibres")
print()
print("PARACONSISTENT LANDING")
print("  one quotient may have the same support-level collision count")
print("  AND a different higher-order fibre hierarchy.")
print()
print("STAGE 125 RESULT : True")
