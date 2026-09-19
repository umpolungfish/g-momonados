#!/usr/bin/env python3
"""
STAGE 124 — EXCESS VERSUS PAIR ENERGY / HIGHER-FIBRE DEFECT

For aggregate fibre sizes q_k, define

    X = sum_k (q_k-1)_+
      = total schedules - number of occupied traffic classes,

and

    P = sum_k C(q_k,2),

the number of unordered equal-traffic schedule pairs.

For each nonempty fibre of size q,

    C(q,2) - (q-1) = C(q-1,2).

Therefore

    P - X = sum_k C(q_k-1,2).

Call

    H2 := P-X

the higher-fibre defect.

Then:
- H2=0 iff every occupied fibre has size at most 2.
- H2>0 iff at least one traffic fibre has size >=3.

So Stage 102 has H2=0 even though X=5,
while equal-weight B4 has H2=16 because its larger fibres create many
extra internal pairs beyond simple collision excess.
"""

from itertools import product
from collections import Counter
from math import comb

def fibres(weights):
    c=Counter()
    for b in product((0,1), repeat=len(weights)):
        c[sum(x*g for x,g in zip(b,weights))]+=1
    return c

def stats(weights):
    c=fibres(weights)
    X=sum(max(q-1,0) for q in c.values())
    P=sum(comb(q,2) for q in c.values())
    H2=sum(comb(q-1,2) for q in c.values() if q>=1)
    assert P-X==H2
    maxq=max(c.values())
    assert (H2==0) == (maxq<=2)
    return c,X,P,H2,maxq

anchors = [
    (1,2,3),
    (1,2,3,4),
    (1,2,4,8),
    (1,1,1,1),
    (1,1,1,1,1),
]
for ws in anchors:
    stats(ws)

print("STAGE 124 — EXCESS VERSUS PAIR ENERGY / HIGHER-FIBRE DEFECT")
print("="*94)
print("124A two collision statistics:")
print("  X=sum_k (q_k-1)_+")
print("  P=sum_k C(q_k,2)")
print()
print("124B exact difference:")
print("  P-X=sum_k C(q_k-1,2)")
print()
print("124C define higher-fibre defect:")
print("  H2 := P-X")
print()
print("124D criterion:")
print("  H2=0 iff every occupied fibre has size <=2")
print("  H2>0 iff some fibre has size >=3")
print()
print("124E anchors:")
for ws in anchors:
    c,X,P,H2,maxq=stats(ws)
    print(f"  {ws}: X={X}, P={P}, H2={H2}, max fibre={maxq}")
print()
print("124F key contrast:")
print("  Stage-102 weights (1,2,3,4): X=5, P=5, H2=0")
print("  Equal-weight B4 (1,1,1,1): X=11, P=27, H2=16")
print()
print("PARACONSISTENT LANDING")
print("  two systems can both have collisions")
print("  AND differ sharply in higher-fibre multiplicity geometry.")
print()
print("STAGE 124 RESULT : True")
