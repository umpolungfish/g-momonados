#!/usr/bin/env python3
"""
STAGE 129 — HISTOGRAM NON-MARKOVIANITY / LAG-JOINT FIBRE TABLE

Let q_k be the aggregate fibre-size sequence.

The global fibre histogram is

    n_j = #{k : q_k=j}.

Appending a Boolean coordinate of weight g gives

    q'_k = q_k + q_{k-g}.

Therefore the next histogram depends not only on the multiset of q_k
values, but on how those values are aligned at lag g.

Define the lag-joint fibre table

    N_g(a,b)
      = #{k in A union (A+g) :
          q_k=a and q_{k-g}=b},

where A={k:q_k>0}, and (a,b)!=(0,0).

Then

    n'_j = sum_{a+b=j} N_g(a,b).

Thus N_g is sufficient for one-step histogram transfer.

The current histogram n_j is NOT sufficient in general.

Exact counterexample:
    weights (1,1,1) and (2,2,2)
have the same current fibre histogram

    {1:2, 3:2},

but after appending the same weight g=1:

    (1,1,1,1) -> {1:2, 4:2, 6:1}
    (2,2,2,1) -> {1:4, 3:4}.

So equal histogram AND different lag geometry lead to different next
histograms.
"""

from itertools import product
from collections import Counter

def fibres(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        c[sum(b*g for b,g in zip(bits,weights))]+=1
    return c

def histogram(c):
    return Counter(c.values())

def joint(c,g):
    A=set(c)
    U=A | {k+g for k in A}
    N=Counter()
    for k in U:
        a=c[k]
        b=c[k-g]
        assert a or b
        N[(a,b)] += 1
    return N

def append(c,g):
    A=set(c)
    U=A | {k+g for k in A}
    return Counter({k:c[k]+c[k-g] for k in U if c[k]+c[k-g]})

def hist_from_joint(N):
    h=Counter()
    for (a,b),n in N.items():
        h[a+b]+=n
    return h

# Counterexample.
c1=fibres((1,1,1))
c2=fibres((2,2,2))
h1=histogram(c1)
h2=histogram(c2)
assert h1==h2==Counter({1:2,3:2})

g=1
N1=joint(c1,g)
N2=joint(c2,g)
assert N1 != N2

a1=append(c1,g)
a2=append(c2,g)
ha1=histogram(a1)
ha2=histogram(a2)

assert ha1==Counter({1:2,4:2,6:1})
assert ha2==Counter({1:4,3:4})
assert ha1 != ha2

assert hist_from_joint(N1)==ha1
assert hist_from_joint(N2)==ha2

# Broad one-step sufficiency audit.
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
        assert hist_from_joint(N)==histogram(append(c,g))

print("STAGE 129 — HISTOGRAM NON-MARKOVIANITY / LAG-JOINT FIBRE TABLE")
print("="*98)
print("129A current histogram:")
print("  n_j=#{k:q_k=j}")
print()
print("129B append law:")
print("  q'_k=q_k+q_{k-g}")
print()
print("129C lag-joint table:")
print("  N_g(a,b)=#{k:q_k=a and q_{k-g}=b}, excluding (0,0)")
print()
print("129D exact histogram transfer:")
print("  n'_j=sum_{a+b=j} N_g(a,b)")
print()
print("129E counterexample:")
print("  (1,1,1) and (2,2,2) both have histogram {1:2, 3:2}")
print("  append g=1:")
print("    (1,1,1,1) -> {1:2, 4:2, 6:1}")
print("    (2,2,2,1) -> {1:4, 3:4}")
print()
print("129F conclusion:")
print("  global fibre histogram alone is not Markov-complete")
print("  lag-joint fibre table N_g is sufficient for one-step transfer")
print()
print("129G finite audit:")
print("  multiple binary, repeated, gapped, and collision-rich families: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  two systems can have identical fibre histograms")
print("  AND evolve differently under the same appended weight.")
print("  histogram forgets placement")
print("  AND N_g restores exactly the lag placement needed for transfer.")
print()
print("STAGE 129 RESULT : True")
