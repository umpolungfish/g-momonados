#!/usr/bin/env python3
r"""
STAGE 118 — SUPPORT AUTOCORRELATION / OVERLAP SPECTRUM

For finite occupied aggregate support A subseteq Z_{\ge 0}, define its
indicator polynomial

    I_A(x) = sum_{a in A} x^a.

The aperiodic support autocorrelation is

    C_A(x) = I_A(x) I_A(x^{-1}).

Its coefficient at lag g is

    [x^g] C_A(x) = |A intersect (A-g)|
                 = |A intersect (A+g)|
                 = O_g(A).

Thus the Stage-117 collision recurrence

    X' = 2X + O_g(A)

can be read directly from one coefficient of the support autocorrelation.

Important:
    this is APERIODIC / LINEAR autocorrelation on integer support.
    It is not the cyclic seam autocorrelation from Stages 70–74.
"""

from itertools import product
from collections import Counter

def support(weights):
    A={0}
    for g in weights:
        A |= {a+g for a in list(A)}
    return A

def overlap(A,g):
    return len(A & {a+g for a in A})

def autocorr(A):
    c=Counter()
    for a in A:
        for b in A:
            c[a-b]+=1
    return c

families=[
    (),
    (1,),
    (2,),
    (1,2,3),
    (1,2,3,4),
    (1,2,4,8),
    (2,5,9),
    (1,3,7,20),
]

for ws in families:
    A=support(ws)
    c=autocorr(A)
    for g in range(0, max(A,default=0)+3):
        assert c[g] == overlap(A,g)
        assert c[-g] == c[g]

# Complete interval sanity check:
for S in range(0,10):
    A=set(range(S+1))
    c=autocorr(A)
    for g in range(0,S+2):
        expected=max(S-g+1,0)
        assert c[g]==expected

print("STAGE 118 — SUPPORT AUTOCORRELATION / OVERLAP SPECTRUM")
print("="*92)
print("118A support indicator:")
print("  I_A(x)=sum_{a in A} x^a")
print()
print("118B aperiodic autocorrelation:")
print("  C_A(x)=I_A(x) I_A(x^-1)")
print()
print("118C overlap coefficient:")
print("  O_g(A)=[x^g] C_A(x)")
print("        =|A intersect (A+g)|")
print()
print("118D Stage-117 recurrence:")
print("  X'=2X+[x^g]C_A(x)")
print()
print("118E complete interval:")
print("  A=[0,S] => O_g=max(S-g+1,0)")
print("  recovering Stage 116 exactly")
print()
print("118F symmetry:")
print("  O_{-g}=O_g")
print()
print("118G finite audit:")
print("  multiple arbitrary supports + complete intervals: TRUE")
print()
print("118H distinction:")
print("  this is linear/aperiodic support autocorrelation,")
print("  not cyclic seam autocorrelation.")
print()
print("PARACONSISTENT LANDING")
print("  one lag coefficient measures the next collision increment")
print("  AND the full autocorrelation records all possible append-lag overlaps.")
print()
print("STAGE 118 RESULT : True")
