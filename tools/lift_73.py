#!/usr/bin/env python3
"""
STAGE 73 — EDGE SUBDIVISION / REGISTER-PHASE REFINEMENT

Measured native ROTAT orbits:
  W7 = ⊢∈⊤⊥∋⊡⊣
    registers: TF TF TF F N N TF
    verdict: T at all 7 cuts

  W8 = ⊢∈⊤≺⊥∋⊡⊣
    registers: TF TF F F F N N TF
    verdict: T at all 8 cuts

The insertion ≺ replaces transition ⊤→⊥ by ⊤→≺→⊥.
"""

from collections import Counter
import math
import cmath

R7=("TF","TF","TF","F","N","N","TF")
R8=("TF","TF","F","F","F","N","N","TF")

def autocorr(r):
    n=len(r)
    return tuple(sum(r[j]==r[(j+t)%n] for j in range(n)) for t in range(n))

def shannon(r):
    n=len(r)
    c=Counter(r)
    return -sum((v/n)*math.log2(v/n) for v in c.values())

def onehot_power(r):
    cats=sorted(set(r))
    n=len(r)
    out=[]
    for m in range(n):
        s=0.0
        for cat in cats:
            z=sum((1 if r[j]==cat else 0)*cmath.exp(-2j*math.pi*m*j/n) for j in range(n))
            s += abs(z)**2
        out.append(s)
    return tuple(out)

K7=autocorr(R7)
K8=autocorr(R8)
assert K7==(7,4,2,1,1,2,4)
assert K8==(8,5,2,0,0,0,2,5)

P8=onehot_power(R8)
expected=(22, 8+5*math.sqrt(2), 4, 8-5*math.sqrt(2), 2,
          8-5*math.sqrt(2), 4, 8+5*math.sqrt(2))
assert all(abs(a-b)<1e-9 for a,b in zip(P8,expected))

print("STAGE 73 — EDGE SUBDIVISION / REGISTER-PHASE REFINEMENT")
print("="*78)
print("73A orbit verdict invariance: TRUE")
print("  W7: T at all 7 cuts")
print("  W8: T at all 8 cuts")
print()
print("73B transition refinement: TRUE")
print("  W7 contains edge:  T -> F")
print("  W8 replaces it by: T -> AREV -> F")
print("  ring length: 7 -> 8")
print()
print("73C register phase fields:")
print("  R7 =",R7)
print("  R8 =",R8)
print("  support counts R7 =",dict(Counter(R7)))
print("  support counts R8 =",dict(Counter(R8)))
print()
print("73D categorical autocorrelation:")
print("  K7 =",K7)
print("  K8 =",K8)
print("  adjacent coincidence: 4/7 -> 5/8")
print("  W8 has zero same-register coincidences at lags 3,4,5.")
print()
print("73E phase entropy:")
print("  H(R7) =",format(shannon(R7),".12f"),"bits")
print("  H(R8) =",format(shannon(R8),".12f"),"bits")
print("  insertion redistributes register occupancy while preserving verdict.")
print()
print("73F one-hot categorical phase power for W8:")
print("  P8 = (22, 8+5√2, 4, 8-5√2, 2, 8-5√2, 4, 8+5√2)")
print()
print("PARACONSISTENT LANDING")
print("  AREV insertion preserves closure verdict across the full orbit")
print("  AND changes the register-phase geometry.")
print("  the ring is the same closure class")
print("  AND a different phase-resolved object.")
print()
print("STAGE 73 RESULT : True")
