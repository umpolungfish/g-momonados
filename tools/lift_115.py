#!/usr/bin/env python3
"""
STAGE 115 — PREFIX BLOCK GEOMETRY / GAP-OVERLAP-ABUT TRICHOTOMY

Let sorted positive integer weights be
    g1 <= g2 <= ... <= gm.

Suppose the subset sums of the first k weights fill the complete interval

    [0,S_k],  S_k=sum_{i<=k} g_i.

Adding the next weight g creates a translated block

    g + [0,S_k] = [g, g+S_k].

Relative to the old block [0,S_k], exactly three cases occur:

1. GAP:
       g > S_k+1
   The new block starts after a gap.

2. ABUT:
       g = S_k+1
   The two blocks touch with no gap and no overlap.

3. OVERLAP:
       g < S_k+1
   The blocks overlap, forcing aggregate collisions.

Therefore complete coverage of [0,S] occurs iff
    g1=1
and
    g_{k+1} <= 1 + sum_{i<=k} g_i
for every k.

Binary weights are the exact ABUT case at every step:
    1,2,4,8,...

Hence they simultaneously have
    no holes
and
    no collisions.

Equal-weight/Pascal systems are repeated OVERLAP after the first weight.
"""

from itertools import product

def subset_sums(ws):
    out={0}
    for g in ws:
        out |= {x+g for x in list(out)}
    return out

def complete(ws):
    ss=subset_sums(ws)
    return ss == set(range(sum(ws)+1))

def criterion(ws):
    ws=tuple(sorted(ws))
    if not ws or ws[0] != 1:
        return False
    S=ws[0]
    for g in ws[1:]:
        if g > S+1:
            return False
        S += g
    return True

# Exact criterion audit over sorted multisets in a finite box.
from itertools import combinations_with_replacement
for m in range(1,6):
    for ws in combinations_with_replacement(range(1,9), m):
        assert complete(ws) == criterion(ws)

# Native anchors.
assert complete((1,2,3))
assert complete((1,2,3,4))
assert complete((1,2,4,8))
assert complete((1,1,1,1))

# Binary: every step abuts.
S=1
for g in (2,4,8,16):
    assert g == S+1
    S += g

# Equal weights: overlap after first.
S=1
for g in (1,1,1,1):
    assert g < S+1
    S += g

print("STAGE 115 — PREFIX BLOCK GEOMETRY / GAP-OVERLAP-ABUT TRICHOTOMY")
print("="*96)
print("115A prefix block:")
print("  old subset-sum interval = [0,S]")
print("  new translated block    = [g,g+S]")
print()
print("115B trichotomy:")
print("  g>S+1  -> GAP")
print("  g=S+1  -> ABUT")
print("  g<S+1  -> OVERLAP")
print()
print("115C complete-coverage theorem:")
print("  subset sums fill [0,sum g_i]")
print("  iff g1=1 and g_{k+1}<=1+sum_{i<=k} g_i for every k")
print("  exhaustive audit for multisets m<=5, weights<=8: TRUE")
print()
print("115D binary specialization:")
print("  every step is ABUT")
print("  => no holes AND no collisions")
print()
print("115E Pascal/equal-weight specialization:")
print("  after the first coordinate every step is OVERLAP")
print("  => complete coverage AND growing collision fibres")
print()
print("115F Stage-102 specialization:")
print("  weights (1,2,3,4) maintain complete coverage")
print("  with later overlap, matching its five collision-excess units")
print()
print("PARACONSISTENT LANDING")
print("  complete traffic coverage does not imply injectivity")
print("  AND exact abutment gives both complete coverage and injectivity.")
print()
print("STAGE 115 RESULT : True")
