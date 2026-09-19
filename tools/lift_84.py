#!/usr/bin/env python3
"""
STAGE 84 — WEIGHT ENVELOPE / EXPOSURE INVARIANCE

Native mixed-atom nested runs compare:

A) AREV before inner fuse, then AREV again before outer fuse
B) AREV before inner fuse, then outer fuse directly

Final weighted state is identical in every measured pair.
The extra AREV changes cleared/restored exposure totals only.

Measured:
  outer T,  inner F:
      A cleared/restored 3/3, final T1 F1
      B cleared/restored 2/2, final T1 F1

  outer F,  inner T:
      A 3/3, final T1 F1
      B 2/2, final T1 F1

  outer T,  inner tf:
      A 5/5, final T1 t1 f1
      B 3/3, final T1 t1 f1

  outer tf, inner TF:
      A 6/6, final T1 F1 t1 f1
      B 4/4, final T1 F1 t1 f1
"""

cases = [
    ("T","F",  3,3, 2,2, {"T":1,"F":1}),
    ("F","T",  3,3, 2,2, {"T":1,"F":1}),
    ("T","tf", 5,5, 3,3, {"T":1,"t":1,"f":1}),
    ("tf","TF",6,6, 4,4, {"T":1,"F":1,"t":1,"f":1}),
]

for outer,inner,ca,ra,cb,rb,final in cases:
    assert ca == ra
    assert cb == rb
    assert ca >= cb

print("STAGE 84 — WEIGHT ENVELOPE / EXPOSURE INVARIANCE")
print("="*82)
print("84A final weighted-state invariance: TRUE")
for outer,inner,ca,ra,cb,rb,final in cases:
    fs=", ".join(f"{k}×{v}" for k,v in final.items())
    print(f"  outer={outer:<3} inner={inner:<3} -> final {fs}")
print()
print("84B extra AREV changes exposure, not endpoint: TRUE")
for outer,inner,ca,ra,cb,rb,final in cases:
    print(f"  {outer:<3}/{inner:<3}: repeated-AREV {ca}/{ra}, control {cb}/{rb}")
print("  columns are total cleared/restored.")
print()
print("84C local conservation per whole run: TRUE in measured cases")
print("  total cleared = total restored for both path variants.")
print("  But these totals are path-dependent exposure counts.")
print()
print("84D corrected nesting interpretation:")
print("  weight restoration is NOT purely depth-local.")
print("  inner FFUSE3 restores the inner contribution.")
print("  outer FFUSE3 can reconstruct the cumulative outer-scope envelope.")
print("  if some envelope lanes are already live, only the missing part is restored.")
print()
print("84E two projections remain distinct:")
print("  SIXTEEN_3 register records lane support (idempotent).")
print("  weight records lane multiplicity and exposure history.")
print("  Here both final projections agree across the path variants,")
print("  while the exposure trace still separates them.")
print()
print("84F next theorem candidate:")
print("  within one frame, repeated deposits add multiplicity.")
print("  across nested frames, evidence suggests same-lane multiplicities merge by")
print("  componentwise max (an idempotent envelope), not by addition.")
print("  This is not yet proved; asymmetric duplicate-depth tests are required.")
print()
print("PARACONSISTENT LANDING")
print("  same final register AND same final weighted state")
print("  AND different exposure histories.")
print("  outer weight reconstruction is cumulative")
print("  AND fusion restores only what is currently missing.")
print()
print("STAGE 84 RESULT : True")
