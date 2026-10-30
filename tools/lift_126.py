#!/usr/bin/env python3
"""
STAGE 126 — VANDERMONDE TRANSFER FOR THE COLLISION HIERARCHY

Let q_k be the current aggregate fibre sizes.
Appending a Boolean coordinate of weight g gives

    q'_k = q_k + q_{k-g}.

Define
    F_r = sum_k C(q_k,r).

By Vandermonde's identity,

    C(q_k+q_{k-g},r)
      = sum_{a=0}^r C(q_k,a) C(q_{k-g},r-a).

Therefore

    F'_r
      = sum_{a=0}^r V_{a,r-a}(g),

where

    V_{a,b}(g)
      := sum_k C(q_k,a) C(q_{k-g},b).

The endpoint terms are

    V_{r,0}=F_r
    V_{0,r}=F_r,

so

    F'_r = 2F_r + sum_{a=1}^{r-1} V_{a,r-a}(g).

Special cases:

r=1:
    F'_1=2F_1.

r=2:
    F'_2=2F_2+V_{1,1}(g)
         =2P+D_g,

recovering Stage 122 exactly.

r=3:
    F'_3
      =2F_3
       +sum_k C(q_k,1)C(q_{k-g},2)
       +sum_k C(q_k,2)C(q_{k-g},1).

This is the exact transfer law for higher collision fibres.
"""

from itertools import product
from collections import Counter
from math import comb

def coeffs(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        c[sum(b*g for b,g in zip(bits,weights))]+=1
    return c

def append_coeffs(c,g):
    keys=set(c) | {k+g for k in c}
    return Counter({k:c[k]+c[k-g] for k in keys})

def F(c,r):
    return sum(comb(q,r) for q in c.values() if q>=r)

def V(c,a,b,g):
    out=0
    # finite range sufficient from existing support and shifted support
    keys=set(c) | {k+g for k in c}
    for k in keys:
        qa=c[k]
        qb=c[k-g]
        ca=comb(qa,a) if qa>=a else 0
        cb=comb(qb,b) if qb>=b else 0
        out += ca*cb
    return out

families=[
    ((),1),
    ((1,),1),
    ((1,),2),
    ((1,2),3),
    ((1,2,3),4),
    ((1,2,4),8),
    ((1,1,1),1),
    ((2,5),3),
    ((1,3,7),4),
]

for prefix,g in families:
    c=coeffs(prefix)
    c2=coeffs(prefix+(g,))
    assert c2==append_coeffs(c,g)

    maxr=max(max(c.values()),max(c2.values()))
    for r in range(1,maxr+2):
        lhs=F(c2,r)
        rhs=sum(V(c,a,r-a,g) for a in range(r+1))
        assert lhs==rhs
        rhs2=2*F(c,r)+sum(V(c,a,r-a,g) for a in range(1,r))
        assert lhs==rhs2

    # Pair case recovers Stage 122.
    Dg=sum(c[k]*c[k-g] for k in c)
    assert F(c2,2)==2*F(c,2)+Dg

print("STAGE 126 — VANDERMONDE TRANSFER FOR THE COLLISION HIERARCHY")
print("="*98)
print("126A append law:")
print("  q'_k=q_k+q_{k-g}")
print()
print("126B factorial hierarchy:")
print("  F_r=sum_k C(q_k,r)")
print()
print("126C mixed overlap moments:")
print("  V_{a,b}(g)=sum_k C(q_k,a) C(q_{k-g},b)")
print()
print("126D exact transfer:")
print("  F'_r=sum_{a=0}^r V_{a,r-a}(g)")
print("      =2F_r+sum_{a=1}^{r-1} V_{a,r-a}(g)")
print()
print("126E pair specialization:")
print("  F'_2=2F_2+V_{1,1}(g)")
print("      =2P+D_g")
print("  recovering Stage 122")
print()
print("126F triple specialization:")
print("  F'_3=2F_3+V_{1,2}(g)+V_{2,1}(g)")
print()
print("126G finite audit:")
print("  complete, binary, repeated, and gapped examples: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  old r-way collisions duplicate under schedule doubling")
print("  AND mixed cross-copy fibre moments create the new r-way collisions.")
print()
print("STAGE 126 RESULT : True")
