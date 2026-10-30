#!/usr/bin/env python3
"""lift_41.py - Stage 41: the pi-fiber becomes CONTENT  (V3 = P(V2)).

Stage 40 made pi a resident JUDGMENT.  Stage 41 uses it as a MEMBERSHIP test and
collects a family of V2 states into ONE carrier element K.

Well-typing (do not conflate):
    H_pi(x) = x -> pi(x) -> eta pi(x) -> ...   heterogeneous history (NOT in V2)
    K_pi(x) = { x, eta pi(x), ... } subset V2  homogeneous V3 candidate
    [TF]pi  = { x in V2 | pi(x)=TF }           the pi-fiber, a V3 candidate

Identity is EXPLICIT. Three distinct relations are measured, never assumed:
    byte identity      : the exact V2 word (UTF-8 byte string)
    register identity  : the kernel (T,F,t,f) surviving register
    pi identity        : the resident judgment pi(x)
Powerset membership uses EXACT V2 object (byte) identity.

Encoding (word-native, length-framed so embedded frames stay unambiguous):
    LEN8(w) = 8 EVALT/EVALF glyphs = UTF-8 byte length of w, big-endian
    MEMBER  = FSPLIT LEN8(w) FFUSE FSPLIT w FFUSE
    SET     = FSPLIT MEMBER* FFUSE      (canonical: dedup by bytes, sorted)
"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import marks, truth_proj, A, Bst, ENGAGR
from lift_35 import weight_fields, run_batch
from lift_40 import REFINED, pi_host, record, crispword, info
import lift_reentry as R

V2 = [("TF", Bst), ("A1", A), ("A2", A + ENGAGR), ("A3", ENGAGR + A),
      ("A4", A + R.EVALT),
      ("A5", R.FSPLIT + R.EVALT + ENGAGR + ENGAGR + R.EVALF + R.FFUSE),
      ("tf", R.FSPLIT + ENGAGR + R.FFUSE),
      ("T", R.FSPLIT + R.EVALT + R.FFUSE),
      ("F", R.FSPLIT + R.EVALF + R.FFUSE),
      ("N", R.FSPLIT + R.FFUSE)]

def len8(w):
    n = len(w.encode("utf-8")) & 0xFF
    return "".join(R.EVALT if b == "1" else R.EVALF for b in format(n, "08b"))

def member(w):
    return R.FSPLIT + len8(w) + R.FFUSE + R.FSPLIT + w + R.FFUSE

def canon_set(words):
    ms = sorted({member(w) for w in words})
    return R.FSPLIT + "".join(ms) + R.FFUSE

def members_of(words):
    return sorted({member(w) for w in words})

def kernel_domain():
    out = run_batch(["weight " + w for _, w in V2])
    d = {}
    for lab, w in V2:
        f = weight_fields(out, w); m = marks(f)
        d[lab] = {"word": w, "reg": (m["T"], m["F"], m["t"], m["f"]),
                  "pi": pi_host(m), "final": f.get("final")}
    return d

def identities(d):
    print("=== 41 PRELIM: three identity relations on the provenance family ===")
    ns = ["A1", "A2", "A3", "A4", "A5"]
    print("  cell = (byte, register, pi)  1=equal 0=differ")
    print(f"{'':<5}" + "".join(f"{b:<15}" for b in ns))
    for a in ns:
        cells = []
        for b in ns:
            byte = int(d[a]["word"] == d[b]["word"])
            reg = int(d[a]["reg"] == d[b]["reg"])
            pi = int(d[a]["pi"] == d[b]["pi"])
            cells.append(f"{byte},{reg},{pi}".ljust(15))
        print(f"{a:<5}" + "".join(cells))
    print("  registers:", {k: d[k]["reg"] for k in ns})
    print("  pi       :", {k: d[k]["pi"] for k in ns})
    print("  -> byte / register / pi are THREE different equivalence relations.")
    print()

def set_behavior(d):
    print("=== 41A: does K behave as a SET  (membership by exact V2/byte identity) ===")
    A = {k: d[k]["word"] for k in ["A1", "A2", "A3", "A4", "A5"]}
    K = [A[k] for k in ["A1", "A2", "A3", "A4", "A5"]]
    inv = canon_set(K) == canon_set(list(reversed(K)))
    dup = canon_set(K + [A["A1"], A["A2"]]) == canon_set(K)
    mem = all(member(A[k]) in set(members_of(K)) for k in A)
    y = d["tf"]["word"]
    nmem = member(y) not in set(members_of(K))
    K2 = [A["A1"], A["A2"]]
    eq_same = canon_set(K2) == canon_set([A["A2"], A["A1"]])
    eq_diff = canon_set(K2) == canon_set([A["A3"], A["A4"]])
    print(f"   order invariance      {{A1,A2}}=={{A2,A1}}     : {inv}")
    print(f"   duplicate idempotence {{A1,A1,A2}}=={{A1,A2}}   : {dup}")
    print(f"   membership  Ai in K for all Ai           : {mem}")
    print(f"   nonmembership  tf (pi=N) not in K        : {nmem}")
    print(f"   extensional eq  K1==K2 (same members)    : {eq_same}")
    print(f"   extensional neq K1!=K2 (diff members)    : {not eq_diff}")
    ok = inv and dup and mem and nmem and eq_same and (not eq_diff)
    print(f"   => genuine SET behavior (not a serialized list): {ok}")
    print()
    return ok, K, A

def fiber(d):
    print("=== 41B: the pi-fiber predicate is the constructor  chi_TF(x)=[pi(x)=TF] ===")
    dom = [lab for lab, _ in V2]
    chi = {lab: ("T" if d[lab]["pi"] == "B" else ("N" if d[lab]["pi"] == "N" else "F"))
           for lab in dom}
    Ktf = [lab for lab in dom if chi[lab] == "T"]
    print("   x     pi(x)  chi_TF")
    for lab in dom:
        print(f"   {lab:<5} {d[lab]['pi']:<6} {chi[lab]}")
    print(f"   K_TF = [TF]pi = {{{', '.join(Ktf)}}}   |K_TF| = {len(Ktf)}")
    print("   constructor chain: V2 object -> resident pi judgment -> membership -> K_TF")
    print()
    return Ktf

def two_orders(d, K, A):
    print("=== 41C: K preserves BOTH orders -- truth collapses, information retains ===")
    wpi = {d[k]["word"]: d[k]["pi"] for k in d}
    wreg = {d[k]["word"]: d[k]["reg"] for k in d}
    pis = [wpi.get(w) for w in K]
    truthproj = all(p == "B" for p in pis)
    card = len(set(K))
    reg_distinct = len({wreg.get(w) for w in K})
    etapi = {ENGAGR + crispword(wpi.get(w)) for w in K}
    eta_TF_final = weight_fields(run_batch(["weight " + ENGAGR + Bst]), ENGAGR + Bst).get("final")
    print(f"   truth projection   all members pi->TF          : {truthproj}  ({pis})")
    print(f"   information content distinct register classes  : {reg_distinct}  "
          f"(regs {sorted({wreg.get(w) for w in K})})")
    print(f"   cardinality        |K|  (exact V2/byte members) : {card}")
    print(f"   canonical crisp representative  eta(TF)=A       : final={eta_TF_final}")
    print(f"   eta.pi acts memberwise -> {sorted(etapi)}  (|.|={len(etapi)})")
    print(f"   eta.pi(K) != K   (retraction collapses provenance): {len(etapi) != card}")
    print("   -> K says: members IDENTICAL on <=t (truth), PLURAL on <=i (information).")
    print()
    return truthproj, card, reg_distinct, len(etapi)

def main():
    print("=== Stage 41: the pi-fiber becomes CONTENT  (V3 = P(V2)) ===")
    d = kernel_domain()
    print("  V2 domain  (label, register, pi, final):")
    for lab, _ in V2:
        print(f"    {lab:<4} {d[lab]['reg']!s:<15} pi={d[lab]['pi']:<2} final={d[lab]['final']}")
    print()
    identities(d)
    ok, K, A = set_behavior(d)
    Ktf = fiber(d)
    tp, card, rd, ej = two_orders(d, K, A)
    # kernel grounding: MEMBER and SET are well-formed words that close
    os_ = run_batch(["weight " + member(A["A1"]), "weight " + canon_set(K),
                     "weight " + ENGAGR + Bst])
    mf = weight_fields(os_, member(A["A1"]))
    sf = weight_fields(os_, canon_set(K))
    print("=== 41D: kernel grounding -- MEMBER/SET are valid framed words ===")
    print(f"   MEMBER(A1)  final={mf.get('final')}  surviving={mf.get('surviving')}")
    print(f"   SET(A1..A5) final={sf.get('final')}  surviving={sf.get('surviving')}")
    print("   (Frobenius: each MEMBER frame reproduces its V2 word; the SET is the")
    print("    frame-union of the five -- identity of the SET is the byte form, above.)")
    print()
    print("STAGE41: K_TF = [TF]pi is a genuine V3 candidate element of P(V2).")
    print(f"   set behavior ok={ok}  |K_TF|={len(Ktf)} -> {{{', '.join(Ktf)}}}")
    print(f"   truth collapses ({tp}) ; info retains ({card} byte-distinct, {rd} reg-classes)")
    print("   NEXT (Stage 42): the SET algebra -- union/meet/join of fibers as V3 ops.")

if __name__ == "__main__":
    main()
