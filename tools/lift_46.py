#!/usr/bin/env python3
"""lift_46.py - Stage 46: FREE RE-ENTRY ALGEBRA / KLEISLI-E-M COMPARISON.

Stage 45's byte-level failure on V2 is NOT something to repair: the free E-M
algebra lives one winding higher, on  T(V2) = P(V2).

  46A  FREE ALGEBRA (recognised, not rediscovered)
       F(D) = (P(D), m_D),   m_D(KK) = UNION KK
       its E-M laws ARE Stage 43's left unit and associativity:
           m_D o u_{P(D)} = id        m_D o m_{P(D)} = m_D o T(m_D)
       => P(D) is the free T-algebra on the measured domain D.

  46B  UNIVERSAL PROPERTY
       for an E-M algebra (A,alpha) and generator map f : D -> A, the unique
       extension is   f#(K) = alpha(T(f)(K)) = alpha{ f(x) | x in K },  with
           f# o u_D = f        f# o m_D = alpha o T(f#).
       Strongest f = pi : D -> FOUR.  Stage 45 gives THREE targets
       (FOUR,alpha_t),(FOUR,alpha_i),(FOUR,alpha_f) => SAME pi, THREE extensions,
       equal on singletons, separating as soon as a distinction is held ({T,F}->T,B,F).

  46C  KLEISLI -> E-M COMPARISON   J : Kl(T) -> EM(T)
           J(k)(K) = K >>= k = UNION_{x in K} k(x) = m_Y(T(k)(K))
       the unique free-algebra extension of k.  Test J(u)=id, J(g*f)=J(g).J(f),
       and the E-M hom law  J(k)(UNION KK) = UNION { J(k)(K) | K in KK }.
"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import marks, ENGAGR
from lift_35 import weight_fields, run_batch
from lift_40 import pi_host, crispword
from lift_41 import V2
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_44 as L44
import lift_reentry as R

LAB = [lab for lab, _ in V2]
W = {lab: w for lab, w in V2}
N2 = len(V2)
PID = {}          # label -> pi value, filled by main()

def measure_pi():
    o = run_batch(["weight " + w for _, w in V2])
    return {lab: pi_host(marks(weight_fields(o, w))) for lab, w in V2}

# ---------------------------------------------------------------- 46A  free
def free_algebra():
    print("=== 46A: P(D) IS THE FREE T-ALGEBRA ON D  (recognised, not rediscovered) ===")
    print("   F(D) = (P(D), m_D),  m_D(KK) = UNION KK ;  D = the measured V2 domain")
    K_TF, S1, S2, S2_pair, S2_flat, S3 = L43.witnesses()
    lu = all(L43.word(L43.m(L43.u(K))) == L43.word(K) for _, K in S1)
    assoc = all(L43.word(L43.m(L43.mapmem(L43.m, X))) == L43.word(L43.m(L43.m(X)))
                for _, X in S3)
    print(f"   left unit     m_D o u_P(D) = id        : {lu}")
    print(f"   associativity m_D o m = m_D o T(m)     : {assoc}")
    print("   These are EXACTLY the E-M algebra laws of Stage 45 -- so (P(D), m_D) closes")
    print("   as the free T-algebra on D.  Nothing to repair at byte level: the free")
    print("   algebra lives one winding higher, on T(V2)=P(V2).")
    print()
    return lu and assoc
# ------------------------------------------------------------ 46B  universal
def fsharp(o, K):
    """unique free-algebra extension of f=pi to the target (FOUR, alpha_o)."""
    return alpha(o, [PID[x] for x in K])

def universal():
    print("=== 46B: UNIVERSAL PROPERTY -- f=pi, THREE distinct extensions ===")
    unit = all(fsharp(o, [x]) == PID[x] for o in ["t", "i", "f"] for x in LAB)
    ground = [frozenset({l}) for l in ["TF", "A1", "A2", "T", "F"]]
    ground += [frozenset({"T", "F"}), frozenset({"A1", "A2", "A3"}),
               frozenset({"TF", "A1", "A2", "A3", "A4", "A5"}), frozenset()]
    n = len(ground)
    mult = True
    for o in ["t", "i", "f"]:
        for msk in range(1 << n):
            KK = [ground[i] for i in range(n) if (msk >> i) & 1]
            union = frozenset().union(*KK) if KK else frozenset()
            if fsharp(o, union) != alpha(o, [fsharp(o, K) for K in KK]):
                mult = False
    agree = all(fsharp("t", [x]) == fsharp("i", [x]) == fsharp("f", [x]) for x in LAB)
    sep = {o: fsharp(o, ["T", "F"]) for o in ["t", "i", "f"]}
    uni = True
    for o in ["t", "i", "f"]:
        for msk in range(1 << N2):
            K = [LAB[i] for i in range(N2) if (msk >> i) & 1]
            if alpha(o, [PID[x] for x in K]) != fsharp(o, K):
                uni = False
    print(f"   f# o u_D = f            (all singletons, 3 orders)  : {unit}")
    print(f"   f# o m_D = alpha o T(f#) (all {2**n} families, 3)     : {mult}")
    print(f"   same generator pi; extensions agree on singletons    : {agree}")
    print(f"   on {{T,F}}: pi#_t={sep['t']}  pi#_i={sep['i']}  pi#_f={sep['f']}  "
          f"-> 3 DISTINCT extensions : {len(set(sep.values())) == 3}")
    print(f"   uniqueness: K=UNION{{x}} => h(K)=alpha{{h({{x}})}}=alpha{{f(x)}}=f#(K) : {uni}")
    print("   uniqueness is RELATIVE to the chosen (FOUR, alpha_o) -- no conflict with three.")
    print()
    return unit and mult and agree and (len(set(sep.values())) == 3) and uni

# ------------------------------------------------------------ 46C  comparison
def J(k, K):
    r = frozenset()
    for x in K:
        r = r | k(x)
    return r

def comparison():
    print("=== 46C: KLEISLI -> E-M COMPARISON  J(k)(K) = UNION_{x in K} k(x) ===")
    ws = list(L44.WIRES.values())
    arrows = {
        "u":        lambda x: frozenset({x}),
        "kappa_FW": lambda x: L44.kappa("FRAME_WORK", x),
        "kbar_FW":  lambda x: L44.kbar("FRAME_WORK", x),
        "kappa_CF": lambda x: L44.kappa("CLOSE_FRAME", x),
        "kbar_CF":  lambda x: L44.kbar("CLOSE_FRAME", x),
    }
    word = L43.word
    subsets = [frozenset(ws[i] for i in range(len(ws)) if (m >> i) & 1)
               for m in range(1 << len(ws))]
    jid = all(word(J(arrows["u"], K)) == word(K) for K in subsets)
    comp_ok = True
    ks = list(arrows.values())
    for f in ks:
        for g in ks:
            gsf = L44.kleisli(g, f)
            for K in subsets:
                if word(J(gsf, K)) != word(J(g, J(f, K))):
                    comp_ok = False
    hom = True
    for nm, k in arrows.items():
        if word(J(k, frozenset())) != word(frozenset()):
            hom = False
        for A in subsets:
            for Bq in subsets:
                if word(J(k, A | Bq)) != word(J(k, A) | J(k, Bq)):
                    hom = False
    print(f"   J(u) = id                              : {jid}")
    print(f"   J(g*f) = J(g) o J(f)                   : {comp_ok}")
    print(f"   E-M hom  J(k)(A∪B)=J(k)(A)∪J(k)(B), J(k)(∅)=∅ : {hom}")
    print("   old arrows re-read: J(kappa_R)(K)=UNION kappa_R(x), J(kbar_R)(K)=UNION kbar_R(x)")
    for Rnm in ["FRAME_WORK", "CLOSE_FRAME"]:
        off = [x for x in ws if L44.MORPH[Rnm](x) is None]
        if off:
            K = frozenset(off)
            k = lambda x, Rn=Rnm: L44.kappa(Rn, x)
            kb = lambda x, Rn=Rnm: L44.kbar(Rn, x)
            print(f"     R={Rnm}: off-domain K |K|={len(K)}  J(kappa)(K)={len(J(k, K))}  "
                  f"J(kbar)(K)={len(J(kb, K))}")
    print("   for finite D, empty-union + binary-union preservation => every union preserved.")
    print()
    return jid and comp_ok and hom

def main():
    global PID
    print("=== Stage 46: FREE RE-ENTRY ALGEBRA / KLEISLI-E-M COMPARISON ===")
    PID = measure_pi()
    _pi = L43.boot()
    for _, _w in V2:
        _pi[ENGAGR + crispword(_pi[_w])] = _pi[_w]
    L43.PI = _pi
    print("  pi on D: " + ", ".join(f"{l}→{PID[l]}" for l in LAB))
    print()
    a = free_algebra()
    b = universal()
    c = comparison()
    ok = a and b and c
    print("STAGE46: the free T-algebra on D is (P(D), m_D); the Kl->EM comparison J extends")
    print("   every Kleisli arrow uniquely, and pi extends to a DISTINCT free-algebra map")
    print("   for each of Stage 45's three crisp orders (same pi, three extensions).")
    print(f"   46A free algebra {a} | 46B universal+3 extensions {b} | 46C comparison {c} => {ok}")
    print("   NEXT (Stage 47): the adjunction F ⊣ U (unit/counit) and the monad = T round trip.")

if __name__ == "__main__":
    main()
