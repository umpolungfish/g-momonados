#!/usr/bin/env python3
"""
STAGE 116 — LOCAL HOLE/COLLISION RECURRENCE FOR COMPLETE PREFIX SUPPORT

Assume the current schedule-weight polynomial has occupied support exactly

    A = [0,S].

Let
    X = collision excess = total schedules - |A|.

Append a new schedule coordinate of positive integer weight g.
The new occupied support is

    A' = [0,S] union [g,g+S].

Define

    O = max(S-g+1, 0)     overlap length,
    G = max(g-S-1, 0)     gap length.

Then exactly

    H' = G
    X' = 2X + O.

Three cases:

GAP:      g > S+1
          O=0, G=g-S-1
          H'=g-S-1, X'=2X

ABUT:     g = S+1
          O=G=0
          H'=0, X'=2X

OVERLAP:  g < S+1
          G=0, O=S-g+1
          H'=0, X'=2X+(S-g+1)

The coefficient multiplicities may be nontrivial; only complete support
is required for these support-level formulas.
"""

from itertools import product
from collections import Counter

def poly_coeffs(weights):
    c=Counter({0:1})
    for g in weights:
        d=Counter(c)
        for k,v in c.items():
            d[k+g]+=v
        c=d
    return c

def stats(weights):
    c=poly_coeffs(weights)
    S=sum(weights)
    supp={k for k,v in c.items() if v}
    X=sum(c.values())-len(supp)
    H=(S+1)-len(supp)
    return S,supp,X,H

tests = [
    ((1,),1),       # overlap
    ((1,),2),       # abut
    ((1,),4),       # gap
    ((1,2),1),      # overlap on collision-free complete prefix
    ((1,2,3),4),    # overlap
    ((1,2,4),8),    # abut
    ((1,1,1),1),    # overlap with pre-existing collisions
]

for prefix,g in tests:
    S,A,X,H=stats(prefix)
    assert A==set(range(S+1))
    assert H==0
    S2,A2,X2,H2=stats(prefix+(g,))
    O=max(S-g+1,0)
    G=max(g-S-1,0)
    assert H2==G
    assert X2==2*X+O

print("STAGE 116 — LOCAL HOLE/COLLISION RECURRENCE FOR COMPLETE PREFIX SUPPORT")
print("="*96)
print("116A geometry:")
print("  A=[0,S], A'=[0,S] union [g,g+S]")
print()
print("116B overlap/gap lengths:")
print("  O=max(S-g+1,0)")
print("  G=max(g-S-1,0)")
print()
print("116C exact recurrence:")
print("  H'=G")
print("  X'=2X+O")
print()
print("116D GAP:")
print("  g>S+1  -> H'=g-S-1, X'=2X")
print()
print("116E ABUT:")
print("  g=S+1  -> H'=0, X'=2X")
print()
print("116F OVERLAP:")
print("  g<S+1  -> H'=0, X'=2X+(S-g+1)")
print()
print("116G audit:")
print("  complete-prefix examples across gap/abut/overlap: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  appending one coordinate always doubles existing collision excess")
print("  AND geometric block overlap contributes exactly the new excess term.")
print("  gaps create holes")
print("  AND overlaps create collisions.")
print()
print("STAGE 116 RESULT : True")
