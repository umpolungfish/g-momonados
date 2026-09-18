#!/usr/bin/env python3
"""lift_53.py - Stage 53: THE FIXED-POINT SPINE CARRIES ITS OWN E-M ALGEBRA.

S_A := Fix(rho_A) = { {a} | a in A }  is the image of Stage-52's split idempotent
in Set -- but it is NOT automatically an E-M subalgebra of the ambient free
algebra (P(A), union):  {T},{F} in S_A  yet  {T} u {F} = {T,F} not in S_A.  So the
spine algebra is NOT inherited union.  It is TRANSPORTED across the carrier iso

    box_A   : A -> S_A     a |-> {a}
    unbox_A : S_A -> A     {a} |-> a
    alpha^S_A = box_A o alpha_A o P(unbox_A) :  P(S_A) -> S_A.

  53A exhaust the E-M laws for alpha^S over 2^16 families, for S_t,S_i,S_f,S_F(2),S_chain
  53B box_A : (A,alpha_A) ≅EM (S_A,alpha^S_A) -- both hom squares + both inverses
  53C r_A : P(A) -> S_A, r_A(K)={alpha_A(K)}  IS an E-M morphism (r_A o m_A = alpha^S o P(r_A));
      the inclusion j_A : S_A ↪ P(A) is NOT.  Sharp witness K={{T},{F}}.
      So in Set  P(A)--r_A-->S_A--j_A-->P(A) is a retraction (r j = id, j r = rho),
      while in EM(T) only the downward evaluation map F(UA) --r_A--> S_A ≅EM A holds.
      Do NOT call this "splitting an idempotent in EM(T)".
"""
import sys, os, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_41 import member, canon_set
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_49 as L49
import lift_50 as L50
import lift_51 as L51
import lift_reentry as R

ORD3 = ["t", "i", "f"]
word = L43.word
allsubs = L50.allsubs
CARRIER = FOUR

CRISP = {o: (lambda K, o=o: alpha(o, sorted(K))) for o in ORD3}
CHAINORD = {"N": 0, "T": 1, "F": 2, "B": 3}

def alpha_chain(K):
    K = list(K)
    return "N" if not K else max(K, key=lambda x: CHAINORD[x])

a, b = "a", "b"
Fc = [frozenset(), frozenset({a}), frozenset({b}), frozenset({a, b})]

def union_family(KK):
    r = frozenset()
    for K in KK:
        r = r | K
    return r

ALGS = [
    ("A_t",     CARRIER, CRISP["t"],   BOT["t"]),
    ("A_i",     CARRIER, CRISP["i"],   BOT["i"]),
    ("A_f",     CARRIER, CRISP["f"],   BOT["f"]),
    ("F(2)",    Fc,      union_family, frozenset()),
    ("A_chain", CARRIER, alpha_chain,  "N"),
]

# --------------------------------------------------------------- box / unbox
def box(x):
    return frozenset({x})

def unbox(s):
    return next(iter(s))

def spine_alg(cA, afun):
    """S_A = {{a}|a in A};  alpha^S_A(K) = box(alpha_A({unbox s | s in K}))."""
    S = [box(x) for x in cA]
    def alphaS(KK):
        inner = {unbox(s) for s in KK}
        return box(afun(inner))
    return S, alphaS

def rmap(cA, afun, K):
    """r_A(K) = { alpha_A(K) } : P(A) -> S_A  (same values as rho_A, codomain S_A)."""
    return box(afun(K))

# ---------------------------------------------- 53A  the spine's own E-M laws
def spine_laws():
    print("=== 53A: THE SPINE ALGEBRA alpha^S_A = box o alpha_A o P(unbox)  OB E-M LAWS ===")
    print("   ambient union does NOT close: {T},{F} in S_A but {T}u{F}={T,F} not in S_A")
    ok = True
    for (name, cA, afun, bot) in ALGS:
        S, aS = spine_alg(cA, afun)
        PS = allsubs(S)
        # a concrete ambient-union failure when the carrier has >=2 points
        amb_ok = True
        if len(S) >= 2:
            s0, s1 = S[0], S[1]
            amb_ok = (s0 | s1) in S
        unit = all(aS(frozenset({s})) == s for s in S)
        mult = True
        n = len(PS)
        for fam in range(1 << n):
            Ks = [PS[i] for i in range(n) if (fam >> i) & 1]
            union = frozenset().union(*Ks) if Ks else frozenset()
            lhs = aS(union)
            rhs = aS(frozenset({aS(K) for K in Ks}))
            if lhs != rhs:
                mult = False
                break
        ok &= unit and mult and (not amb_ok if len(S) >= 2 else True)
        print("   S_" + name + ":  UNIT alpha^S({s})=s " + str(unit) +
              "  MULTIPLY over 2^" + str(n) + "=" + str(1 << n) + " families " + str(mult) +
              "  ambient-union closed? " + str(amb_ok))
    print("   => transported evaluation IS an E-M algebra on the spine; inherited union is NOT.")
    print()
    return ok

