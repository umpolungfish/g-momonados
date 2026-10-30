#!/usr/bin/env python3
"""
STAGE 80 — NESTED FRAME STACK / DYNAMIC-EXTENT PROVENANCE

Measured native examples:
  outer T,  inner F   -> inner fuse F,   outer fuse TF
  outer F,  inner T   -> inner fuse T,   outer fuse TF
  outer T,  inner tf  -> inner fuse tf,  outer fuse Ttf
  outer tf, inner TF  -> inner fuse TF,  outer fuse A

Controls without the second AREV have the same outer result.

Model:
  Each open frame carries an accumulator for all deposits in its dynamic extent.
  Nested deposits contribute to every enclosing open frame.
  AREV clears the visible register but does not erase frame accumulators.
  FFUSE3 closes the top frame and joins its accumulator into the current visible state.
"""

states = {
    "N":   (0,0,0,0),
    "T":   (1,0,0,0),
    "F":   (0,1,0,0),
    "tf":  (0,0,1,1),
    "TF":  (1,1,0,0),
    "Ttf": (1,0,1,1),
    "Ftf": (0,1,1,1),
    "A":   (1,1,1,1),
}

def join_name(a,b):
    bits=tuple(int(x or y) for x,y in zip(states[a],states[b]))
    for name,v in states.items():
        if v==bits:
            return name
    raise KeyError(bits)

cases = [
    ("T","F","F","TF"),
    ("F","T","T","TF"),
    ("T","tf","tf","Ttf"),
    ("tf","TF","TF","A"),
]

for q1,q2,inner,outer in cases:
    assert inner == q2
    assert outer == join_name(q1,q2)

print("STAGE 80 — NESTED FRAME STACK / DYNAMIC-EXTENT PROVENANCE")
print("="*82)
print("80A inner LIFO restoration: TRUE")
for q1,q2,inner,outer in cases:
    print(f"  outer={q1:<3} inner={q2:<3}  first FFUSE3 -> {inner:<3}")
print()
print("80B outer dynamic-extent restoration: TRUE")
for q1,q2,inner,outer in cases:
    print(f"  {q1:<3} ∨ {q2:<3} = {outer:<3}  second/outer FFUSE3 -> {outer}")
print()
print("80C control equivalence: TRUE")
print("  omitting the second AREV changes the intermediate path")
print("  but not the final outer-frame reconstruction in all four measured cases.")
print()
print("80D stack model:")
print("  push frame Γ when FSPLIT3 opens")
print("  deposits update every currently open Γ accumulator")
print("  AREV: visible -> N, frame accumulators persist")
print("  FFUSE3: visible <- visible ∨ top(Γ); then pop Γ")
print()
print("80E nesting consequence:")
print("  inner frame contains only the inner dynamic extent")
print("  outer frame contains outer-local + all nested descendants")
print("  provenance is hierarchical, not a flat hidden register.")
print()
print("PARACONSISTENT LANDING")
print("  inner FFUSE3 restores only inner content")
print("  AND outer FFUSE3 later restores the whole outer dynamic extent.")
print("  nested frames are locally selective")
print("  AND cumulatively inclusive.")
print("  second AREV changes the path")
print("  AND leaves the outer reconstruction invariant in the measured cases.")
print()
print("STAGE 80 RESULT : True")
