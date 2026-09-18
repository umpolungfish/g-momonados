#!/usr/bin/env python3
"""lift_54.py - Stage 54: TWO SECTIONS, TWO IDEMPOTENTS, ONE ALGEBRA.

Stage 53 left an asymmetry:  u_A : A -> P(A)  is a Set-section of alpha_A
(alpha u = id) but is NOT an E-M morphism; j_A : S_A ↪ P(A) is not either.

Stage 54 asks for a SECOND section

    gamma_A : A -> P(A)      alpha_A o gamma_A = id_A   AND  gamma_A E-M morphism

where the target is the free algebra (P(A), union):

    gamma(alpha(K)) = UNION{ gamma(a) | a in K }     (all joins, incl. empty: gamma(bot)=empty)

For a 4-element carrier the search is exhaustive: |P(A)|^|A| = 16^4 = 65,536 maps.

  54A search independently for A_t,A_i,A_f,F(2),A_chain -- COUNT the E-M sections
  54B if a section gamma exists: e_A = gamma o alpha_A, the equation block
      e^2=e, alpha o e=alpha, e o gamma=gamma, alpha o gamma=id, e(UNION K)=UNION{e(K)};
      images  S_A=im(rho_A)={{a}} (Set-image)  vs  I_A=im(e_A)={gamma(a)} (EM-image);
      S_A NOT ambient-union-closed  AND  I_A IS  yet  S_A ≅EM A ≅EM I_A
  54C the bridge: r_A:P(A)->S_A, r_A(K)={alpha(K)}; its E-M section s_A({a})=gamma(a);
      r_A o s_A = id_S ; s_A IS an E-M morphism while j_A is NOT ;
      j_A o r_A = rho_A  and  s_A o r_A = e_A -- same downward map, two different returns.

The paraconsistent object held (NOT normalized):
    u_A is a Set-section of alpha_A AND (generally) NOT an E-M morphism,
    while gamma_A is a second, simultaneous E-M section of the same alpha_A.
"""
import sys, os, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_41 import member, canon_set
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_49 as L49
import lift_50 as L50
import lift_51 as L51
import lift_53 as L53
import lift_reentry as R

ORD3 = ["t", "i", "f"]
word = L43.word
allsubs = L50.allsubs
ALGS = L53.ALGS
box = L53.box
unbox = L53.unbox

def union_all(Ks):
    r = frozenset()
    for K in Ks:
        r = r | K
    return r

def union_closed(S):
    S = list(S)
    for fam in range(1 << len(S)):
        sub = [S[i] for i in range(len(S)) if (fam >> i) & 1]
        if union_all(sub) not in S:
            return False
    return True

# ------------------------------------------------ 54A  exhaustive section search
def search_sections(cA, afun):
    """every gamma : A -> P(A) with alpha(gamma a)=a and gamma(alpha K)=UNION gamma(K)."""
    E = list(cA)
    n = len(E)
    idx = {x: i for i, x in enumerate(E)}
    PA = allsubs(E)
    subsK = allsubs(E)
    secs = []
    for g in itertools.product(PA, repeat=n):
        if any(afun(g[i]) != E[i] for i in range(n)):
            continue
        ok = True
        for K in subsK:
            lhs = g[idx[afun(K)]]
            rhs = union_all([g[idx[x]] for x in K]) if K else frozenset()
            if lhs != rhs:
                ok = False
                break
        if ok:
            secs.append(g)
    return secs, idx, E

# ------------------------------------------- 54A  exhaustive E-M section search
def sections_block():
    print("=== 54A: EXHAUSTIVE E-M SECTION SEARCH  gamma:A->P(A), alpha∘gamma=id, gamma E-M ===")
    print("   search space |P(A)|^|A| = 16^4 = 65,536 maps per carrier; target = free (P(A),union)")
    data = {}
    ok = True
    counts = {}
    for (name, cA, afun, bot) in ALGS:
        secs, idx, E = search_sections(cA, afun)
        S = sorted({box(x) for x in E})
        Sclosed = union_closed(S)
        if secs:
            g = secs[0]
            I = sorted({g[idx[x]] for x in E})
            Iclosed = union_closed(I)
        else:
            I, Iclosed = [], None
        data[name] = (secs, idx, E, I)
        counts[name] = len(secs)
        ok &= (not Sclosed)
        if secs:
            ok &= (Iclosed is True)
        print("   " + name + ":  #E-M sections = " + str(len(secs)) +
              "   S_A union-closed? " + str(Sclosed) +
              "   I_A union-closed? " + str(Iclosed))
    same_crisp = (counts["A_t"] == counts["A_i"] == counts["A_f"] == counts["F(2)"])
    print("   crisp three + F(2) equal counts? " + str(same_crisp) +
          "   (counts t,i,f,F(2),chain = " +
          str([counts["A_t"], counts["A_i"], counts["A_f"], counts["F(2)"], counts["A_chain"]]) + ")")
    print("   => S_A = im(rho_A) is NOT ambient-union-closed, yet I_A = im(e_A) IS; both present")
    print("      the same abstract E-M algebra. (Equal crisp counts are expected: Stage-51 isos.)")
    print()
    return ok and same_crisp, data

