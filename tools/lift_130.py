#!/usr/bin/env python3
"""
STAGE 130 — BIVARIATE JOINT KERNEL / DIAGONAL HISTOGRAM TRANSFER

For the lag-joint fibre table from Stage 129,

    N_g(a,b)
      = #{k : q_k=a, q_{k-g}=b},

define the bivariate joint kernel

    J_g(u,v)
      = sum_{(a,b)!=(0,0)} N_g(a,b) u^a v^b.

This polynomial stores the complete one-step alignment data at lag g.

Appending weight g gives

    q'_k = q_k + q_{k-g}.

Therefore the next fibre-histogram polynomial

    H'(t)=sum_{j>=1} n'_j t^j

is exactly the diagonal specialization

    H'(t)=J_g(t,t).

Equivalently,

    n'_j = [t^j] J_g(t,t)
         = sum_{a+b=j} N_g(a,b).

The current histogram polynomial

    H(t)=sum_j n_j t^j

is only a marginal multiplicity summary; it does not determine J_g.

Orientation reversal satisfies

    J_{-g}(u,v)=J_g(v,u).

Thus:
    H is a global one-variable summary,
    J_g is the lag-resolved two-variable state needed for exact append transfer.
"""

from itertools import product
from collections import Counter

def fibres(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        c[sum(b*g for b,g in zip(bits,weights))]+=1
    return c

def joint(c,g):
    A=set(c)
    U=A | {k+g for k in A}
    N=Counter()
    for k in U:
        a=c[k]
        b=c[k-g]
        if a or b:
            N[(a,b)] += 1
    return N

def append(c,g):
    A=set(c)
    U=A | {k+g for k in A}
    return Counter({k:c[k]+c[k-g] for k in U if c[k]+c[k-g]})

def histogram(c):
    return Counter(c.values())

def diagonal_hist(N):
    h=Counter()
    for (a,b),n in N.items():
        h[a+b]+=n
    return h

families=[
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
    for g in range(1,8):
        N=joint(c,g)
        Nm=joint(c,-g)

        # Orientation reversal.
        swapped=Counter({(b,a):n for (a,b),n in N.items()})
        assert Nm==swapped

        # Diagonal specialization reproduces next histogram.
        assert diagonal_hist(N)==histogram(append(c,g))

# Explicit Stage-129 pair.
c1=fibres((1,1,1))
c2=fibres((2,2,2))
N1=joint(c1,1)
N2=joint(c2,1)
assert histogram(c1)==histogram(c2)
assert N1!=N2
assert diagonal_hist(N1)!=diagonal_hist(N2)

print("STAGE 130 — BIVARIATE JOINT KERNEL / DIAGONAL HISTOGRAM TRANSFER")
print("="*100)
print("130A lag-joint kernel:")
print("  J_g(u,v)=sum N_g(a,b) u^a v^b")
print()
print("130B append diagonal:")
print("  H'(t)=J_g(t,t)")
print()
print("130C coefficient form:")
print("  n'_j=[t^j]J_g(t,t)=sum_{a+b=j}N_g(a,b)")
print()
print("130D orientation reversal:")
print("  J_{-g}(u,v)=J_g(v,u)")
print()
print("130E information distinction:")
print("  H(t) records the fibre-size multiset")
print("  J_g(u,v) records lag-g alignment of those fibre sizes")
print()
print("130F Stage-129 counterexample:")
print("  equal H for (1,1,1) and (2,2,2)")
print("  AND unequal J_1")
print("  => unequal next histograms after appending g=1")
print()
print("130G finite audit:")
print("  diagonal transfer + orientation reversal across multiple families: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  one-variable histogram data can agree globally")
print("  AND two-variable lag kernels can remain distinct.")
print("  exact append dynamics are a diagonal projection of the finer kernel.")
print()
print("STAGE 130 RESULT : True")
