#!/usr/bin/env python3
"""
STAGE 123 — SIGNED-RELATION MULTIPLICITY FORMULA

For schedule weights g1,...,gm,

    Q(x)=prod_i (1+x^{g_i})

and the multiplicity difference spectrum is

    M(x)=Q(x)Q(x^-1)
        =prod_i (2+x^{g_i}+x^{-g_i}).

For each coordinate i, a pair of schedule bits (b_i,c_i) contributes

    eps_i = b_i-c_i in {-1,0,+1}.

The multiplicity of a given eps_i is:
    eps_i=+1 : one bit pair  (1,0)
    eps_i=-1 : one bit pair  (0,1)
    eps_i= 0 : two bit pairs (0,0),(1,1).

Therefore, for any lag d,

    D_d
      = sum_{eps in {-1,0,1}^m,
              sum_i eps_i g_i = d}
          2^{z(eps)}

where
    z(eps)=#{i : eps_i=0}.

At d=0:
    D_0 = 2^m
          + sum_{nonzero signed zero relations eps}
              2^{z(eps)}.

The first term is the trivial relation eps=0.

Thus the unordered equal-traffic collision-pair count is

    P
      = (D_0-2^m)/2
      = 1/2 * sum_{eps != 0,
                    sum eps_i g_i=0}
                2^{z(eps)}.

This upgrades Stage 109:
  existence of a nontrivial signed zero relation <=> a collision exists,
while Stage 123 weights every relation by the number of schedule pairs
realizing it.
"""

from itertools import product
from collections import Counter

def q_coeffs(weights):
    c=Counter()
    for b in product((0,1), repeat=len(weights)):
        c[sum(x*g for x,g in zip(b,weights))]+=1
    return c

def diff_spectrum_from_q(c):
    D=Counter()
    for a,qa in c.items():
        for b,qb in c.items():
            D[a-b]+=qa*qb
    return D

def signed_spectrum(weights):
    D=Counter()
    for eps in product((-1,0,1), repeat=len(weights)):
        d=sum(e*g for e,g in zip(eps,weights))
        z=sum(e==0 for e in eps)
        D[d]+=1<<z
    return D

def collision_pairs(weights):
    q=q_coeffs(weights)
    return sum(v*(v-1)//2 for v in q.values())

families=[
    (1,),
    (1,2,3),
    (1,2,3,4),
    (1,2,4,8),
    (1,1,1,1),
    (2,5,9),
    (1,3,7,20),
]

for ws in families:
    q=q_coeffs(ws)
    Dq=diff_spectrum_from_q(q)
    Ds=signed_spectrum(ws)
    assert Dq==Ds

    m=len(ws)
    nontrivial_weight=0
    for eps in product((-1,0,1), repeat=m):
        if all(e==0 for e in eps):
            continue
        if sum(e*g for e,g in zip(eps,ws))==0:
            nontrivial_weight += 1 << sum(e==0 for e in eps)

    assert Ds[0] == (1<<m) + nontrivial_weight
    assert collision_pairs(ws) == nontrivial_weight//2
    assert nontrivial_weight % 2 == 0

anchors={}
for ws in [(1,2,3),(1,2,3,4),(1,2,4,8),(1,1,1,1)]:
    D=signed_spectrum(ws)
    P=collision_pairs(ws)
    anchors[ws]=(D[0],P)

print("STAGE 123 — SIGNED-RELATION MULTIPLICITY FORMULA")
print("="*94)
print("123A factorization:")
print("  M(x)=prod_i (2+x^{g_i}+x^{-g_i})")
print()
print("123B signed-relation spectrum:")
print("  D_d = sum_{sum eps_i g_i=d} 2^{z(eps)}")
print("  eps_i in {-1,0,+1}")
print("  z(eps)=number of zero coordinates")
print()
print("123C zero-lag decomposition:")
print("  D_0 = 2^m + sum_{nonzero signed zero relations} 2^{z(eps)}")
print()
print("123D unordered collision-pair formula:")
print("  P = 1/2 * sum_{eps != 0, sum eps_i g_i=0} 2^{z(eps)}")
print()
print("123E Stage-109 equivalence:")
print("  collision exists iff a nonzero signed zero relation exists")
print()
print("123F anchors:")
for ws,(D0,P) in anchors.items():
    print(f"  {ws}: D0={D0}, unordered collision pairs={P}")
print()
print("123G executable audit:")
print("  direct schedule-pair spectrum = signed-relation spectrum")
print("  across binary, repeated, gapped, and collision-rich families: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  Stage 109 detects whether the signed kernel is nontrivial")
print("  AND Stage 123 measures the exact multiplicity carried by that kernel.")
print()
print("STAGE 123 RESULT : True")
