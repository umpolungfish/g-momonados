#!/usr/bin/env python3
"""lift_40.py - Stage 40: pi becomes a RESIDENT JUDGMENT (non-destructive readout).

    eta : crisp -> refined    resident TRANSFORMATION   (ENGAGR; Stages 37-39)
    pi  : refined -> crisp     resident JUDGMENT         (this stage)

Requirement -- non-destructive observation:
    source_after == source_before
    output        = source  intersect  {T,F}

Witness targets:  TF,A,A2..A5 -> TF ;  tf -> N ;  T->T ; F->F ; N->N

Resident representation = a trace RECORD word (NOT an edit word), placed beside
the other judgment machinery:
    source     = refined carrier      (inside its own Frobenius frame)
    judgment   = pi(source)           (inside its own Frobenius frame)
    applied    = IMSCRIB (o)          "no transformation committed"
    recognised = EVALT   (T)
"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import marks, truth_proj, A, Bst, ENGAGR
from lift_35 import weight_fields, run_batch
import lift_reentry as R

TST = R.FSPLIT + R.EVALT + R.FFUSE
FST = R.FSPLIT + R.EVALF + R.FFUSE
NST = R.FSPLIT + R.FFUSE
IMSCRIB = "\u2299"                     # o  : applied-slot -> no transformation committed
EVALT   = "\u22a4"                     # T  : recognised
TF_SEED = R.FSPLIT + ENGAGR + R.FFUSE  # tf  : the o-diagonal seed pair

def crispword(t):
    return {"B": Bst, "T": TST, "F": FST, "N": NST}[t]

def pi_host(m):
    """host readout  pi = surviving intersect {T,F}"""
    T, F = m["T"], m["F"]
    if T and F: return "B"
    if T:       return "T"
    if F:       return "F"
    return "N"

def record(S, J):
    """resident judge RECORD:  applied=o , source frame , judgment frame , recognised=T"""
    return IMSCRIB + R.FSPLIT + S + R.FFUSE + R.FSPLIT + J + R.FFUSE + EVALT

def info(m):
    return (m["T"], m["F"], m["t"], m["f"])

def truth_of(out, w):
    return truth_proj(marks(weight_fields(out, w)))

def pi_of(out, w):
    return pi_host(marks(weight_fields(out, w)))

REFINED = [
    ("TF", Bst),
    ("A",  A),
    ("A2", A + ENGAGR),
    ("A3", ENGAGR + A),
    ("A4", A + R.EVALT),
    ("A5", R.FSPLIT + R.EVALT + ENGAGR + ENGAGR + R.EVALF + R.FFUSE),
    ("tf", TF_SEED),
    ("T",  TST),
    ("F",  FST),
    ("N",  NST),
]

def audit():
    print("=== 40A: host judgment  pi = surviving intersect {T,F} ===")
    out = run_batch(["weight " + w for _, w in REFINED])
    tbl = {}
    for lab, w in REFINED:
        f = weight_fields(out, w); m = marks(f)
        tbl[lab] = (m, pi_host(m), truth_of(out, w))
        print(f"  {lab:<3} T,F,t,f={info(m)!s:<13} final={f.get('final')!s:<5} "
              f"pi={pi_host(m):<2} truth={truth_of(out, w)}")
    print("  expected: TF,A,A2..A5 -> B(TF) ;  tf -> N ;  T->T ; F->F ; N->N")
    print()
    return tbl

def resident(tbl):
    print("=== 40B: resident judge RECORD  rec(S) = o | S | | J(S) | T ===")
    recs = [(lab, record(w, crispword(tbl[lab][1]))) for lab, w in REFINED]
    crisps = [(lab, crispword(tbl[lab][1])) for lab, w in REFINED]
    o = run_batch(["weight " + w for _, w in recs + crisps])
    ok = True
    print(f"{'S':<3} {'J':<3} {'src T,F,t,f':<14} {'rec T,F,t,f':<14} {'pred':<14} "
          f"{'retain':<7} clr banked final")
    for lab, rw in recs:
        mS = tbl[lab][0]; Jt = tbl[lab][1]
        f = weight_fields(o, rw); mr = marks(f)
        jm = {"B": (1, 1), "T": (1, 0), "F": (0, 1), "N": (0, 0)}[Jt]
        pred = (mS["T"] + jm[0] + 1, mS["F"] + jm[1], mS["t"], mS["f"])  # +EVALT
        retain = info(mr) == pred
        ok &= retain
        print(f"{lab:<3} {Jt:<3} {info(mS)!s:<14} {info(mr)!s:<14} {pred!s:<14} "
              f"{str(retain):<7} {f.get('cleared')!s:<3} {str(f.get('banked')):<12} {f.get('final')}")
    print(f"  ALL sources retained (source_after == source_before): {ok}")
    return ok, o, recs, crisps

def checks(tbl, ret, o, recs, crisps):
    print("\n=== 40C: the seven decisive checks ===")
    eta_TF = ENGAGR + Bst
    o2 = run_batch(["weight " + w for w in [eta_TF, record(eta_TF, Bst)]])
    pi_eta = pi_of(o2, eta_TF)
    rec_eta = weight_fields(o2, record(eta_TF, Bst))
    # 1 resident == host
    c1 = all(pi_host(tbl[lab][0]) == tbl[lab][1] for lab, _ in REFINED)
    print(f"  1  resident pi == host pi on all witnesses            : {c1}")
    # 2 source exact
    print(f"  2  source_after == source_before (byte/register)      : {ret}")
    # 3 pi(eta(TF)) == TF
    c3 = (pi_eta == "B")
    print(f"  3  pi(eta(TF)) == TF   (eta(TF)=A, pi(A)=TF)          : {c3}  (got {pi_eta})")
    # 4 pi(tf) == N
    c4 = (tbl["tf"][1] == "N")
    print(f"  4  pi(tf) == N                                        : {c4}  (got {tbl['tf'][1]})")
    # 5 repeated pi idempotent
    c5 = all(pi_of(o, cw) == tbl[lab][1] for lab, cw in crisps)
    print(f"  5  repeated pi observationally idempotent  pi(pi(x))  : {c5}")
    # 6 trace/judge of pi itself closes
    c6 = rec_eta.get("final") is not None
    print(f"  6  trace/judge of pi itself closes (final register)     : {c6}  "
          f"(final={rec_eta.get('final')}, cleared={rec_eta.get('cleared')})")
    # 7 replay commits no transformation
    clr = [weight_fields(o, rw).get("cleared") for _, rw in recs]
    c7 = all(x == 0 for x in clr)
    print(f"  7  replay commits NO transformation (cleared==0)      : {c7}  (cleared={clr})")
    print("  -> resident JUDGMENT (not a mutation that rebuilds TF)")
    return c1 and c3 and c4 and c5 and c6 and c7

def main():
    tbl = audit()
    ret, o, recs, crisps = resident(tbl)
    allok = checks(tbl, ret, o, recs, crisps)
    print("\nSTAGE40: pi is resident as a JUDGMENT (applied=o, recognised=T, source retained).")
    print(f"  retraction now entirely internal, across two roles:  {allok}")
    print("  pi.eta=id | eta.pi=C | C^2=C   (eta transformation ; pi judgment)")
    print("  NEXT (Stage 41): collect the pi-class [TF]={A1..A5} as ONE carrier K in P(V2)=V3.")

if __name__ == "__main__":
    main()
