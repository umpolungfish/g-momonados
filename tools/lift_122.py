#!/usr/bin/env python3
"""
STAGE 122 — DIFFERENCE-SPECTRUM TRANSFER / PAIR-COLLISION RECURRENCE

Let
    Q'(x)=Q(x)(1+x^g)
after appending one Boolean coordinate of weight g.

For the multiplicity autocorrelation
    M(x)=Q(x)Q(x^-1)=sum_d D_d x^d,

we get

    M'(x)=M(x)(2+x^g+x^-g).

Therefore the exact coefficient recurrence is

    D'_d = 2D_d + D_{d-g} + D_{d+g}.

At zero lag, using D_{-g}=D_g,

    D'_0 = 2D_0 + 2D_g.

Let
    P=(D_0-N)/2
be the number of unordered distinct equal-traffic schedule pairs,
where N is the current schedule count.

Since N'=2N,

    P' = 2P + D_g.

Thus D_g is exactly the number of newly created cross-copy collision pairs
at append weight g, while all old collision pairs are duplicated.
"""

from itertools import product
from collections import Counter

def coeffs(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        t=sum(b*g for b,g in zip(bits,weights))
        c[t]+=1
    return c

def spectrum(c):
    D=Counter()
    for a,qa in c.items():
        for b,qb in c.items():
            D[a-b]+=qa*qb
    return D

def unordered_pairs(c):
    return sum(q*(q-1)//2 for q in c.values())

families = [
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
    D=spectrum(c)
    c2=coeffs(prefix+(g,))
    D2=spectrum(c2)

    keys=set(D2)
    for d in keys:
        assert D2[d] == 2*D[d] + D[d-g] + D[d+g]

    assert D2[0] == 2*D[0] + 2*D[g]

    P=unordered_pairs(c)
    P2=unordered_pairs(c2)
    assert P2 == 2*P + D[g]

print("STAGE 122 — DIFFERENCE-SPECTRUM TRANSFER / PAIR-COLLISION RECURRENCE")
print("="*98)
print("122A append transfer:")
print("  Q'(x)=Q(x)(1+x^g)")
print()
print("122B autocorrelation transfer:")
print("  M'(x)=M(x)(2+x^g+x^-g)")
print()
print("122C lag recurrence:")
print("  D'_d=2D_d+D_{d-g}+D_{d+g}")
print()
print("122D zero-lag recurrence:")
print("  D'_0=2D_0+2D_g")
print()
print("122E unordered collision-pair recurrence:")
print("  P' = 2P + D_g")
print()
print("122F interpretation:")
print("  old collision pairs duplicate across the new Boolean coordinate")
print("  AND D_g counts the newly created cross-copy equal-traffic pairs.")
print()
print("122G finite audit:")
print("  complete, binary, repeated, and gapped examples: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  support overlap controls collision-excess creation")
print("  AND multiplicity difference-spectrum controls collision-pair creation.")
print()
print("STAGE 122 RESULT : True")
