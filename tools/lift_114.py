#!/usr/bin/env python3
"""
STAGE 114 — TRAFFIC CAPACITY BALANCE / COLLISIONS VERSUS HOLES

For positive integer schedule weights g1,...,gm, define

    S = sum_i g_i
    Q(x) = product_i (1+x^{g_i}) = sum_{k=0}^S q_k x^k.

Let
    A = {k : q_k > 0}          occupied aggregate traffic levels
    s = |A|                    number of occupied levels
    X = sum_k max(q_k-1, 0)    collision excess
    H = (S+1) - s              holes in the traffic interval [0,S].

Since
    sum_k q_k = 2^m
and every occupied level contributes one baseline schedule,

    X = 2^m - s.

Therefore the exact balance law is

    H - X = (S+1) - 2^m

or equivalently

    X - H = 2^m - (S+1).

Interpretation:
- if S+1 < 2^m, compression forces at least 2^m-(S+1) collision excess;
- if S+1 = 2^m, collision excess equals hole count;
- if S+1 > 2^m, an injective code must have exactly S+1-2^m holes.

Native anchors:
- Stage 94 weights (1,2,3): S=6, m=3, H=0, X=1.
- Stage 102 weights (1,2,3,4): S=10, m=4, H=0, X=5.
- Stage 110 weights (1,2,4,8): S=15, m=4, H=0, X=0.
"""

from itertools import product
from collections import Counter

def stats(weights):
    m=len(weights)
    S=sum(weights)
    c=Counter()
    for bits in product((0,1), repeat=m):
        t=sum(b*g for b,g in zip(bits,weights))
        c[t]+=1
    s=len(c)
    X=sum(max(v-1,0) for v in c.values())
    H=(S+1)-s
    assert X == (1<<m)-s
    assert H-X == (S+1)-(1<<m)
    return S,s,H,X,c

anchors = {
    (1,2,3):(6,7,0,1),
    (1,2,3,4):(10,11,0,5),
    (1,2,4,8):(15,16,0,0),
    (1,1,1,1):(4,5,0,11),
}
for ws,expected in anchors.items():
    S,s,H,X,_=stats(ws)
    assert (S,s,H,X)==expected

# Audit a broad finite family.
families = [
    (1,), (2,), (1,3), (2,3,4), (1,2,5),
    (1,3,4,10), (1,2,4,7), (1,1,2,5),
    (2,5,9,20), (1,2,4,8,16)
]
for ws in families:
    stats(ws)

print("STAGE 114 — TRAFFIC CAPACITY BALANCE / COLLISIONS VERSUS HOLES")
print("="*94)
print("114A definitions:")
print("  S=sum weights")
print("  s=# occupied aggregate traffic levels")
print("  X=collision excess=sum_k max(q_k-1,0)")
print("  H=(S+1)-s holes in [0,S]")
print()
print("114B exact balance law:")
print("  X=2^m-s")
print("  H-X=(S+1)-2^m")
print("  equivalently X-H=2^m-(S+1)")
print()
print("114C Stage-94 native anchor:")
print("  weights (1,2,3): S+1=7, schedules=8")
print("  H=0, X=1")
print()
print("114D Stage-102 native anchor:")
print("  weights (1,2,3,4): S+1=11, schedules=16")
print("  H=0, X=5")
print()
print("114E Stage-110 native anchor:")
print("  weights (1,2,4,8): S+1=16, schedules=16")
print("  H=0, X=0")
print()
print("114F capacity interpretation:")
print("  span deficit forces collision excess")
print("  span surplus forces holes if the code stays injective")
print("  exact span balance permits collision-free hole-free coding")
print()
print("PARACONSISTENT LANDING")
print("  collisions and holes are distinct defects")
print("  AND their difference is fixed exactly by traffic-span capacity.")
print()
print("STAGE 114 RESULT : True")
