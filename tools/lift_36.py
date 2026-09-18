#!/usr/bin/env python3
"""lift_36.py - Stage 36: the morphism table as ONE Belnap/FOUR carrier.

bit_s = [Δsubstantial > 0]  (structural face)
bit_w = [deposits   > 0]    (weight face)
outer geometry fixed: dp=+1, du=-1.

36A: empirical carrier table -- pack every FW/CF/ID witness into (bit_s,bit_w),
     report which of the four cells are OCCUPIED.
36B: action table -- apply the grammar actions to each cell representative and
     read how (bit_s,bit_w) transforms.
36C: literal ENGAGR (⊞) closure -- build the 2-cell candidate, apply ⊞, measure
     whether it lands on the diagonal without erasing provenance.
"""
import subprocess, sys, os, re, itertools
GMO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(GMO, "tools"))
import lift_34 as L
from lift_35 import weight_fields, run_batch
import lift_reentry as R

VINIT, TANCH, AREV, ENGAGR = "\u22a2", "\u22a3", "\u227a", "\u229e"

def image(w, Rk):
    return L.MORPH[Rk](w)

def cell(w, Rk, out):
    d = L.dsigma(w, Rk)
    if d is None:
        return None
    f = weight_fields(out, image(w, Rk))
    return (1 if d[1] > 0 else 0, 1 if (f.get("deposits") or 0) > 0 else 0, f)

# ---- 36A: empirical carrier table -----------------------------------------
def carrier_table():
    print("=== Stage 36A: the carrier table (bit_s, bit_w) ===")
    groups = {"FRAME_WORK": L.FW_WITNESSES, "CLOSE_FRAME": L.CF_WITNESSES,
              "IDENTITY": L.ID_WITNESSES}
    imgs = [image(w, Rk) for Rk, ws in groups.items() for _, w in ws]
    out = run_batch(["weight " + w for w in imgs])
    occ = {Rk: set() for Rk in groups}
    for Rk, ws in groups.items():
        print(f"-- {Rk} --")
        for label, w in ws:
            c = cell(w, Rk, out)
            if c is None:
                print(f"   {label:<22} UNDEFINED (off-domain)")
                continue
            occ[Rk].add((c[0], c[1]))
            print(f"   {label:<22} (bit_s,bit_w)=({c[0]},{c[1]})  deposits={c[2].get('deposits')}"
                  f"  surviving={c[2].get('surviving')}  final={c[2].get('final')}")
    allc = set()
    print()
    print("-- occupied cells per class --")
    for Rk in groups:
        print(f"   {Rk:<12} -> {sorted(occ[Rk])}")
        allc |= occ[Rk]
    missing = sorted(set(itertools.product([0, 1], [0, 1])) - allc)
    print(f"   UNION      -> {sorted(allc)}   MISSING: {missing}")
    return occ

# ---- state-level carrier (the image word's own reading) --------------------
def state_cell(w, out):
    s = L.sigma(w)
    f = weight_fields(out, w)
    b_s = 1 if s["substantial"] > 0 else 0
    b_w = 1 if (f.get("deposits") or 0) > 0 else 0
    return ((b_s, b_w), f.get("deposits"), f.get("surviving"), f.get("final"), s["substantial"])

def _lift(w):
    try:
        r = R.apply_lift("IDENTITY", w, w)
        if isinstance(r, tuple):
            r = r[0]
        return r if isinstance(r, str) else w
    except Exception:
        return w

ACTIONS = {
    "FRAME_WORK":  lambda w: R.frame_work(w),
    "CLOSE_FRAME": lambda w: R.close_frame(w),
    "IDENTITY":    lambda w: R.identity(w),
    "IFIX":        lambda w: w + R.IFIX,
    "REENTER":     lambda w: R.FSPLIT + AREV + R.FFUSE + w,
    "LIFT":        _lift,
}

# ---- 36B: action table on cell representatives (two batched boots) --------
def action_table():
    print("=== Stage 36B: how the grammar actions transform each cell ===")
    groups = {"FRAME_WORK": L.FW_WITNESSES, "CLOSE_FRAME": L.CF_WITNESSES,
              "IDENTITY": L.ID_WITNESSES}
    imgs = [image(w, Rk) for Rk, ws in groups.items() for _, w in ws]
    out = run_batch(["weight " + w for w in imgs])
    reps = {}
    for Rk, ws in groups.items():
        for _, w in ws:
            iw = image(w, Rk)
            reps.setdefault(state_cell(iw, out)[0], iw)
    base_cells = sorted(reps)
    targets, twords = {}, []
    for act, fn in ACTIONS.items():
        for c in base_cells:
            try:
                wn = fn(reps[c])
            except Exception:
                wn = None
            if not isinstance(wn, str):
                wn = reps[c]
            targets[(act, c)] = wn
            twords.append(wn)
    tout = run_batch(["weight " + w for w in twords])
    print(f"{'cell':<8} " + "  ".join(f"{a:<11}" for a in ACTIONS))
    for c in base_cells:
        row = [str(state_cell(targets[(act, c)], tout)[0]) for act in ACTIONS]
        print(f"{str(c):<8} " + "  ".join(f"{v:<11}" for v in row))
    print("  (entry = cell after the action; Belnap realization needs the map on")
    print("   {(0,0),(1,1)} to match the T/F/N/B action table, not merely to exist)")

# ---- 36C: literal ENGAGR closure ------------------------------------------
def dep(x):
    return R.EVALT if x else R.EVALF

def cellword(bs, bw):
    return R.FSPLIT + dep(bs) + ENGAGR + dep(bw) + R.FFUSE

def engager():
    print("=== Stage 36C: literal ENGAGR (⊞) closure of the 2-cell candidate ===")
    words = []
    for bs, bw in itertools.product([0, 1], [0, 1]):
        cw = cellword(bs, bw)
        words += [cw, cw + R.IFIX, R.FSPLIT + AREV + R.FFUSE + cw]
    out = run_batch(["weight " + w for w in words])
    print(f"{'cell':<8} {'⊞ final':<9} {'deposits':<9} {'surviving':<11} {'IFIX-stable':<12} {'reentry final'}")
    diag = {}
    for bs, bw in itertools.product([0, 1], [0, 1]):
        cw = cellword(bs, bw)
        f = weight_fields(out, cw)
        fi = weight_fields(out, cw + R.IFIX)
        fr = weight_fields(out, R.FSPLIT + AREV + R.FFUSE + cw)
        diag[(bs, bw)] = f.get("final")
        stable = "yes" if f.get("final") == fi.get("final") else "no"
        print(f"{str((bs,bw)):<8} {str(f.get('final')):<9} {str(f.get('deposits')):<9} "
              f"{str(f.get('surviving')):<11} {stable:<12} {fr.get('final')}")
    print()
    print("-- ⊞-diagonal reading --")
    for k in sorted(diag):
        print(f"   ⊞{k} -> {diag[k]}")
    cws = [cellword(a, b) for a, b in itertools.product([0, 1], [0, 1])]
    vw = L.surface(cws)
    print("-- criterion 5: vox vs SIXTEEN_3 on the candidate cellwords --")
    for k in cws:
        print(f"   {k}  ->  (vox,S3)=({vw[k][0]},{vw[k][2]})  weight_final={weight_fields(out,k).get('final')}")
    return diag

def main():
    carrier_table(); print()
    action_table(); print()
    engager(); print()
    print("STAGE36: read the occupied-cell union and the ENGAGR diagonal above.")

if __name__ == "__main__":
    main()
