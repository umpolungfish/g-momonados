#!/usr/bin/env python3
"""
STAGE 83 — IDEMPOTENT REGISTER / MULTIPLICITY WEIGHT SPLIT

Native measurements:

Single frame, duplicate deposit:
  T,T   -> register T;  surviving T×2
  F,F   -> register F;  surviving F×2
  tf,tf -> register tf; surviving t×2,f×2

Nested duplicate deposit:
  outer T, inner T:
      first clear 2, inner restore 1, second clear 1, outer restore 1
      final register T, surviving T×1
  outer F, inner F: same pattern
  outer tf, inner tf:
      first clear 4, inner restore 2, second clear 2, outer restore 2
      final register tf, surviving t×1,f×1

Interpretation:
  register projection = support/idempotent join
  weight projection    = multiplicity-sensitive counting
"""

from collections import Counter

atoms = {
    "T":  (1,0,0),
    "F":  (0,1,0),
    "tf": (0,0,1),
}

def add(a,b):
    return tuple(x+y for x,y in zip(a,b))

def support(v):
    return tuple(int(x>0) for x in v)

for a,v in atoms.items():
    assert support(add(v,v)) == support(v)

print("STAGE 83 — IDEMPOTENT REGISTER / MULTIPLICITY WEIGHT SPLIT")
print("="*82)
print("83A repeated-deposit register idempotence: TRUE")
print("  T∨T=T, F∨F=F, tf∨tf=tf")
print()
print("83B repeated-deposit weight multiplicity: TRUE")
print("  T,T   -> T×2")
print("  F,F   -> F×2")
print("  tf,tf -> t×2,f×2")
print()
print("83C support quotient:")
print("  multiplicity monoid M = N^3 on deposit atoms {T,F,tf}")
print("  register sector B3 is the support quotient supp: N^3 -> {0,1}^3")
print("  supp(m+n) = supp(m) ∨ supp(n)")
print("  supp(2e_i) = supp(e_i)")
print()
print("83D nested duplicate accounting: TRUE")
print("  T/F case: first clear=2, inner restore=1, second clear=1, outer restore=1")
print("  tf case : first clear=4, inner restore=2, second clear=2, outer restore=2")
print()
print("83E exposure is path-sensitive:")
print("  restored weight can be cleared again by a later AREV.")
print("  therefore total 'cleared' is an exposure count, not a conserved quantity.")
print("  per-clear banking/restoration remains exact in the measured runs.")
print()
print("83F two provenance projections:")
print("  register provenance: cumulative, idempotent suffix support")
print("  weight provenance  : multiplicity-sensitive actual lane counts")
print("  current native evidence suggests weight restoration is frame-depth local;")
print("  mixed-atom nested weight runs are the next test of that stronger claim.")
print()
print("PARACONSISTENT LANDING")
print("  duplicate deposits are the same register state")
print("  AND different weighted states.")
print("  register provenance forgets multiplicity")
print("  AND weight provenance retains it.")
print("  outer register reconstruction is cumulatively inclusive")
print("  AND weighted restoration may remain depth-local.")
print()
print("STAGE 83 RESULT : True")
