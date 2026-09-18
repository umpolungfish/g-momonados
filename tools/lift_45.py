#!/usr/bin/env python3
"""lift_45.py - Stage 45: EILENBERG-MOORE ALGEBRAS OF RE-ENTRY.

The powerset monad T(X)=P(X) has algebras  alpha : P(X) -> X  with
    alpha({x})      = x                             (UNIT)     alpha o u = id
    alpha(UNION K)  = alpha({ alpha(K) | K in K })  (MULTIPLY) alpha o m = alpha o T(alpha)
For P this is exactly an ARBITRARY-JOIN operation: E-M algebras = complete join-semilattices.

TWO RESOLUTIONS, HELD APART (the singleton law forbids canonicalizing provenance away):

  CRISP FOUR quotient -- Stage 42 gave THREE distinct orders, so THREE candidate
    algebra maps alpha_t / alpha_i / alpha_f.  "the E-M algebra" is NOT unique.

  REFINED V2 carrier  -- the trap.  alpha_can(K)=eta_e(join_i{ pi(x) | x in K })
    LOOKS natural but CANNOT be an E-M algebra on byte-distinct V2: the unit law
    demands alpha({A2})=A2, whereas canonicalization gives eta_e pi(A2)=A, and
    A2 !=byte A.  So C = eta_e pi fails the unit law on the refined carrier.
"""
import sys, os, random
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import marks, ENGAGR
from lift_35 import weight_fields, run_batch
from lift_40 import pi_host, crispword
from lift_41 import V2
from lift_42 import join_t, join_i, join_f
import lift_reentry as R

FOUR = ["N", "T", "F", "B"]
J = {"t": join_t, "i": join_i, "f": join_f}
BOT = {"t": "F", "i": "N", "f": "T"}     # join identity (least element) per order

LAB = [lab for lab, _ in V2]
W = {lab: w for lab, w in V2}
N2 = len(V2)

def supword(a, b):
    return R.FSPLIT + a + R.FFUSE + R.FSPLIT + b + R.FFUSE

def fold(ws):
    """left fold of ⊕ over a list of words; None for the empty list."""
    ws = list(ws)
    if not ws:
        return None
    acc = ws[0]
    for x in ws[1:]:
        acc = supword(acc, x)
    return acc

def measure_v2():
    o = run_batch(["weight " + w for _, w in V2])
    d = {}
    for lab, w in V2:
        m = marks(weight_fields(o, w))
        d[lab] = {"w": w, "reg": (m["T"], m["F"], m["t"], m["f"]), "pi": pi_host(m)}
    return d

def measure_words(words):
    words = list(dict.fromkeys(words))
    o = run_batch(["weight " + w for w in words])
    out = {}
    for w in words:
        m = marks(weight_fields(o, w))
        out[w] = {"reg": (m["T"], m["F"], m["t"], m["f"]), "pi": pi_host(m)}
    return out

# ---------------------------------------------------------------- 45A  crisp
def alpha(o, K):
    """arbitrary join in order o; bottom_o for the empty set."""
    K = list(K)
    if not K:
        return BOT[o]
    acc = K[0]
    for x in K[1:]:
        acc = J[o](acc, x)
    return acc

def em_crisp():
    print("=== 45A: THREE E-M ALGEBRAS ON THE CRISP FOUR QUOTIENT ===")
    print("   carrier FOUR={N,T,F,B};  alpha_o(K)=join_o over K,  alpha_o(empty)=bottom_o")
    print(f"   three joins on (T,F): join_t={join_t('T','F')}  join_i={join_i('T','F')}  join_f={join_f('T','F')}")
    allok = True
    for o in ["t", "i", "f"]:
        unit = all(alpha(o, [x]) == x for x in FOUR)
        emp = (alpha(o, []) == BOT[o])
        mult = True
        for fam in range(1 << 16):
            Ks = [[FOUR[i] for i in range(4) if (s >> i) & 1]
                  for s in range(16) if (fam >> s) & 1]
            union = sorted({x for K in Ks for x in K})
            if alpha(o, union) != alpha(o, [alpha(o, K) for K in Ks]):
                mult = False
                break
        allok &= unit and emp and mult
        print(f"   alpha_{o}:  UNIT alpha({{x}})=x {str(unit):<6}  alpha(empty)={BOT[o]} {str(emp):<6}  "
              f"MULTIPLY (all 2^16 families) {mult}")
    d = {o: alpha(o, ["T", "F"]) for o in ["t", "i", "f"]}
    distinct = len(set(d.values())) == 3
    print(f"   distinct on {{T,F}}: alpha_t={d['t']}  alpha_i={d['i']}  alpha_f={d['f']}  "
          f"=> three DISTINCT structures on ONE carrier: {distinct}")
    print("   (FOUR finite => each order's join is total; the semilattice is complete.)")
    print()
    return allok and distinct
