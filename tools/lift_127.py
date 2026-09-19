#!/usr/bin/env python3
"""
STAGE 127 — COLLISION-HIERARCHY GENERATING FUNCTION

For aggregate fibre sizes q_k define the reduced fibre polynomial

    Psi(y) = sum_k ((1+y)^{q_k} - 1).

Because
    (1+y)^{q_k}-1 = sum_{r>=1} C(q_k,r)y^r,

we have

    Psi(y) = sum_{r>=1} F_r y^r,

where
    F_r = sum_k C(q_k,r)

is the Stage-125 r-way collision hierarchy.

Thus one polynomial packages the entire hierarchy.

Appending a Boolean coordinate of weight g gives
    q'_k = q_k + q_{k-g}.

Define
    A_k(y) = (1+y)^{q_k}-1.

Then
    A'_k = A_k + A_{k-g} + A_k A_{k-g},

so after summing over k,

    Psi'(y) = 2 Psi(y) + K_g(y),

where

    K_g(y) = sum_k A_k(y) A_{k-g}(y).

The coefficient of y^r in K_g is exactly

    sum_{a=1}^{r-1} V_{a,r-a}(g),

recovering the mixed Vandermonde term from Stage 126.
"""

from itertools import product
from collections import Counter
from math import comb

def fibres(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        c[sum(b*g for b,g in zip(bits,weights))]+=1
    return c

def F(c,r):
    return sum(comb(q,r) for q in c.values() if q>=r)

def psi_coeffs(c):
    M=max(c.values(), default=0)
    return [0] + [F(c,r) for r in range(1,M+1)]

def V(c,a,b,g):
    keys=set(c) | {k+g for k in c}
    total=0
    for k in keys:
        q=c[k]
        p=c[k-g]
        ca=comb(q,a) if q>=a else 0
        cb=comb(p,b) if p>=b else 0
        total += ca*cb
    return total

def append(c,g):
    keys=set(c) | {k+g for k in c}
    return Counter({k:c[k]+c[k-g] for k in keys if c[k]+c[k-g]})

families=[
    ((),1),
    ((1,),1),
    ((1,2),3),
    ((1,2,3),4),
    ((1,2,4),8),
    ((1,1,1),1),
    ((2,5),3),
]

for prefix,g in families:
    c=fibres(prefix)
    c2=append(c,g)
    M=max(max(c.values(),default=0), max(c2.values(),default=0))
    for r in range(1,M+1):
        lhs=F(c2,r)
        cross=sum(V(c,a,r-a,g) for a in range(1,r))
        rhs=2*F(c,r)+cross
        assert lhs==rhs

print("STAGE 127 — COLLISION-HIERARCHY GENERATING FUNCTION")
print("="*94)
print("127A reduced fibre polynomial:")
print("  Psi(y)=sum_k ((1+y)^{q_k}-1)")
print()
print("127B hierarchy packaging:")
print("  Psi(y)=sum_{r>=1} F_r y^r")
print()
print("127C append law:")
print("  q'_k=q_k+q_{k-g}")
print()
print("127D overlap kernel:")
print("  A_k(y)=(1+y)^{q_k}-1")
print("  K_g(y)=sum_k A_k(y)A_{k-g}(y)")
print()
print("127E exact transfer:")
print("  Psi'(y)=2 Psi(y)+K_g(y)")
print()
print("127F coefficient recovery:")
print("  [y^r]K_g=sum_{a=1}^{r-1} V_{a,r-a}(g)")
print("  recovering Stage 126 simultaneously for every r")
print()
print("127G finite audit:")
print("  binary, repeated, collision-rich, and gapped examples: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  the whole collision hierarchy is one polynomial")
print("  AND its transfer still resolves mixed cross-copy fibre structure.")
print()
print("STAGE 127 RESULT : True")
