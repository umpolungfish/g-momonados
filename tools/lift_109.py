#!/usr/bin/env python3
"""
STAGE 109 — SUBSET-SUM COLLISION CRITERION / SIGNED KERNEL RELATIONS

For squarefree schedules on positive weights g1,...,gm:

    tau(E) = sum_{i in E} g_i.

Two schedules E,F collide iff tau(E)=tau(F).

Equivalently, after cancelling E∩F,

    sum_{i in E\F} g_i = sum_{j in F\E} g_j.

Equivalently there is a signed relation

    sum_i eps_i g_i = 0

with eps_i in {-1,0,+1}, not all zero.

Thus:
  aggregate injectivity on the squarefree schedule cube
  iff there is no nontrivial {-1,0,+1}-relation among the weights.

This file audits the criterion on native-weighted examples:
  (1,2,3)   collision-rich
  (1,2,3,4) collision-rich
and binary weights:
  (1,2,4,8) collision-free.
"""

from itertools import product
from collections import defaultdict

def tau(bits, weights):
    return sum(b*g for b,g in zip(bits,weights))

def fibres(weights):
    d=defaultdict(list)
    for b in product((0,1), repeat=len(weights)):
        d[tau(b,weights)].append(b)
    return d

def signed_zero_relations(weights):
    rel=[]
    for eps in product((-1,0,1), repeat=len(weights)):
        if all(e==0 for e in eps):
            continue
        if sum(e*g for e,g in zip(eps,weights))==0:
            rel.append(eps)
    return rel

families = {
    (1,2,3): True,
    (1,2,3,4): True,
    (1,2,4,8): False,
}

for ws, expect_collision in families.items():
    f=fibres(ws)
    has_collision=any(len(v)>1 for v in f.values())
    rel=signed_zero_relations(ws)
    assert has_collision == bool(rel)
    assert has_collision == expect_collision

# Explicit native relations.
assert (-1,-1,1) in signed_zero_relations((1,2,3))  # 3=1+2
assert (-1,0,-1,1) in signed_zero_relations((1,2,3,4))  # 4=1+3

# Binary weights unique subset sums.
fb=fibres((1,2,4,8))
assert len(fb)==16
assert all(len(v)==1 for v in fb.values())
assert sorted(fb)==list(range(16))

print("STAGE 109 — SUBSET-SUM COLLISION CRITERION / SIGNED KERNEL RELATIONS")
print("="*92)
print("109A collision criterion:")
print("  tau(E)=tau(F)")
print("  iff sum(E\\F)=sum(F\\E)")
print()
print("109B signed-relation form:")
print("  collision iff exists nonzero eps_i in {-1,0,+1}")
print("  with sum_i eps_i g_i = 0")
print()
print("109C native-weighted anchors:")
print("  (1,2,3): collisions TRUE; witness 3=1+2")
print("  (1,2,3,4): collisions TRUE; multiple signed relations")
print()
print("109D collision-free example:")
print("  (1,2,4,8): no nontrivial signed zero relation")
print("  all 16 subset sums are unique")
print()
print("109E injectivity theorem:")
print("  squarefree aggregate traffic is injective")
print("  iff the weights are dissociated over coefficients {-1,0,+1}.")
print()
print("PARACONSISTENT LANDING")
print("  equal aggregate traffic is exactly a signed weight relation")
print("  AND absence of such relations makes the aggregate quotient faithful.")
print()
print("STAGE 109 RESULT : True")
