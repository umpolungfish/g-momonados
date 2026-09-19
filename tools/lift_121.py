#!/usr/bin/env python3
"""
STAGE 121 — MULTIPLICITY AUTOCORRELATION / COLLISION ENERGY

Let
    Q(x)=sum_k q_k x^k
be the weighted schedule polynomial, where q_k is the number of schedules
with aggregate weight k.

Define the multiplicity autocorrelation

    M(x)=Q(x)Q(x^-1).

Its lag-d coefficient is

    D_d = [x^d]M(x) = sum_k q_k q_{k-d},

which counts ordered schedule pairs (E,F) with

    tau(E)-tau(F)=d.

At zero lag,

    D_0 = sum_k q_k^2

counts ordered equal-traffic pairs, including the diagonal E=F.

If N=2^m is the number of schedules, then

    ordered distinct collision pairs = D_0 - N
    unordered distinct collision pairs = (D_0-N)/2.

This is finer than collision excess

    X=sum_k max(q_k-1,0).

When all nontrivial fibres have size 2, unordered collision-pair count
equals X. Larger fibres separate the two statistics.
"""

from itertools import product
from collections import Counter

def coeffs(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        t=sum(b*g for b,g in zip(bits,weights))
        c[t]+=1
    return c

def diff_spectrum(c):
    D=Counter()
    for a,qa in c.items():
        for b,qb in c.items():
            D[a-b]+=qa*qb
    return D

def stats(weights):
    c=coeffs(weights)
    D=diff_spectrum(c)
    N=1<<len(weights)
    energy=D[0]
    X=sum(max(q-1,0) for q in c.values())
    unordered=(energy-N)//2
    assert energy==sum(q*q for q in c.values())
    assert 2*unordered==energy-N
    return c,D,N,energy,X,unordered

anchors = {
    (1,2,3): (10,1,1),
    (1,2,3,4): (26,5,5),
    (1,2,4,8): (16,0,0),
    (1,1,1,1): (70,11,27),
}
for ws,(energy,X,pairs) in anchors.items():
    c,D,N,e,x,p=stats(ws)
    assert (e,x,p)==(energy,X,pairs)
    for d,v in D.items():
        assert D[-d]==v

print("STAGE 121 — MULTIPLICITY AUTOCORRELATION / COLLISION ENERGY")
print("="*94)
print("121A multiplicity polynomial:")
print("  Q(x)=sum_k q_k x^k")
print()
print("121B difference spectrum:")
print("  M(x)=Q(x)Q(x^-1)")
print("  D_d=[x^d]M counts ordered pairs with tau(E)-tau(F)=d")
print()
print("121C zero-lag energy:")
print("  D_0=sum_k q_k^2")
print()
print("121D collision-pair counts:")
print("  ordered distinct = D_0-2^m")
print("  unordered distinct = (D_0-2^m)/2")
print()
print("121E anchors:")
for ws in anchors:
    c,D,N,e,x,p=stats(ws)
    print(f"  {ws}: D0={e}, X={x}, unordered collision pairs={p}")
print()
print("121F distinction:")
print("  Stage-102 (1,2,3,4): X=5 and pair-count=5")
print("  because every nontrivial fibre has size 2.")
print("  Equal-weight (1,1,1,1): X=11 but pair-count=27")
print("  because larger fibres contain many internal schedule pairs.")
print()
print("PARACONSISTENT LANDING")
print("  X measures excess schedules beyond one representative per fibre")
print("  AND D_0 measures pairwise collision energy inside those fibres.")
print()
print("STAGE 121 RESULT : True")
