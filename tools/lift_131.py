#!/usr/bin/env python3
"""
STAGE 131 — TWO-STEP RECTANGLE KERNEL / COMMUTING APPEND SQUARE

Let q_k be the current aggregate fibre-size sequence.

Appending weight g applies
    (T_g q)_k = q_k + q_{k-g}.

Appending h after g gives

    (T_h T_g q)_k
      = q_k + q_{k-g} + q_{k-h} + q_{k-g-h}.

The same expression is obtained in the opposite order:

    T_h T_g = T_g T_h.

Thus aggregate fibre evolution under append weights is commutative.

However, to predict the exact final fibre histogram from the current
state without retaining every q_k, the needed local statistic is now
the four-corner rectangle distribution

    R_{g,h}(a,b,c,d)
      = #{k :
          q_k         = a,
          q_{k-g}     = b,
          q_{k-h}     = c,
          q_{k-g-h}   = d},

excluding the all-zero tuple.

Then the final two-step histogram satisfies

    n''_j
      = sum_{a+b+c+d=j} R_{g,h}(a,b,c,d).

Equivalently, with the four-variable kernel

    J_{g,h}(u,v,w,z)
      = sum R_{g,h}(a,b,c,d) u^a v^b w^c z^d,

we have

    H''(t) = J_{g,h}(t,t,t,t).

This is the exact two-step analogue of Stage 130.
"""

from itertools import product
from collections import Counter

def fibres(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        c[sum(b*g for b,g in zip(bits,weights))]+=1
    return c

def append(c,g):
    A=set(c)
    U=A | {k+g for k in A}
    return Counter({k:c[k]+c[k-g] for k in U if c[k]+c[k-g]})

def histogram(c):
    return Counter(c.values())

def rectangle(c,g,h):
    A=set(c)
    shifts=(0,g,h,g+h)
    U=set()
    for s in shifts:
        U |= {k+s for k in A}
    R=Counter()
    for k in U:
        tup=(c[k],c[k-g],c[k-h],c[k-g-h])
        if any(tup):
            R[tup]+=1
    return R

def diagonal_hist(R):
    out=Counter()
    for vals,n in R.items():
        out[sum(vals)]+=n
    return out

families=[
    (),
    (1,),
    (1,2,3),
    (1,2,3,4),
    (1,2,4,8),
    (1,1,1,1),
    (2,2,2),
    (2,5,9),
    (1,3,7,20),
]

for ws in families:
    c=fibres(ws)
    for g,h in [(1,2),(1,3),(2,3),(2,5),(4,1)]:
        gh=append(append(c,g),h)
        hg=append(append(c,h),g)
        assert gh==hg

        R=rectangle(c,g,h)
        assert diagonal_hist(R)==histogram(gh)

        # Exchange of g,h swaps middle coordinates.
        Rswap=rectangle(c,h,g)
        expected=Counter()
        for (a,b,c1,d),n in R.items():
            expected[(a,c1,b,d)] += n
        assert Rswap==expected

print("STAGE 131 — TWO-STEP RECTANGLE KERNEL / COMMUTING APPEND SQUARE")
print("="*100)
print("131A append operator:")
print("  (T_g q)_k=q_k+q_{k-g}")
print()
print("131B commuting square:")
print("  T_h T_g q = T_g T_h q")
print("  q''_k=q_k+q_{k-g}+q_{k-h}+q_{k-g-h}")
print()
print("131C rectangle table:")
print("  R_{g,h}(a,b,c,d)")
print("  counts four-corner fibre patterns")
print("  (q_k,q_{k-g},q_{k-h},q_{k-g-h})")
print()
print("131D exact two-step histogram transfer:")
print("  n''_j=sum_{a+b+c+d=j}R_{g,h}(a,b,c,d)")
print()
print("131E polynomial form:")
print("  H''(t)=J_{g,h}(t,t,t,t)")
print()
print("131F symmetry:")
print("  exchanging g and h swaps the two middle rectangle coordinates")
print()
print("131G finite audit:")
print("  commutativity + rectangle-diagonal transfer across multiple families: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  append weights commute at aggregate fibre level")
print("  AND exact prediction requires the finer four-corner lag geometry.")
print()
print("STAGE 131 RESULT : True")
