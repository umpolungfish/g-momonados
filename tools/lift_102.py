#!/usr/bin/env python3
"""
STAGE 102 — FOUR-WEIGHT TRANSFER CUBE / WEIGHTED COLLISION FIBRES

Native target:
  depth 5, one T lane, local multiplicities
      m=(5,4,3,2,1)
  from outermost to innermost frame.

Then suffix-max envelope norms are
      g1=5, g2=4, g3=3, g4=2, g5=1

and the four optional re-clear coordinates have weights
      (4,3,2,1).

Raw weight:
      W=15

Predicted aggregate law:
      C_E = 15 + τ(E)
      R_E =  5 + τ(E)
      Δ   = 10
where
      τ(E)=sum_{i∈E} g_i.

Generating polynomial:
      Q(x)=(1+x)(1+x^2)(1+x^3)(1+x^4)
          =1+x+x^2+2x^3+2x^4+2x^5+2x^6+2x^7+x^8+x^9+x^10

Predicted fibre sizes by extra traffic 0..10:
      1,1,1,2,2,2,2,2,1,1,1
"""

from itertools import product
from collections import Counter

weights = (1,2,3,4)  # execution order Γ5,Γ4,Γ3,Γ2
W = 15
g1 = 5

fibres = {}
for bits in product((0,1), repeat=4):
    tau = sum(b*g for b,g in zip(bits,weights))
    fibres.setdefault(tau, []).append(bits)

counts = [len(fibres.get(k,[])) for k in range(sum(weights)+1)]
assert counts == [1,1,1,2,2,2,2,2,1,1,1]
assert sum(counts) == 16

for tau, bs in fibres.items():
    C, R = W+tau, g1+tau
    assert C-R == 10

print("STAGE 102 — FOUR-WEIGHT TRANSFER CUBE / WEIGHTED COLLISION FIBRES")
print("="*88)
print("102A executable prediction:")
print("  local multiplicities m=(5,4,3,2,1)")
print("  optional envelope weights=(4,3,2,1)")
print("  W=15, g1=5, Δ=10")
print()
print("102B generating polynomial:")
print("  Q(x)=(1+x)(1+x^2)(1+x^3)(1+x^4)")
print("      =1+x+x^2+2x^3+2x^4+2x^5+2x^6+2x^7+x^8+x^9+x^10")
print()
print("102C predicted traffic fibres:")
for k in range(11):
    print(f"  τ={k:2d}: C/R={15+k}/{5+k}, fibre={counts[k]}")
print()
print("102D nontrivial collision fibres:")
for k in range(11):
    if counts[k] > 1:
        print(f"  τ={k}: {fibres[k]}")
print()
print("102E complement symmetry:")
print("  τ(E^c)=10-τ(E)")
print("  fibre counts are palindromic.")
print()
print("102F native status:")
print("  PENDING — run the exact 16-word qr3 batch.")
print()
print("STAGE 102 EXECUTABLE RESULT : True")
