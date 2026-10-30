#!/usr/bin/env python3
"""
STAGE 110 — BINARY-WEIGHT COLLISION-FREE CUBE

Native target:
  local multiplicities (outer -> inner)
      m=(16,8,4,2,1)

Since these are strictly decreasing, suffix-max envelope norms are
      g1=16, g2=8, g3=4, g4=2, g5=1.

Optional re-clear weights in execution order are
      (1,2,4,8).

Raw deposit weight:
      W=31

For every schedule E:
      tau(E) in {0,...,15} uniquely
      C_E = 31 + tau(E)
      R_E = 16 + tau(E)
      Delta = 15.

Generating polynomial:
      Q(x)=(1+x)(1+x^2)(1+x^4)(1+x^8)
          =1+x+x^2+...+x^15
          =(1-x^16)/(1-x).

Thus every aggregate fibre has size 1:
resolved schedule -> aggregate traffic is injective on this cube.
"""

from itertools import product

weights=(1,2,4,8)
seen={}
for bits in product((0,1), repeat=4):
    t=sum(b*g for b,g in zip(bits,weights))
    assert t not in seen
    seen[t]=bits

assert sorted(seen)==list(range(16))

for t in range(16):
    C,R=31+t,16+t
    assert C-R==15

print("STAGE 110 — BINARY-WEIGHT COLLISION-FREE CUBE")
print("="*88)
print("110A executable prediction:")
print("  local multiplicities m=(16,8,4,2,1)")
print("  optional weights=(8,4,2,1)")
print("  execution-order weights=(1,2,4,8)")
print()
print("110B raw/envelope:")
print("  W=31, g1=16, Delta=15")
print()
print("110C generating polynomial:")
print("  Q(x)=(1+x)(1+x^2)(1+x^4)(1+x^8)")
print("      =1+x+x^2+...+x^15")
print()
print("110D predicted native traffic classes:")
for t in range(16):
    print(f"  tau={t:2d}: C/R={31+t}/{16+t}, fibre=1")
print()
print("110E injectivity:")
print("  every one of the 16 schedules has a distinct aggregate C/R pair")
print()
print("110F native status:")
print("  PENDING")
print()
print("STAGE 110 EXECUTABLE RESULT : True")
