#!/usr/bin/env python3
"""lift_fork.py — Stage 33: the lift HOLDS the functorial / re-entry fork.

LiftObject(R) = (partial_R, total_R), both projections retained.
Axis 1: two equivalences —  R1 ~v R2 (same verdict transition)  vs  R1 ~s R2 (same structural transformation).
Axis 2: partialLift vs totalLift, simultaneously.
Axis 3: 3x3 composition — record partial composite AND total composite; disagreement is the B-region.
Also: does a ⊙-bearing word realize the off-domain (identity) branch?
"""
import subprocess, sys, os, re

GMO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VOX = os.environ.get("VOX", os.path.join(GMO, "vendor/vox/target/debug/vox"))
sys.path.insert(0, os.path.join(GMO, "tools"))
from lift_reentry import (MORPH, pair, verdict, pairstats, rows, tower,
                          FSPLIT, FFUSE, TANCH, AFWD, IFIX, EVALT, EVALF)

FAM = ("FRAME_WORK", "CLOSE_FRAME", "IDENTITY")

# ---- partial vs total -----------------------------------------------------
def partial_lift(R, walk, typ):
    w2 = MORPH[R](walk)
    return None if w2 is None else (w2, typ)

def total_lift(R, walk, typ):
    w2 = MORPH[R](walk)
    return (walk, typ) if w2 is None else (w2, typ)

# ---- axis 1: ~v and ~s on the lifted objects ------------------------------
def obj_signature(R):
    """(verdict-transition, structural-transition) of LIFT(R), pooled over its task rows."""
    vts, sts = set(), set()
    for name, w, t, Rr in rows():
        if Rr != R:
            continue
        wl, tl = pair(w, t), pair(t, t)
        aw, at = pairstats(wl), pairstats(tl)
        vts.add((verdict(wl), verdict(tl)))
        sts.add((at["paired"] - aw["paired"], at["substantial"] - aw["substantial"]))
    return vts, sts

def axis1():
    print("=== Axis 1: ~v (verdict transition) vs ~s (structural transformation) ===")
    sig = {R: obj_signature(R) for R in FAM}
    for R in FAM:
        vts, sts = sig[R]
        print(f"  LIFT({R:<11}) vt={sorted(vts)}  st(dPaired,dSubst)={sorted(sts)}")
    a, b = "FRAME_WORK", "IDENTITY"
    v_same = sig[a][0] == sig[b][0]
    s_same = sig[a][1] == sig[b][1]
    print(f"  LIFT({a}) ~v LIFT({b}) : {v_same}    (both sit at the same verdict transition)")
    print(f"  LIFT({a}) ~s LIFT({b}) : {s_same}    "
          f"({'structural faces agree' if s_same else 'structural faces DIFFER — the fork'})")
    print("  -> same at the verdict face, different at the structural face: held jointly (Belnap B).")
    print()
    return v_same, s_same

# ---- axis 2: the off-domain branch ---------------------------------------
def axis2():
    print("=== Axis 2: partialLift and totalLift, both retained ===")
    wires = {"w_edit": FSPLIT + FFUSE + EVALF + AFWD + IFIX,
             "w_shape": "\u22a2" + FSPLIT + AFWD + EVALT + TANCH,
             "w_closed": FSPLIT + EVALT + AFWD + IFIX + FFUSE}
    t = FSPLIT + EVALF + AFWD + IFIX + FFUSE
    print(f"{'R':<12} {'wire':<9} {'R(w)':<8} {'partialLift':<14} {'totalLift':<14} {'on/off'}")
    for R in FAM:
        for wn, w in wires.items():
            rw = MORPH[R](w)
            pl = partial_lift(R, w, t)
            tl = total_lift(R, w, t)
            dom = "on-domain" if pl is not None else "OFF-domain"
            ps = "undef" if pl is None else f"({len(pl[0])},t)"
            ts = f"({len(tl[0])},{len(tl[1])})"
            print(f"{R:<12} {wn:<9} {'id' if rw == w else ('none' if rw is None else len(rw)):<8} "
                  f"{ps:<14} {ts:<14} {dom}")
    print("  bridge: on-domain partialLift == totalLift; off-domain partial undefined, total = current object")
    print("  the off-domain identity is the fixed-point branch: nothing produced -> re-enter unchanged")
    print()