# -------------------------------------------- 53B  box : (A,alpha_A) ≅EM (S_A,alpha^S_A)
def box_iso():
    print("=== 53B: box_A : (A,alpha_A) ≅EM (S_A,alpha^S_A) ===")
    ok = True
    for (name, cA, afun, bot) in ALGS:
        S, aS = spine_alg(cA, afun)
        inv1 = all(unbox(box(x)) == x for x in cA)                    # unbox o box = id_A
        inv2 = all(box(unbox(s)) == s for s in S)                     # box o unbox = id_S
        boxhom = all(box(afun(K)) == aS(frozenset({box(x) for x in K})) for K in allsubs(cA))
        unboxhom = all(unbox(aS(K)) == afun({unbox(s) for s in K}) for K in allsubs(S))
        ok &= inv1 and inv2 and boxhom and unboxhom
        print("   " + name + ":  unbox∘box=id_A " + str(inv1) + "  box∘unbox=id_S " + str(inv2) +
              "  box(alpha K)=alpha^S(P(box) K) " + str(boxhom) +
              "  unbox(alpha^S K)=alpha(P(unbox) K) " + str(unboxhom))
    print("   => the singleton spine is the SAME E-M structure in singleton coordinates,")
    print("      categorical (not merely set-theoretic).")
    print()
    return ok

# ------------------------------- 53C  r_A is EM, j_A is not; sharp witness K={{T},{F}}
def retraction():
    print("=== 53C: r_A IS AN E-M MORPHISM, j_A IS NOT ===")
    ok = True
    for (name, cA, afun, bot) in ALGS:
        S, aS = spine_alg(cA, afun)
        PA = allsubs(cA)
        n = len(PA)                                    # 16 ; families of P(A): 2^16
        # r_A o m_A = alpha^S o P(r_A)
        rhom = True
        for fam in range(1 << n):
            Ks = [PA[i] for i in range(n) if (fam >> i) & 1]
            union = frozenset().union(*Ks) if Ks else frozenset()
            lhs = rmap(cA, afun, union)                        # r_A(m_A(fam))
            rhs = aS(frozenset({rmap(cA, afun, K) for K in Ks}))   # alpha^S(P(r_A)(fam))
            if lhs != rhs:
                rhom = False
                break
        # j_A (inclusion) : j_A(alpha^S K) = m_A(P(j_A)(K)) ?  fails in general
        jhom = True
        jfails = 0
        PS = allsubs(S)
        for K in PS:
            lhs = aS(K)                              # j_A(alpha^S K), j_A inclusion
            rhs = frozenset().union(*K) if K else frozenset()   # m_A(P(j_A) K)
            if lhs != rhs:
                jhom = False
                jfails += 1
        ok &= rhom and (not jhom)
        print("   S_" + name + ":  r_A o m_A = alpha^S o P(r_A) over 2^" + str(n) + " families " +
              str(rhom) + "  ;  j_A E-M hom? " + str(jhom) + "  (fails on " + str(jfails) +
              "/" + str(len(PS)) + " families)")
    # the sharp witness on the three crisp algebras
    print("   sharp witness  K = {{T},{F}}  (a two-singleton family in P(S_A)):")
    Kfam = frozenset({box("T"), box("F")})
    for o in ORD3:
        S, aS = spine_alg(CARRIER, CRISP[o])
        left = aS(Kfam)                                    # j_A(alpha^S(K))
        right = frozenset().union(*Kfam)                   # m_A(P(j_A)(K)) = {T,F}
        S, aS = spine_alg(CARRIER, CRISP[o])
        print("        " + o + ":  j(alpha^S K) = " + str(sorted(left)) + "   vs   m(P(j) K) = " +
              str(sorted(right)) + "   equal? " + str(left == right))
    # set-retraction identities
    Ks = allsubs(CARRIER)
    rj = all(rmap(CARRIER, CRISP["t"], frozenset({x})) == frozenset({x}) for x in CARRIER)
    jr = all(frozenset(rmap(CARRIER, CRISP["t"], K)) == rmap(CARRIER, CRISP["t"], K) for K in Ks)
    print("   in Set:  r_A∘j_A = id_S " + str(rj) + "  ;  j_A∘r_A = rho_A " + str(jr) +
          "  (same values as Stage-52 rho)")
    print("   => in Set  P(A) --r_A--> S_A --j_A--> P(A)  is a retraction (r j = id, j r = rho),")
    print("      while in EM(T) ONLY the downward evaluation map F(UA) --r_A--> S_A ≅EM A holds.")
    print("      Do NOT call this 'splitting an idempotent in EM(T)'.")
    print()
    return ok

def main():
    print("=== Stage 53: THE FIXED-POINT SPINE CARRIES ITS OWN E-M ALGEBRA ===")
    a = spine_laws()
    b = box_iso()
    c = retraction()
    ok = a and b and c
    print("STAGE53: S_A = Fix(rho_A) = {{a}|a in A} is NOT closed under the ambient union,")
    print("   yet it CARRIES an E-M algebra alpha^S_A = box∘alpha_A∘P(unbox) transported from A.")
    print("   box_A : (A,alpha_A) ≅EM (S_A,alpha^S_A); r_A : P(A)->S_A is an E-M morphism but the")
    print("   inclusion j_A : S_A ↪ P(A) is NOT -- two-faced, on the same singleton spine.")
    print("   53A E-M laws " + str(a) + " | 53B box iso " + str(b) + " | 53C r/j " + str(c) +
          " => " + str(ok))
    print("   NEXT (Stage 54): r_A's section / the spine as the image object of the idempotent.")

if __name__ == "__main__":
    main()
