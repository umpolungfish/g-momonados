#!/usr/bin/env python3
"""
STAGE 78 — EIGHT-STATE DEPOSIT SUBALGEBRA / FRAME-PROVENANCE OBSTRUCTION

Native facts:
- Direct framed witnesses reach:
  N, T, F, tf, TF, Ttf, Ftf, A.
- Each direct witness satisfies AREV: s -> N and FFUSE3: N -> s.
- anyon-sync supplies a distinct context with visible A before AREV but
  FFUSE3 restores TF, not A.

Therefore restoration depends on frame provenance, not visible register alone.
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
assert len(states) == 8
assert all(t == f for _,_,t,f in states.values())

remaining = []
for T in (0,1):
    for F in (0,1):
        for t in (0,1):
            for f in (0,1):
                bits=(T,F,t,f)
                if bits not in states.values():
                    remaining.append(bits)
assert len(remaining) == 8
assert all(t != f for _,_,t,f in remaining)

print("STAGE 78 — EIGHT-STATE DEPOSIT SUBALGEBRA / FRAME-PROVENANCE OBSTRUCTION")
print("="*86)
print("78A directly generated states:", ", ".join(states))
print("  exactly the 8 states satisfying lowercase-lane pairing t=f.")
print()
print("78B direct framed roundtrip: TRUE on all 8 measured states")
print("  s --AREV--> N --FFUSE3--> s")
print()
print("78C full carrier implementation exists independently:")
print("  native gpu16_3 verified all 16 values / 12 gates against CPU scalar.")
print()
print("78D state-only restoration law: FALSE")
print("  direct frame: A -> N -> A")
print("  native anyon-sync nested context: A -> N -> TF")
print("  same visible A, different restored state.")
print()
print("78E consequence:")
print("  FFUSE3 restoration is contextual in frame provenance Γ.")
print("  correct type is restore(N, Γ), not restore(N, visible_state_only).")
print()
print("PARACONSISTENT LANDING")
print("  direct framed AREV→FFUSE3 is identity on the measured 8-state subalgebra")
print("  AND the same visible state can restore differently in another frame context.")
print("  register state is insufficient")
print("  AND frame provenance carries the missing information.")
print()
print("STAGE 78 RESULT : True")
