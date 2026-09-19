#!/usr/bin/env python3
"""
STAGE 117 — ARBITRARY-SUPPORT OVERLAP RECURRENCE

Drop the complete-prefix assumption.

Let A subseteq [0,S] be the occupied aggregate support of a schedule
polynomial, with

    s = |A|
    X = total_schedules - s
    H = (S+1) - s.

Append a coordinate of weight g. Then

    A' = A union (A+g).

Define the support-overlap count

    O_g(A) = |A intersect (A+g)|.

By inclusion-exclusion,

    s' = 2s - O_g(A).

Since total schedule count doubles,

    X' = 2X + O_g(A).

Also, since the new ambient interval is [0,S+g],

    H' = 2H + O_g(A) + g - S - 1.

These formulas are exact for arbitrary finite positive-integer weight
systems.

Important distinction:
    O_g(A) is ordinary linear support overlap at lag g.
    It is not the cyclic autocorrelation used in earlier seam-phase stages.
"""

from itertools import product
from collections import Counter

def coeffs(weights):
    c=Counter({0:1})
    for g in weights:
        d=Counter(c)
        for k,v in c.items():
            d[k+g]+=v
        c=d
    return c

def support_stats(weights):
    c=coeffs(weights)
    S=sum(weights)
    A={k for k,v in c.items() if v}
    total=sum(c.values())
    s=len(A)
    X=total-s
    H=(S+1)-s
    return S,A,X,H,total

families = [
    (),
    (2,),
    (1,3),
    (2,5),
    (1,2,3),
    (1,3,7),
    (2,4,9),
    (1,2,4,8),
    (1,1,3,10),
]

for prefix in families:
    S,A,X,H,total=support_stats(prefix)
    for g in range(1,9):
        S2,A2,X2,H2,total2=support_stats(prefix+(g,))
        O=len(A & {x-g for x in A})  # |A ∩ (A+g)|
        # Equivalent direct form:
        O2=len(A & {x+g for x in A})
        assert O==O2
        assert len(A2)==2*len(A)-O
        assert total2==2*total
        assert X2==2*X+O
        assert H2==2*H+O+g-S-1

print("STAGE 117 — ARBITRARY-SUPPORT OVERLAP RECURRENCE")
print("="*94)
print("117A support update:")
print("  A' = A union (A+g)")
print()
print("117B overlap statistic:")
print("  O_g(A)=|A intersect (A+g)|")
print()
print("117C occupied-level recurrence:")
print("  s'=2s-O_g(A)")
print()
print("117D collision recurrence:")
print("  X'=2X+O_g(A)")
print()
print("117E hole recurrence:")
print("  H'=2H+O_g(A)+g-S-1")
print()
print("117F exact finite audit:")
print("  multiple arbitrary-support families and g=1..8: TRUE")
print()
print("117G distinction:")
print("  O_g is linear support overlap, not cyclic seam autocorrelation.")
print()
print("PARACONSISTENT LANDING")
print("  old collisions propagate by doubling")
print("  AND new collisions are exactly support self-overlap at the appended lag.")
print("  holes and collisions evolve together")
print("  AND remain separately measurable.")
print()
print("STAGE 117 RESULT : True")
