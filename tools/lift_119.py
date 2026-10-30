#!/usr/bin/env python3
"""
STAGE 119 — DYADIC COLLISION DECOMPOSITION / INSERTION HISTORY

Let weights g1,...,gm be inserted in that order.

At stage j:
    A_{j-1} = occupied support before inserting g_j
    O_j = O_{g_j}(A_{j-1})
        = |A_{j-1} intersect (A_{j-1}+g_j)|

Stage 117 gives
    X_j = 2 X_{j-1} + O_j
with X_0=0.

Unrolling:

    X_m = sum_{j=1}^m 2^{m-j} O_j.

Thus every collision created at insertion j is replicated through every
later Boolean coordinate; its final contribution is amplified by
2^(m-j).

For complete-prefix support:
    O_j = max(S_{j-1}-g_j+1,0),
which gives a direct gap/abut/overlap decomposition of final collision
excess.
"""

from collections import Counter

def support(A,g):
    return A | {a+g for a in A}

def overlap(A,g):
    return len(A & {a+g for a in A})

def direct_stats(weights):
    A={0}
    history=[]
    X=0
    S=0
    for g in weights:
        O=overlap(A,g)
        history.append(O)
        X=2*X+O
        A=support(A,g)
        S+=g
    direct=(1<<len(weights))-len(A)
    assert X==direct
    unrolled=sum((1<<(len(weights)-j-1))*O for j,O in enumerate(history))
    assert unrolled==X
    return history,X,A

families=[
    (1,2,3),
    (1,2,3,4),
    (1,2,4,8),
    (1,1,1,1),
    (2,5,9),
    (1,3,4,10),
    (1,2,5,6,10),
]
for ws in families:
    direct_stats(ws)

anchors={}
for ws in [(1,2,3),(1,2,3,4),(1,2,4,8),(1,1,1,1)]:
    h,X,A=direct_stats(ws)
    anchors[ws]=(h,X)

print("STAGE 119 — DYADIC COLLISION DECOMPOSITION / INSERTION HISTORY")
print("="*94)
print("119A local recurrence:")
print("  X_j=2 X_{j-1}+O_j")
print()
print("119B unrolled theorem:")
print("  X_m=sum_{j=1}^m 2^(m-j) O_j")
print()
print("119C interpretation:")
print("  a collision created at stage j is duplicated by every")
print("  later Boolean schedule coordinate.")
print()
print("119D native-anchor decompositions:")
for ws,(h,X) in anchors.items():
    terms=[
        f"{1<<(len(ws)-j-1)}*{O}"
        for j,O in enumerate(h)
        if O
    ]
    rhs=" + ".join(terms) if terms else "0"
    print(f"  {ws}: O={h}, X={X} = {rhs}")
print()
print("119E binary case:")
print("  (1,2,4,8) has O_j=0 at every insertion")
print("  => X=0")
print()
print("119F executable audit:")
print("  multiple complete and gapped weight systems: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  final collision excess is a single aggregate number")
print("  AND it retains an exact decomposition by the stage where")
print("  each overlap was first created.")
print()
print("STAGE 119 RESULT : True")
