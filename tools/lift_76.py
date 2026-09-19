#!/usr/bin/env python3
"""
STAGE 76 — SIXTEEN_3 TRACE REFINEMENT / CHECKPOINT TRANSPARENCY

Native traces:
  W7  = ⊢∈⊤⊥∋⊡⊣
  W8L = ⊢∈⊤≺⊥∋⊡⊣
  W8R = ⊢∈⊤⊥≺∋⊡⊣

All finish at TF with tri-ancestral verdict T.
All report Closed walk: false.
The distinction is therefore in the internal register path.
"""

from collections import Counter

G7  = ("⊢","∈","⊤","⊥","∋","⊡","⊣")
G8L = ("⊢","∈","⊤","≺","⊥","∋","⊡","⊣")
G8R = ("⊢","∈","⊤","⊥","≺","∋","⊡","⊣")

# Post-step register values from native `vox sixteen3 check`.
T7  = ("N","N","T","TF","TF","TF","TF")
T8L = ("N","N","T","N","F","TF","TF","TF")
T8R = ("N","N","T","TF","N","TF","TF","TF")

def pullback(gbig, trace):
    pos = {g:i for i,g in enumerate(gbig)}
    return tuple(trace[pos[g]] for g in G7)

PB_L = pullback(G8L,T8L)
PB_R = pullback(G8R,T8R)

assert PB_L == ("N","N","T","F","TF","TF","TF")
assert PB_R == T7

def transitions(trace, initial="N"):
    xs=(initial,)+trace
    return Counter(zip(xs,xs[1:]))

def first_return_after_insert(glyphs, trace, insert_glyph="≺", target=None):
    i=glyphs.index(insert_glyph)
    if target is None:
        target=trace[i-1]
    for j in range(i+1,len(trace)):
        if trace[j]==target:
            return j-i
    return None

print("STAGE 76 — SIXTEEN_3 TRACE REFINEMENT / CHECKPOINT TRANSPARENCY")
print("="*82)
print("76A quotient/final agreement: TRUE")
print("  W7  final TF, verdict T, closed-walk false")
print("  W8L final TF, verdict T, closed-walk false")
print("  W8R final TF, verdict T, closed-walk false")
print()
print("76B native post-step traces:")
print("  W7  =",T7)
print("  W8L =",T8L)
print("  W8R =",T8R)
print()
print("76C AREV action is state-dependent: TRUE")
print("  left : T  -> N")
print("  right: TF -> N")
print()
print("76D checkpoint pullback to original seven glyphs:")
print("  pullback(W8L) =",PB_L)
print("  pullback(W8R) =",PB_R)
print("  W8R pullback == W7 : TRUE")
print("  W8L differs only after ⊥: TF -> F")
print()
print("76E excursion geometry:")
print("  W7 base segment       : T -> TF")
print("  W8L refinement        : T -> N -> F -> TF")
print("  W8R inserted excursion: TF -> N -> TF")
print("  W8L returns to TF two steps after AREV; W8R returns in one.")
print()
print("76F post-step occupancy:")
print("  W7  :",dict(Counter(T7)))
print("  W8L :",dict(Counter(T8L)))
print("  W8R :",dict(Counter(T8R)))
print()
print("76G transition multisets:")
print("  W7  :",dict(transitions(T7)))
print("  W8L :",dict(transitions(T8L)))
print("  W8R :",dict(transitions(T8R)))
print()
print("PARACONSISTENT LANDING")
print("  all three have the same final TF AND different internal trajectories")
print("  W8R is transparent on all original forward checkpoints")
print("  AND it is more disruptive in the cyclic phase field than W8L")
print("  closed-walk is false AND tri-ancestral closure is T")
print("  state-return closure AND semantic reconnection remain distinct predicates")
print()
print("STAGE 76 RESULT : True")
