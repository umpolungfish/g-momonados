#!/usr/bin/env python3
"""
STAGE 133 — SUBSET-SUM SHIFT MULTIPLICITIES / COMPRESSED HORIZON OPERATOR

Let future append weights be

    G=(g1,...,gr).

Stage 132 gives

    q^(G)_k
      = sum_{E subseteq[r]}
          q_{k-s_E},

where
    s_E=sum_{i in E}g_i.

Different subsets can produce the same shift s.
Define the future subset-sum multiplicity polynomial

    A_G(x)
      = prod_i (1+x^{g_i})
      = sum_s a_s x^s,

where
    a_s = #{E subseteq[r] : s_E=s}.

Then the exact r-step transfer compresses to

    q^(G)_k = sum_s a_s q_{k-s}.

Thus:
- the 2^r indexed Boolean corners are exact but may repeat;
- the compressed operator uses only distinct shifts s with multiplicity a_s.

If all future subset sums are distinct, a_s in {0,1} and no compression occurs.
If future weights collide, repeated corners merge into coefficients a_s>1.
"""

from itertools import product
from collections import Counter

def future_mults(G):
    a=Counter()
    for bits in product((0,1), repeat=len(G)):
        s=sum(b*g for b,g in zip(bits,G))
        a[s]+=1
    return a

def fibres(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        c[sum(b*g for b,g in zip(bits,weights))]+=1
    return c

def append(c,g):
    A=set(c)
    U=A | {k+g for k in A}
    return Counter({k:c[k]+c[k-g] for k in U if c[k]+c[k-g]})

def horizon_direct(c,G):
    out=c
    for g in G:
        out=append(out,g)
    return out

def horizon_compressed(c,G):
    a=future_mults(G)
    A=set(c)
    U=set()
    for s in a:
        U |= {k+s for k in A}
    out=Counter()
    for k in U:
        v=sum(mult*c[k-s] for s,mult in a.items())
        if v:
            out[k]=v
    return out

future_sets=[
    (1,),
    (1,2),
    (1,2,4),
    (1,1,2),
    (1,2,3),
    (2,2,2),
    (1,3,3,5),
]

bases=[
    (),
    (1,),
    (1,2,3),
    (1,2,4,8),
    (1,1,1),
    (2,5,9),
]

for G in future_sets:
    a=future_mults(G)
    assert sum(a.values()) == 1<<len(G)
    for base in bases:
        c=fibres(base)
        assert horizon_direct(c,G)==horizon_compressed(c,G)

# Collision-free binary future.
a=future_mults((1,2,4,8))
assert len(a)==16
assert all(v==1 for v in a.values())

# Repeated/colliding future.
a=future_mults((1,1,2))
assert len(a)<8
assert max(a.values())>1

print("STAGE 133 — SUBSET-SUM SHIFT MULTIPLICITIES / COMPRESSED HORIZON OPERATOR")
print("="*102)
print("133A future polynomial:")
print("  A_G(x)=prod_i (1+x^{g_i})=sum_s a_s x^s")
print()
print("133B shift multiplicity:")
print("  a_s=#{future subsets E with subset sum s}")
print()
print("133C compressed transfer:")
print("  q^(G)_k=sum_s a_s q_{k-s}")
print()
print("133D relation to Stage 132:")
print("  Stage-132 Boolean window has 2^r indexed corners")
print("  AND equal-shift corners combine into coefficient a_s")
print()
print("133E collision-free future:")
print("  G=(1,2,4,8): 16 distinct shifts, all a_s=1")
print()
print("133F colliding future:")
print("  G=(1,1,2): fewer than 8 distinct shifts, some a_s>1")
print()
print("133G exact audit:")
print("  direct repeated appends = compressed shift-multiplicity transfer")
print("  across binary, repeated, and collision-rich future sets: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  2^r future schedule choices remain distinct combinatorially")
print("  AND aggregate evolution may compress them onto fewer shift locations.")
print()
print("STAGE 133 RESULT : True")