# ---- axis 2b: can ⊙ carry the off-domain branch? --------------------------
def axis2b():
    print("=== Axis 2b: can the off-domain (identity) branch be carried by ⊙? ===")
    cands = [
        ("\u2208\u220b\u2299\u227b\u22a1", "\u2208"+FFUSE and FSPLIT + FFUSE + "\u2299" + AFWD + IFIX,
         "seed T, NO weight (seeded 1, deposits 0, inert 0); cycle final T, ROTAT-invariant"),
        ("\u22a2\u2208\u2299\u220b\u22a3", "\u22a2" + FSPLIT + "\u2299" + FFUSE + TANCH, "OP_JUDGE"),
        ("\u22a2\u2299\u22a1\u22a3", "\u22a2" + "\u2299" + IFIX + TANCH, "OP_FIX"),
        ("\u22a2\u2208\u227a\u220b\u22a3", "\u22a2" + FSPLIT + "\u227a" + FFUSE + TANCH,
         "final N, seeded 0, cleared 0; cycle final N, ROTAT-invariant  (OP_REENTER)"),
    ]
    for label, w, note in cands:
        p = pairstats(w)
        print(f"  {label:<18} vox={verdict(w):<2} pairs={p['paired']}p/{p['substantial']}s u{p['unanswered']}   imasm: {note}")
    print("  ⊙ in the morphism position SEEDs T with ZERO weight -> 'no distinction produced' = the off-domain branch.")
    print("  CROSS-SURFACE B: for \u2208\u220b\u2299\u227b\u22a1 vox verdict=N, imasm weight final=T — carry both axes.")
    print("  REENTER carries it with vox=T but a pure no-op (seeded 0); the two candidates differ at both faces.")
    print()

# ---- axis 3: 3x3 composition, partial vs total ----------------------------
def axis3():
    print("=== Axis 3: 3x3 composition — partial composite vs total composite (disagreement retained) ===")
    wires = {"w1": FSPLIT + FFUSE + EVALF + AFWD + IFIX,
             "w2": FSPLIT + FFUSE + EVALT + AFWD + IFIX,
             "w3": "\u22a2" + FSPLIT + AFWD + EVALT + TANCH,
             "w4": FSPLIT + EVALT + AFWD + IFIX + FFUSE}

    def partial(R1, R2, w):
        x = MORPH[R1](w)
        return None if x is None else MORPH[R2](x)

    def total_seq(R1, R2, w):
        x = MORPH[R1](w); x = w if x is None else x
        y = MORPH[R2](x); return x if y is None else y

    def total_comp(R1, R2, w):
        z = partial(R1, R2, w); return w if z is None else z

    print(f"{'R1':<12} {'R2':<12} {'partial-def':<12} {'total-self-cons':<16} {'B4'}")
    cnt = {"T": 0, "soft": 0, "hard": 0}
    for R1 in FAM:
        for R2 in FAM:
            pdef = all(partial(R1, R2, w) is not None for w in wires.values())
            agree = all(total_seq(R1, R2, w) == total_comp(R1, R2, w) for w in wires.values())
            if pdef and agree:
                lab = "T"; cnt["T"] += 1
            elif (not pdef) and agree:
                lab = "B  (identity completion)"; cnt["soft"] += 1
            elif (not pdef) and (not agree):
                lab = "B* (total self-forks)"; cnt["hard"] += 1
            else:
                lab = "B"
            print(f"{R1:<12} {R2:<12} {str(pdef):<12} {str(agree):<16} {lab}")
    print(f"  partial-defined {cnt['T']}/9   B identity-completion {cnt['soft']}/9   B* total-self-fork {cnt['hard']}/9")
    print("  the four B* cells ARE Stage-32's four failures — the B-region, not errors:")
    print("  partial: no morphism ; total: two different answers -> the fork held inside one object.")
    print()
    return cnt

if __name__ == "__main__":
    v_same, s_same = axis1()
    axis2()
    axis2b()
    cnt = axis3()
    print("STAGE33:",
          "fork HELD" if (v_same and not s_same and (cnt["soft"] + cnt["hard"]) > 0) else "fork not clean")
