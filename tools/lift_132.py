#!/usr/bin/env python3
"""
STAGE 132 — r-STEP BOOLEAN WINDOW KERNEL / HYPERCUBE TRANSFER

Let q_k be the current fibre-size sequence and let future append weights be

    G=(g1,...,gr).

Because each append operator is
    T_g = I + S_g
with
    (S_g q)_k=q_{k-g},

and the shift operators commute,

    T_{g1} ... T_{gr}
      = product_i (I+S_{g_i})
      = sum_{E subseteq [r]} S_{sum_{i in E} g_i}.

Therefore the final fibre sequence is

    q^{(G)}_k
      = sum_{E subseteq [r]}
          q_{k - sum_{i in E} g_i}.

So r future append coordinates probe a 2^r-corner Boolean window of
the original fibre sequence.

Define the Boolean-window pattern at k:

    W_G(k)
      = ( q_{k-s_E} )_{E subseteq [r]},

where
    s_E=sum_{i in E}g_i.

Let
    N_G(v)
be the number of k carrying corner-value vector v, excluding the all-zero
vector.

Then the exact final fibre histogram is

    n^{(G)}_j
      = sum_{v : sum(v)=j} N_G(v).

Equivalently, the 2^r-variate kernel specialized on the full diagonal
recovers the final histogram.

This generalizes:
    r=1 -> Stage 130 pair kernel,
    r=2 -> Stage 131 rectangle kernel.

Important:
    repeated subset-sum shifts are retained with multiplicity.
    If different subsets E,F have the same s_E=s_F, their q-contributions
    appear separately in the Boolean-window sum, exactly matching
    multiplication by product_i (1+x^{g_i}).
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

def subset_shifts(G):
    shifts=[]
    for bits in product((0,1), repeat=len(G)):
        shifts.append(sum(b*g for b,g in zip(bits,G)))
    return shifts  # keep multiplicity

def window_table(c,G):
    shifts=subset_shifts(G)
    A=set(c)
    U=set()
    for s in set(shifts):
        U |= {k+s for k in A}
    N=Counter()
    for k in U:
        vals=tuple(c[k-s] for s in shifts)
        if any(vals):
            N[vals]+=1
    return N

def diagonal_hist(N):
    out=Counter()
    for vals,n in N.items():
        out[sum(vals)]+=n
    return out

families=[
    (),
    (1,),
    (1,2,3),
    (1,2,3,4),
    (1,2,4,8),
    (1,1,1),
    (2,2,2),
    (2,5,9),
]

future_sets=[
    (1,),
    (1,2),
    (1,2,4),
    (1,1,2),  # repeated subset sums
    (2,3,5),
]

for ws in families:
    c=fibres(ws)
    for G in future_sets:
        out=c
        for g in G:
            out=append(out,g)

        # Direct hypercube sum.
        shifts=subset_shifts(G)
        A=set(c)
        U=set()
        for s in set(shifts):
            U |= {k+s for k in A}
        direct=Counter()
        for k in U:
            q=sum(c[k-s] for s in shifts)
            if q:
                direct[k]=q
        assert out==direct

        N=window_table(c,G)
        assert diagonal_hist(N)==histogram(out)

        # Order invariance of append multiset.
        out_rev=c
        for g in reversed(G):
            out_rev=append(out_rev,g)
        assert out_rev==out

print("STAGE 132 — r-STEP BOOLEAN WINDOW KERNEL / HYPERCUBE TRANSFER")
print("="*100)
print("132A operator factorization:")
print("  T_g=I+S_g")
print("  product_i T_{g_i}=sum_{E subseteq[r]} S_{sum_{i in E}g_i}")
print()
print("132B exact r-step fibre law:")
print("  q^(G)_k=sum_{E subseteq[r]} q_{k-s_E}")
print()
print("132C Boolean window:")
print("  r future weights require 2^r corner values of the original q-sequence")
print()
print("132D exact histogram transfer:")
print("  n^(G)_j=sum_{v:sum(v)=j}N_G(v)")
print()
print("132E hierarchy:")
print("  r=1 -> pair kernel (Stage 130)")
print("  r=2 -> rectangle kernel (Stage 131)")
print("  general r -> 2^r-corner hypercube kernel")
print()
print("132F repeated-shift qualification:")
print("  equal subset sums are retained with multiplicity")
print("  exactly as in product_i(1+x^{g_i})")
print()
print("132G order invariance:")
print("  final aggregate fibre sequence depends on the multiset of append weights,")
print("  not their order")
print()
print("132H finite audit:")
print("  r=1..3, including repeated subset-sum shifts: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  append dynamics commute globally")
print("  AND exact finite-horizon prediction requires exponentially finer")
print("  local lag-window state.")
print()
print("STAGE 132 RESULT : True")