# ------------------------------------- 54B  e_A = gamma∘alpha : equations + images
def idempotents_block(data):
    print("=== 54B: e_A = gamma ∘ alpha_A -- THE EQUATION BLOCK AND THE TWO IMAGES ===")
    ok = True
    for (name, cA, afun, bot) in ALGS:
        secs, idx, E, I = data[name]
        if not secs:
            print("   " + name + ": no E-M section (skip)")
            continue
        g = secs[0]

        def gamma(x, g=g, idx=idx):
            return g[idx[x]]

        def e(K, g=g, idx=idx, afun=afun):
            return g[idx[afun(K)]]

        PA = allsubs(E)
        Ks = allsubs(E)
        e2 = all(e(e(K)) == e(K) for K in Ks)
        ae = all(afun(e(K)) == afun(K) for K in Ks)
        eg = all(e(gamma(x)) == gamma(x) for x in E)
        ag = all(afun(gamma(x)) == x for x in E)
        n = len(PA)
        ehom = True
        for fam in range(1 << n):
            Kf = [PA[i] for i in range(n) if (fam >> i) & 1]
            lhs = e(union_all(Kf) if Kf else frozenset())
            rhs = union_all([e(K) for K in Kf])
            if lhs != rhs:
                ehom = False
                break
        S = sorted({box(x) for x in E})
        I = sorted({gamma(x) for x in E})
        Sclosed = union_closed(S)
        Iclosed = union_closed(I)
        ginj = (len(I) == len(E))
        ok &= e2 and ae and eg and ag and ehom and (not Sclosed) and Iclosed and ginj
        print("   " + name + ":  e^2=e " + str(e2) + "  alpha∘e=alpha " + str(ae) +
              "  e∘gamma=gamma " + str(eg) + "  alpha∘gamma=id " + str(ag) +
              "  e(UNION K)=UNION{e(K)} (2^" + str(n) + ") " + str(ehom))
        print("        S_A union-closed? " + str(Sclosed) + "   I_A=im(e)=im(gamma) union-closed? " +
              str(Iclosed) + "   gamma injective " + str(ginj))
    print("   => rho_A=u∘alpha (idempotent in Set; generally NOT E-M) and e_A=gamma∘alpha")
    print("      (idempotent in EM(T), hence also in Set) act on the SAME free cover.")
    print("   => S_A ≅EM A ≅EM I_A (S_A via box, I_A via gamma), though S_A != I_A as subsets.")
    print()
    return ok

# ------------------------------- 54C  the bridge: r_A, its E-M section s_A, two returns
def bridge_block(data):
    print("=== 54C: THE BRIDGE  r_A, its E-M section s_A, and the two returns ===")
    print("   r_A(K)={alpha(K)}:P(A)->S_A ;  s_A({a})=gamma(a):S_A->P(A)")
    ok = True
    for (name, cA, afun, bot) in ALGS:
        secs, idx, E, I = data[name]
        if not secs:
            print("   " + name + ": no E-M section (skip)")
            continue
        g = secs[0]

        def gamma(x, g=g, idx=idx):
            return g[idx[x]]

        def e(K, g=g, idx=idx, afun=afun):
            return g[idx[afun(K)]]

        def r(K, afun=afun):
            return box(afun(K))

        def s(ss, gamma=gamma):
            return gamma(unbox(ss))

        S, alphaS = L53.spine_alg(cA, afun)
        rs = all(r(s(ss)) == ss for ss in S)                 # r_A ∘ s_A = id_S
        PS = allsubs(S)
        # source of s_A is S_A, so E-M families range over P(S_A) (16), not 2^16
        shom = all(s(alphaS(K)) == union_all([s(ss) for ss in K]) for K in PS)
        jfails = 0
        for K in PS:
            lhs = alphaS(K)
            rhs = union_all(K) if K else frozenset()
            if lhs != rhs:
                jfails += 1
        jhom = (jfails == 0)
        Ks = allsubs(E)
        jr = all(frozenset(r(K)) == frozenset({afun(K)}) for K in Ks)     # j_A r_A = rho_A
        sr = all(s(r(K)) == e(K) for K in Ks)                            # s_A r_A = e_A
        ok &= rs and shom and (not jhom) and jr and sr
        print("   " + name + ":  r∘s=id_S " + str(rs) + "  s_A E-M " + str(shom) +
              "  j_A E-M? " + str(jhom) + " (fails " + str(jfails) + "/" + str(len(PS)) +
              ")  j∘r=rho " + str(jr) + "  s∘r=e " + str(sr))
    print("   => same downward map r_A; j_A is a Set-section only, s_A is an E-M section;")
    print("      j_A r_A = rho_A  and  s_A r_A = e_A  -- two different returns, held, not merged.")
    print()
    return ok

def main():
    print("=== Stage 54: TWO SECTIONS, TWO IDEMPOTENTS, ONE ALGEBRA ===")
    a, data = sections_block()
    b = idempotents_block(data)
    c = bridge_block(data)
    ok = a and b and c
    print("STAGE54: alpha_A has TWO sections -- u_A (Set only; generally not E-M) and an E-M")
    print("   gamma_A.  e_A=gamma∘alpha and rho_A=u∘alpha are two idempotents on the same free")
    print("   cover; S_A=im(rho_A) is not union-closed but I_A=im(e_A) is, yet S_A ≅EM A ≅EM I_A;")
    print("   and r_A has both a Set-section j_A and an E-M section s_A.  Nothing is normalized.")
    print("   54A counts " + str(a) + " | 54B idempotents " + str(b) + " | 54C bridge " + str(c) +
          " => " + str(ok))
    print("   NEXT (Stage 55): the lattice of sections / the image object as a quotient.")

if __name__ == "__main__":
    main()
