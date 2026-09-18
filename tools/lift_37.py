#!/usr/bin/env python3
"""lift_37.py - Stage 37: A/B extensional identity under the B4 algebra.

A   := ⊞ applied to the mixed cell (Stage 36 construct)
Bst := the crisp both-state  ∈⊤⊥∋  (T and F, no ⊞ provenance)

Compare through operations, not token names.  Two projections:
  truth projection  -> B4 value from {T,F} content
  info  projection  -> register mark multiset (T,F,t,f)
Verdict = extentional identity iff truth AND info agree on every context.
"""
import sys, os, re, itertools
GMO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(GMO, "tools"))
import lift_34 as L
from lift_35 import weight_fields, run_batch
import lift_reentry as R

VINIT, TANCH, AREV, ENGAGR = "\u22a2", "\u22a3", "\u227a", "\u229e"
A   = R.FSPLIT + R.EVALT + ENGAGR + R.EVALF + R.FFUSE   # ∈⊤⊞⊥∋
Bst = R.FSPLIT + R.EVALT + R.EVALF + R.FFUSE            # ∈⊤⊥∋

def marks(f):
    s = f.get("surviving") or ""
    d = {"T": 0, "F": 0, "t": 0, "f": 0}
    for c, n in re.findall(r"([TFtf])\u00d7(\d+)", s):
        d[c] += int(n)
    return d

def truth_proj(m):
    hasT, hasF = m["T"] > 0, m["F"] > 0
    return "B" if (hasT and hasF) else ("T" if hasT else ("F" if hasF else "N"))

CTX = {
    "none":    lambda w: w,
    "IFIX":    lambda w: w + R.IFIX,
    "REENTER": lambda w: R.FSPLIT + AREV + R.FFUSE + w,
    "ENGAGR":  lambda w: ENGAGR + w,
}
# ⊞-join absorption tests: join(A,x) for x in {T,F,B}
JOIN = {
    "⊞(A,T)":  R.FSPLIT + R.EVALT + ENGAGR + R.EVALF + ENGAGR + R.EVALT + R.FFUSE,
    "⊞(A,F)":  R.FSPLIT + R.EVALT + ENGAGR + R.EVALF + ENGAGR + R.EVALF + R.FFUSE,
    "⊞(A,⊞)":  R.FSPLIT + R.EVALT + ENGAGR + R.EVALF + ENGAGR + ENGAGR + R.FFUSE,
    "⊞(TF,T)": R.FSPLIT + R.EVALT + R.EVALF + ENGAGR + R.EVALT + R.FFUSE,
    "⊞(T,F)":  R.FSPLIT + R.EVALT + ENGAGR + R.EVALF + R.FFUSE,
    "⊞(T,T)":  R.FSPLIT + R.EVALT + ENGAGR + R.EVALT + R.FFUSE,
    "⊞(F,F)":  R.FSPLIT + R.EVALF + ENGAGR + R.EVALF + R.FFUSE,
}
# B4 reference (para_vm op=lattice): join row B = B B B B ; meet row B = N T F B ; ¬B=B

def main():
    print("=== Stage 37: A/B under the B4 algebra (operations, not names) ===")
    rows = []
    for bn, base in [("A", A), ("TF", Bst)]:
        for cn, fn in CTX.items():
            rows.append((bn, cn, fn(base)))
    for jn, w in JOIN.items():
        rows.append(("join", jn, w))
    out = run_batch(["weight " + w for _, _, w in rows])
    print(f"{'state':<6} {'context':<10} {'final':<6} {'truth':<6} {'T,F,t,f':<14} info")
    truth_agree = info_agree = True
    for bn, cn, w in rows:
        f = weight_fields(out, w)
        m = marks(f)
        print(f"{bn:<6} {cn:<10} {str(f.get('final')):<6} {truth_proj(m):<6} "
              f"{m['T']},{m['F']},{m['t']},{m['f']:<8} {sum(m.values())}")
    # pairwise verdict on the four shared contexts
    print()
    print("-- A vs TF on the four shared contexts --")
    for cn in CTX:
        wa = CTX[cn](A); wb = CTX[cn](Bst)
        fa, fb = weight_fields(out, wa), weight_fields(out, wb)
        ma, mb = marks(fa), marks(fb)
        t_eq = truth_proj(ma) == truth_proj(mb)
        i_eq = ma == mb
        truth_agree &= t_eq; info_agree &= i_eq
        print(f"   {cn:<9} truth: A={truth_proj(ma)} TF={truth_proj(mb)} {'=' if t_eq else '≠':<2} "
              f"| info: A={sum(ma.values())} TF={sum(mb.values())} {'=' if i_eq else '≠'}")
    print()
    print(f"VERDICT: truth-lattice agreement = {truth_agree};  info-lattice agreement = {info_agree}")
    if truth_agree and not info_agree:
        print("         PARTIAL IDENTITY:  A ≡B4 B  on truth,  A ≠struct B on information.")
        print("         (a refinement internal to the carrier, not a 5th truth value in FOUR)")

if __name__ == "__main__":
    main()