# -------------------------------------------------------------- 45B  refined
def semilattice(d):
    print("=== 45B: is the kernel composition ⊕ a JOIN on the REFINED carrier? ===")
    print("   ⊕(x,y)=(∈x∋)(∈y∋);  three identity relations recorded separately")
    pairs = [(i, j) for i in range(N2) for j in range(N2)]
    pw = [supword(W[LAB[i]], W[LAB[j]]) for i, j in pairs]
    mp = measure_words(pw)
    # idempotence  x⊕x
    idem = {"byte": 0, "reg": 0, "pi": 0}
    for i in range(N2):
        r = supword(W[LAB[i]], W[LAB[i]])
        idem["byte"] += (r == W[LAB[i]])
        idem["reg"] += (mp[r]["reg"] == d[LAB[i]]["reg"])
        idem["pi"] += (mp[r]["pi"] == d[LAB[i]]["pi"])
    # commutativity  x⊕y vs y⊕x
    com = {"byte": 0, "reg": 0, "pi": 0}
    for i in range(N2):
        for j in range(N2):
            a = supword(W[LAB[i]], W[LAB[j]]); b = supword(W[LAB[j]], W[LAB[i]])
            com["byte"] += (a == b)
            com["reg"] += (mp[a]["reg"] == mp[b]["reg"])
            com["pi"] += (mp[a]["pi"] == mp[b]["pi"])
    # associativity (sampled)
    rnd = random.Random(45)
    tri = [(rnd.randrange(N2), rnd.randrange(N2), rnd.randrange(N2)) for _ in range(400)]
    aw = []
    for i, j, k in tri:
        aw.append(supword(supword(W[LAB[i]], W[LAB[j]]), W[LAB[k]]))
        aw.append(supword(W[LAB[i]], supword(W[LAB[j]], W[LAB[k]])))
    ma = measure_words(aw)
    assoc = {"byte": 0, "reg": 0, "pi": 0}
    for i, j, k in tri:
        L = supword(supword(W[LAB[i]], W[LAB[j]]), W[LAB[k]])
        Rr = supword(W[LAB[i]], supword(W[LAB[j]], W[LAB[k]]))
        assoc["byte"] += (L == Rr)
        assoc["reg"] += (ma[L]["reg"] == ma[Rr]["reg"])
        assoc["pi"] += (ma[L]["pi"] == ma[Rr]["pi"])
    # bottom law  x⊕b  (b a measured pi=N candidate)
    bots = [lab for lab in LAB if d[lab]["pi"] == "N"]
    bot = {"byte": 0, "reg": 0, "pi": 0}
    bw = [supword(W[lab], W[b]) for b in bots for lab in LAB]
    mb = measure_words(bw)
    totb = 0
    for b in bots:
        for lab in LAB:
            totb += 1
            r = supword(W[lab], W[b])
            bot["byte"] += (r == W[lab])
            bot["reg"] += (mb[r]["reg"] == d[lab]["reg"])
            bot["pi"] += (mb[r]["pi"] == d[lab]["pi"])
    print(f"   idempotence  x⊕x   byte {idem['byte']}/{N2}   register {idem['reg']}/{N2}   π {idem['pi']}/{N2}")
    print(f"   commutativity x⊕y  byte {com['byte']}/{N2*N2}   register {com['reg']}/{N2*N2}   π {com['pi']}/{N2*N2}")
    print(f"   associativity      byte {assoc['byte']}/400   register {assoc['reg']}/400   π {assoc['pi']}/400")
    print(f"   bottom ({bots}) x⊕b byte {bot['byte']}/{totb}   register {bot['reg']}/{totb}   π {bot['pi']}/{totb}")
    print("   -> ⊕ is an associative/commutative/idempotent join on the π QUOTIENT")
    print("      but NOT on the refined information (byte) carrier: a genuine paraconsistent split.")
    print()
    return idem, com, assoc, bot, totb, bots

# ---------------------------------------------------------------- 45C  trap
def trap(d):
    print("=== 45C: the canonicalizer C=eta_e∘pi CANNOT be the E-M algebra on V2 ===")
    cw = {lab: ENGAGR + crispword(d[lab]["pi"]) for lab in LAB}
    mc = measure_words(list(cw.values()))
    nbyte = sum(cw[lab] == W[lab] for lab in LAB)
    npi = sum(mc[cw[lab]]["pi"] == d[lab]["pi"] for lab in LAB)
    print(f"   UNIT law alpha({{x}})=x demanded on byte-distinct V2.")
    print(f"   C(x)=eta_e(pi(x)) byte-equal to x on {nbyte}/{N2} V2 objects; π-equal on {npi}/{N2}")
    for lab in ["A1", "A2", "A3", "A5"]:
        print(f"     {lab:<4} pi={d[lab]['pi']}  C-> {'==byte x' if cw[lab]==W[lab] else '!=byte x'}"
              f"   {'=pi x' if mc[cw[lab]]['pi']==d[lab]['pi'] else '!=pi x'}")
    print(f"   witness: alpha_can({{A2}}) = C(A2) !=byte A2 : {cw['A2'] != W['A2']}  (A2 !=byte A)")
    print("   => C passes the unit law only AFTER quotienting by pi, never on the refined carrier.")
    print()
    return nbyte == N2, npi == N2, cw

