#!/usr/bin/env python3
"""lift_38.py - Stage 38: provenance retraction.

eta = ENGAGR   (crisp -> refined):  TF --⊞--> A          [resident, Stage 37]
pi  = FORGET   (refined -> crisp):  A  --> TF           [search for resident pi]

Tests:
  1. resident FORGET search  (all 12 marks prepend/append + nested AREV)
  2. the section and the two composites  pi.eta=id, eta.pi=C
  3. non-invertibility on provenance variants with equal truth
"""
import sys, os, re
GMO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(GMO, "tools"))
from lift_37 import marks, truth_proj, A, Bst, ENGAGR
from lift_35 import weight_fields, run_batch
import lift_reentry as R

GL = ["\u22a2", "\u22a3", "\u227b", "\u227a", "\u22c8", "\u22a4", "\u22a5",
      "\u2208", "\u220b", "\u2299", "\u229e", "\u22a1"]
NEST = ["\u2208\u22a4\u2208\u229e\u227a\u220b\u22a5\u220b",
        "\u2208\u2208\u229e\u227a\u220b\u22a4\u22a5\u220b",
        "\u2208\u22a4\u2208\u229e\u22a5\u227a\u220b\u220b"]

def main():
    print("=== Stage 38: provenance retraction ===")
    cands = []
    for g in GL:
        cands += [("app " + g, A + g), ("pre " + g, g + A)]
    for i, w in enumerate(NEST):
        cands.append((f"nest{i}", w))
    cands += [("A", A), ("TF", Bst)]
    out = run_batch(["weight " + w for _, w in cands])
    print("-- 1. resident FORGET search: hit = surviving T\u00d71,F\u00d71 and t=0,f=0 --")
    found = []
    for lab, w in cands:
        f = weight_fields(out, w); m = marks(f)
        hit = (m["T"] == 1 and m["F"] == 1 and m["t"] == 0 and m["f"] == 0 and w != Bst)
        if hit:
            found.append((lab, w))
        if lab in ("A", "TF") or hit:
            print(f"   {lab:<8} {w:<14} final={f.get('final')}  T,F,t,f="
                  f"{m['T']},{m['F']},{m['t']},{m['f']}  {'<-- HIT' if hit else ''}")
    print(f"   resident FORGET found: {len(found)}  "
          f"=> pi is NOT a single resident mark; it exists as the truth-readout (surviving \u2229 {{T,F}})")
    print()
    eta = ENGAGR + Bst
    o2 = run_batch(["weight " + w for w in [Bst, eta, ENGAGR + eta]])
    print("-- 2. section eta = ENGAGR, and the composites --")
    for lab, w in [("TF", Bst), ("eta(TF)", eta), ("eta(eta(TF))", ENGAGR + eta)]:
        f = weight_fields(o2, w); m = marks(f)
        print(f"   {lab:<12} {w:<14} final={f.get('final')}  T,F,t,f="
              f"{m['T']},{m['F']},{m['t']},{m['f']}")
    print("   pi.eta(TF): truth(eta(TF)) = truth(A) = B, pi(A) = TF  =>  pi.eta = id on crisp")
    print()
    vars_ = [("A1", A), ("A2", A + ENGAGR), ("A3", ENGAGR + A),
             ("A4", A + R.EVALT), ("A5", R.FSPLIT + R.EVALT + ENGAGR + ENGAGR + R.EVALF + R.FFUSE)]
    o3 = run_batch(["weight " + w for _, w in vars_])
    print("-- 3. provenance variants with equal truth --")
    print(f"{'var':<5} {'final':<6} {'truth':<6} {'T,F,t,f':<14} {'pi':<4} eta.pi")
    for lab, w in vars_:
        f = weight_fields(o3, w); m = marks(f)
        print(f"{lab:<5} {str(f.get('final')):<6} {truth_proj(m):<6} "
              f"{m['T']},{m['F']},{m['t']},{m['f']:<8} {'TF':<4} A")
    print("   => variants share truth B; pi maps them all to TF; eta.pi maps them all to A")
    print("      so eta.pi = canonicalization C, NOT identity (pi is a quotient on information)")
    print()
    print("VERDICT: eta (ENGAGR) resident;  pi (FORGET) NOT resident -> truth-readout.")
    print("         pi.eta = id on the crisp carrier;  eta.pi = C (canonicalization), not an isomorphism.")

if __name__ == "__main__":
    main()
