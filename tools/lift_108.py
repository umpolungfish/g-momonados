#!/usr/bin/env python3
"""
STAGE 108 — NORMAL-FORM FIBRE ENUMERATOR / SQUAREFREE WEIGHT SERIES

For a finite schedule weight list g1,...,gm, let the squarefree schedule
sector consist of all subsets E of coordinates.

The normal-form map sends
    E -> z1^tau(E)
where
    tau(E)=sum_{i in E} g_i.

Therefore the coefficient of x^k in
    Q(x)=prod_i (1+x^{g_i})
is exactly the number of squarefree schedule words whose canonical
normal form is z1^k.

This identifies the weighted traffic polynomial simultaneously as:
  - aggregate-traffic generating polynomial,
  - kernel-fibre enumerator,
  - canonical-normal-form multiplicity series
on the squarefree schedule sector.

Audits:
  Stage 94 weights (1,2,3)
  Stage 102 weights (1,2,3,4)
  equal-weight B3/B4/B5 cases.
"""

from itertools import product
from collections import Counter
from math import comb

def fibre_coeffs(weights):
    c=Counter()
    for bits in product((0,1), repeat=len(weights)):
        k=sum(b*g for b,g in zip(bits,weights))
        c[k]+=1
    return [c[k] for k in range(sum(weights)+1)]

# Stage 94
assert fibre_coeffs((1,2,3)) == [1,1,1,2,1,1,1]

# Stage 102
assert fibre_coeffs((1,2,3,4)) == [1,1,1,2,2,2,2,2,1,1,1]

# Equal-weight cubes
for n in (3,4,5):
    coeff=fibre_coeffs((1,)*n)
    assert coeff == [comb(n,k) for k in range(n+1)]

print("STAGE 108 — NORMAL-FORM FIBRE ENUMERATOR / SQUAREFREE WEIGHT SERIES")
print("="*92)
print("108A squarefree schedule sector:")
print("  E subset of {1,...,m}")
print()
print("108B canonical normal form:")
print("  NF(E)=z1^tau(E)")
print("  tau(E)=sum_{i in E} g_i")
print()
print("108C fibre enumerator:")
print("  Q(x)=prod_i (1+x^{g_i})")
print("  [x^k]Q = #{E : NF(E)=z1^k}")
print()
print("108D native-weighted anchors:")
print("  (1,2,3)   -> 1,1,1,2,1,1,1")
print("  (1,2,3,4) -> 1,1,1,2,2,2,2,2,1,1,1")
print()
print("108E equal-weight specialization:")
print("  g_i=1 => Q(x)=(1+x)^m")
print("  fibre multiplicities are binomial coefficients")
print()
print("108F interpretation:")
print("  one polynomial counts aggregate traffic classes")
print("  AND multiplicities of canonical normal forms")
print("  AND sizes of kernel-congruence fibres on the schedule sector.")
print()
print("PARACONSISTENT LANDING")
print("  canonical quotient form forgets resolved schedule position")
print("  AND its coefficient remembers how many distinct schedules were collapsed.")
print()
print("STAGE 108 RESULT : True")
