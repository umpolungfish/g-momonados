#!/usr/bin/env python3
"""
STAGE 74 — CUT GAUGE / GLYPH-ROOTED EDIT TRANSPORT

W7 = ⊢ ∈ ⊤ ⊥ ∋ ⊡ ⊣
W8 = ⊢ ∈ ⊤ ≺ ⊥ ∋ ⊡ ⊣

Native phase fields:
  R7 = TF TF TF F N N TF
  R8 = TF TF F  F F N N TF

The insertion is between ⊤ and ⊥.  Absolute-index comparison sees a phase
shift after the insertion.  Glyph-rooted transport compares the same surviving
glyph in W7 and W8.
"""

from collections import Counter
from itertools import product

G7=("⊢","∈","⊤","⊥","∋","⊡","⊣")
G8=("⊢","∈","⊤","≺","⊥","∋","⊡","⊣")
R7=("TF","TF","TF","F","N","N","TF")
R8=("TF","TF","F","F","F","N","N","TF")

# Same absolute index, exactly the native shorter-orbit interference comparison.
abs_pairs=list(zip(R7,R8[:7]))
abs_hits=[i for i,(a,b) in enumerate(abs_pairs) if a==b]
assert abs_hits == [0,1,3,5]

# Canonical surviving-glyph embedding after inserting ≺ between ⊤ and ⊥.
phi=tuple(G8.index(g) for g in G7)
assert phi == (0,1,2,4,5,6,7)
transported=tuple(R8[i] for i in phi)
glyph_hits=[i for i,(a,b) in enumerate(zip(R7,transported)) if a==b]
glyph_misses=[(G7[i],R7[i],transported[i]) for i in range(7) if R7[i]!=transported[i]]
assert glyph_hits == [0,1,3,4,5,6]
assert glyph_misses == [("⊤","TF","F")]

# Best order-preserving 7-of-8 deletion alignment with no cyclic relabeling.
best=[]
for drop in range(8):
    r=R8[:drop]+R8[drop+1:]
    score=sum(a==b for a,b in zip(R7,r))
    best.append((score,drop,G8[drop],r))
best_score=max(x[0] for x in best)
assert best_score == 6

print("STAGE 74 — CUT GAUGE / GLYPH-ROOTED EDIT TRANSPORT")
print("="*78)
print("74A native absolute-cut interference: TRUE")
print("  coincidences =",len(abs_hits),"/ 7")
print("  matching cuts =",abs_hits)
print()
print("74B canonical edit transport: TRUE")
print("  insertion: ⊤ -> ⊥  becomes  ⊤ -> ≺ -> ⊥")
print("  surviving-glyph embedding phi =",phi)
print("  pullback R8|phi =",transported)
print("  agreement with R7 =",len(glyph_hits),"/ 7")
print("  unique semantic mismatch =",glyph_misses[0])
print()
print("74C optimal one-vertex contraction bound: TRUE")
print("  best possible order-preserving 7-of-8 agreement =",best_score,"/ 7")
print("  therefore the 6/7 transported agreement is optimal.")
print()
print("74D locality:")
print("  same-glyph cuts unchanged at ⊢, ∈, ⊥, ∋, ⊡, ⊣")
print("  only ⊤ changes: TF -> F")
print("  absolute indexing makes the edit look global; structural transport makes it local.")
print()
print("PARACONSISTENT LANDING")
print("  direct phase comparison says 4/7")
print("  AND glyph-rooted transport says 6/7")
print("  the insertion globally reindexes cuts")
print("  AND locally changes only one surviving glyph-rooted register")
print("  C7 and C8 are combinatorially different rings")
print("  AND one is an edge subdivision of the other")
print()
print("STAGE 74 RESULT : True")