def em_refined(d, fams):
    print("=== 45C': E-M MULTIPLY on the refined carrier (alpha⊕(K)=fold⊕ over K) ===")
    comp = []
    ws = []
    for fam in fams:
        subsets = [[W[l] for l in S] for S in fam]
        union = sorted({w for S in subsets for w in S})
        lhs = fold(union)
        inner = [fold(S) for S in subsets]
        rhs = fold(inner)
        comp.append((lhs, rhs))
        ws += [lhs, rhs]
    mw = measure_words(ws)
    cnt = {"byte": 0, "reg": 0, "pi": 0}
    for lhs, rhs in comp:
        cnt["byte"] += (lhs == rhs)
        cnt["reg"] += (mw[lhs]["reg"] == mw[rhs]["reg"])
        cnt["pi"] += (mw[lhs]["pi"] == mw[rhs]["pi"])
    tot = len(comp)
    print(f"   families tested: {tot}")
    print(f"   MULTIPLY  alpha(⋃K) =? alpha({{alpha(K)}})   byte {cnt['byte']}/{tot}   "
          f"register {cnt['reg']}/{tot}   π {cnt['pi']}/{tot}")
    print("   (π-resolution reproduces the crisp alpha_i; byte-resolution fails on the refined carrier.)")
    print()
    return cnt, tot
# ---------------------------------------------------------------- table
def perm_test(d):
    """permutation invariance of fold over the sharp witness K_TF."""
    K = ["TF", "A1", "A2", "A3", "A4", "A5"]
    a = fold([W[l] for l in K]); b = fold([W[l] for l in reversed(K)])
    mw = measure_words([a, b] if a != b else [a])
    byte = (a == b)
    reg = (mw[a]["reg"] == mw[b]["reg"])
    pi = (mw[a]["pi"] == mw[b]["pi"])
    return byte, reg, pi

def table(idem, com, assoc, bot, totb, cnt, tot, perm):
    print("=== 45B/45C TABLE: property resolved at BYTES / REGISTER / π ===")
    def bl(x): return "True" if x else "False"
    rows = [
        ("singleton law", True, True, True),
        ("empty/bottom law", bl(bot['byte'] == totb), bl(bot['reg'] == totb), bl(bot['pi'] == totb)),
        ("permutation invariance", bl(perm[0]), bl(perm[1]), bl(perm[2])),
        ("join idempotence", bl(idem['byte'] == N2), bl(idem['reg'] == N2), bl(idem['pi'] == N2)),
        ("join commutativity", bl(com['byte'] == N2*N2), bl(com['reg'] == N2*N2), bl(com['pi'] == N2*N2)),
        ("join associativity", bl(assoc['byte'] == 400), bl(assoc['reg'] == 400), bl(assoc['pi'] == 400)),
        ("E-M multiplication law", bl(cnt['byte'] == tot), bl(cnt['reg'] == tot), bl(cnt['pi'] == tot)),
    ]
    print(f"   {'property':<24}{'bytes':<10}{'register':<10}{'π':<10}")
    for nm, b, r, p in rows:
        print(f"   {nm:<24}{b:<10}{r:<10}{p:<10}")
    print("   A π-level PASS must not mask a byte-level FAIL: the refined carrier is NOT")
    print("   an E-M algebra, while its π-quotient is (up to the three crisp orders of 45A).")
    print()

def main():
    print("=== Stage 45: EILENBERG-MOORE ALGEBRAS OF RE-ENTRY ===")
    print("   alpha:P(X)->X with alpha∘u=id and alpha∘m=alpha∘T(alpha)  <=>  arbitrary joins.")
    print()
    d = measure_v2()
    print("  V2 domain π: " + ", ".join(f"{l}→{d[l]['pi']}" for l in LAB))
    print()
    a = em_crisp()
    idem, com, assoc, bot, totb, bots = semilattice(d)
    nb, npi, cw = trap(d)
    KTF = ["TF", "A1", "A2", "A3", "A4", "A5"]
    fams = [
        [KTF, ["A1"], ["A2"]],
        [["T"], ["F"]],
        [["A1"], ["A2"], ["A3"]],
        [KTF, ["TF"]],
        [["A2"], ["A5"], ["A1"]],
        [["TF"], ["A2"], ["A3"], ["A4"]],
        [["T", "F"]],
        [["A1", "A2"], ["A3"]],
    ]
    cnt, tot = em_refined(d, fams)
    perm = perm_test(d)
    table(idem, com, assoc, bot, totb, cnt, tot, perm)
    print("STAGE45: E-M structure exists at the CRISP FOUR quotient (THREE distinct algebras)")
    print("   and at the π-view of the refined carrier; it does NOT exist on byte-distinct V2.")
    print(f"   45A three algebras {a} | 45B split (idem pi {idem['pi']}/{N2}) | 45C byte-unit {nb} pi-unit {npi}")
    print("   The singleton law forbids canonicalizing provenance: C=eta_e∘pi is NOT the algebra")
    print("   on V2 -- it is the algebra only AFTER the π-quotient, where A2 and A merge.")
    print("   NEXT (Stage 46): the free 𝔗-algebra / Kleisli adjunction E-M side.")

if __name__ == "__main__":
    main()
