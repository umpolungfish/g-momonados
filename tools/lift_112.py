#!/usr/bin/env python3
"""
STAGE 112 — MINIMAL LOSSLESS TRAFFIC BUDGET / BINARY OPTIMALITY

Let positive integer weights be
    1 <= g1 < g2 < ... < gm
and encode squarefree schedules E subseteq {1,...,m} by
    tau(E) = sum_{i in E} g_i.

If tau is injective on all 2^m schedules, then the subset sums are
2^m distinct nonnegative integers in the interval [0,S], where
    S = sum_i g_i.

Therefore
    S + 1 >= 2^m
so
    S >= 2^m - 1.

Binary weights
    (1,2,4,...,2^(m-1))
attain equality:
    S = 2^m - 1
and the subset sums are exactly {0,...,2^m-1}.

Moreover, equality forces the binary system uniquely (for sorted positive
integer weights): if the first k weights are 1,2,...,2^(k-1), then their
subset sums fill [0,2^k-1]. To preserve injectivity with no unused span
when adding g_{k+1}, the next block must begin exactly at 2^k, hence
g_{k+1}=2^k.

This file audits the theorem by exhaustive search for m<=5 over sorted
positive integer weights within a modest bound.
"""

from itertools import combinations, product

def subset_sums(ws):
    sums=[]
    for bits in product((0,1), repeat=len(ws)):
        sums.append(sum(b*w for b,w in zip(bits,ws)))
    return sums

def injective(ws):
    ss=subset_sums(ws)
    return len(set(ss))==len(ss)

def binary(m):
    return tuple(1<<i for i in range(m))

for m in range(1,6):
    b=binary(m)
    S=sum(b)
    assert S == (1<<m)-1
    ss=sorted(subset_sums(b))
    assert ss == list(range(1<<m))

# Exhaustive optimality search for small m.
for m,bound in [(1,4),(2,8),(3,12),(4,18),(5,34)]:
    best=None
    winners=[]
    for ws in combinations(range(1,bound+1), m):
        if injective(ws):
            s=sum(ws)
            if best is None or s<best:
                best=s
                winners=[ws]
            elif s==best:
                winners.append(ws)
    assert best == (1<<m)-1
    assert winners == [binary(m)]

print("STAGE 112 — MINIMAL LOSSLESS TRAFFIC BUDGET / BINARY OPTIMALITY")
print("="*92)
print("112A counting lower bound:")
print("  injective 2^m subset sums lie in {0,...,S}")
print("  therefore S+1 >= 2^m")
print("  hence S >= 2^m-1")
print()
print("112B binary attainment:")
print("  g_i=2^(i-1)")
print("  S=2^m-1")
print("  subset sums are exactly 0,...,2^m-1")
print()
print("112C equality uniqueness:")
print("  among sorted positive integer weights, equality forces")
print("  (1,2,4,...,2^(m-1))")
print()
print("112D exhaustive audit:")
print("  m=1..5: binary weights are the unique minimum-sum")
print("  injective systems within the searched bounds: TRUE")
print()
print("112E native Stage-110 anchor:")
print("  m=4 binary weights (1,2,4,8)")
print("  S=15=2^4-1")
print("  all 16 aggregate traffic classes measured distinctly")
print()
print("PARACONSISTENT LANDING")
print("  lossless aggregate coding needs enough traffic span")
print("  AND binary weighting achieves the exact minimum span.")
print()
print("STAGE 112 RESULT : True")
