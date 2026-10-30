#!/usr/bin/env python3
"""
STAGE 128 — BINOMIAL INVERSION / EXACT FIBRE-HISTOGRAM RECOVERY

Let
    n_j = #{k : q_k=j}
be the number of aggregate traffic fibres of size exactly j.

Define
    F_0 = number of occupied fibres = sum_j n_j,
and for r>=1
    F_r = sum_k C(q_k,r).

Then

    F_r = sum_{j>=r} n_j C(j,r).

This is an upper-triangular binomial transform.

The exact inverse is

    n_j = sum_{r>=j} (-1)^{r-j} C(r,j) F_r.

Equivalently, if

    C(y) = sum_{r>=0} F_r y^r,

then

    C(y) = sum_j n_j (1+y)^j.

Writing
    H(t)=sum_j n_j t^j,

we have

    C(y)=H(1+y),
    H(t)=C(t-1).

Therefore the full factorial collision hierarchy determines the exact
fibre-size histogram, and vice versa.
"""

from itertools import product
from collections import Counter
from math import comb

def fibres(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        c[sum(b*g for b,g in zip(bits,weights))]+=1
    return c

def histogram(c):
    return Counter(c.values())

def hierarchy(c):
    M=max(c.values(),default=0)
    F={0:len(c)}
    for r in range(1,M+1):
        F[r]=sum(comb(q,r) for q in c.values() if q>=r)
    return F,M

def invert(F,M):
    n={}
    for j in range(1,M+1):
        n[j]=sum(((-1)**(r-j))*comb(r,j)*F.get(r,0)
                 for r in range(j,M+1))
    return Counter({j:v for j,v in n.items() if v})

anchors=[
    (1,2,4,8),
    (1,2,3,4),
    (1,1,1,1),
    (1,1,1,1,1),
    (2,5,9),
]
for ws in anchors:
    c=fibres(ws)
    h=histogram(c)
    F,M=hierarchy(c)
    inv=invert(F,M)
    assert inv==h
    assert F[0]==sum(h.values())

print("STAGE 128 — BINOMIAL INVERSION / EXACT FIBRE-HISTOGRAM RECOVERY")
print("="*96)
print("128A fibre histogram:")
print("  n_j=#{traffic fibres of size exactly j}")
print()
print("128B forward transform:")
print("  F_r=sum_{j>=r} n_j C(j,r)")
print()
print("128C exact inverse:")
print("  n_j=sum_{r>=j} (-1)^(r-j) C(r,j) F_r")
print()
print("128D generating-function form:")
print("  C(y)=sum_{r>=0} F_r y^r")
print("      =sum_j n_j(1+y)^j")
print("  H(t)=sum_j n_j t^j = C(t-1)")
print()
print("128E anchors:")
for ws in anchors:
    c=fibres(ws)
    h=histogram(c)
    F,M=hierarchy(c)
    hs=", ".join(f"{j}:{h[j]}" for j in sorted(h))
    print(f"  {ws}: histogram {{{hs}}}")
print()
print("128F information theorem:")
print("  full factorial hierarchy <-> exact fibre-size histogram")
print("  with no multiplicity information loss")
print()
print("128G executable audit:")
print("  binary, Stage-102, equal-weight, and gapped anchors: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  the hierarchy compresses fibres into global moments")
print("  AND binomial inversion reconstructs their exact size distribution.")
print()
print("STAGE 128 RESULT : True")
