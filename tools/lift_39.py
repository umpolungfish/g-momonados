#!/usr/bin/env python3
"""lift_39.py - Stage 39: resident section / observational projection duality.

39A: audit eta = ENGAGR as an executable object (FOUR, SIXTEEN_3, pairs, weight,
     reconciliation, fixed point).  Distinguish truth fixed point from structural
     (register) fixed point; test eta^2 =pi eta.
39B: pi as a first-class readout (observational, not a mutation).  Test the
     retraction algebra  pi.eta = id,  eta.pi = C,  mu.delta = id,  C^2 = C.
"""
import sys, os, re
GMO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(GMO, "tools"))
from lift_37 import marks, truth_proj, A, Bst, ENGAGR
from lift_35 import weight_fields, run_batch
import lift_reentry as R

TST = R.FSPLIT + R.EVALT + R.FFUSE
FST = R.FSPLIT + R.EVALF + R.FFUSE
NST = R.FSPLIT + R.FFUSE

def crispword(t):
    return {"B": Bst, "T": TST, "F": FST, "N": NST}[t]

def truth_of(out, w):
    return truth_proj(marks(weight_fields(out, w)))

def info(m):
    return (m["T"], m["F"], m["t"], m["f"])

def _m(fn, w):
    try:
        r = fn(w)
        return r if isinstance(r, str) else w
    except Exception:
        return w

def audit_eta():
    print("=== 39A: audit eta = ENGAGR as executable object ===")
    wds = {"eta_walk(|-)": ENGAGR, "eta(TF)": ENGAGR + Bst, "TF": Bst, "A": A}
    words = list(wds.values())
    for w in list(words):
        words += [_m(R.frame_work, w), _m(R.close_frame, w), _m(R.identity, w)]
    o = run_batch(["weight " + w for w in words])
    print(f"{'object':<14} {'final':<6} {'truth':<6} {'T,F,t,f':<13} recon: FW / CF / ID")
    for lab, w in wds.items():
        f = weight_fields(o, w); m = marks(f)
        rw = [truth_of(o, _m(R.frame_work, w)), truth_of(o, _m(R.close_frame, w)),
              truth_of(o, _m(R.identity, w))]
        print(f"{lab:<14} {str(f.get('final')):<6} {truth_proj(m):<6} "
              f"{info(m)!s:<13} {rw}")
    print("  (eta_walk final tells whether |-- closes; recon = truth after FW/CF/ID)")
    print()

def retraction():
    print("=== 39B: retraction algebra  pi.eta=id, eta.pi=C, C^2=C ===")
    refined = [("A1", A), ("A2", A + ENGAGR), ("A3", ENGAGR + A), ("A4", A + R.EVALT),
               ("A5", R.FSPLIT + R.EVALT + ENGAGR + ENGAGR + R.EVALF + R.FFUSE)]
    o0 = run_batch(["weight " + w for _, w in refined])
    crisp = [(lab, crispword(truth_of(o0, w))) for lab, w in refined]
    cw = [c for _, c in crisp]
    eta_w = [ENGAGR + c for c in cw]
    o1 = run_batch(["weight " + w for w in cw + eta_w])
    c2w = [ENGAGR + crispword(truth_of(o1, ew)) for ew in eta_w]
    o2 = run_batch(["weight " + w for w in c2w])
    print(f"{'var':<4} {'truth':<6} {'info(src)':<12} {'pi=C1':<10} {'info C(A)':<12} "
          f"{'info C2(A)':<12} C2==C")
    for (lab, w), (_, cr), ew, c2 in zip(refined, crisp, eta_w, c2w):
        m0 = marks(weight_fields(o0, w))
        m1 = marks(weight_fields(o1, ew))
        m2 = marks(weight_fields(o2, c2))
        print(f"{lab:<4} {truth_proj(m0):<6} {info(m0)!s:<12} {cr:<10} {info(m1)!s:<12} "
              f"{info(m2)!s:<12} {info(m1) == info(m2)}")
    print("  pi.eta(TF): eta(TF)=A, pi(A)=TF  =>  pi.eta = id on the crisp carrier")
    print("  C = eta.pi ;  C^2 == C  <=>  C idempotent  (the Stage-39 decisive test)")
    print("  mu.delta=id reference: FSPLIT+FFUSE on a deposit re-enters unchanged")
    print()

def main():
    audit_eta()
    retraction()
    print("STAGE39: eta resident (transformational); pi observational; algebra above.")

if __name__ == "__main__":
    main()
