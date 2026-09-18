#!/usr/bin/env python3
"""lift_55.py - Stage 55: THE SECTION SPACE AND THE COMMON QUOTIENT.

Stage 54 found four E-M sections per carrier.  Stage 55 shows WHY there are four:
every section splits the SAME evaluation quotient, choosing a different coherent
representative of each class.

    K ≡_α L  iff  alpha_A(K) = alpha_A(L)          (the evaluation kernel)
    Q_A := P(A) / ≡_α      ;      alpha_bar : Q_A -> A,  alpha_bar([K]) = alpha(K)

alpha is a union congruence because alpha is an E-M algebra map; the singleton law
makes alpha surjective, so alpha_bar : Q_A ≅EM A.

  55A the quotient Q_A: 16 subsets, classes, class sizes, union well-definedness,
      union congruence, alpha surjective, Q_A ≅EM A, the fiber signature
      Fib(A) = multiset { |alpha^-1(a)| : a in A }  (an E-M iso invariant).
  55B every E-M section gamma gives a selector sigma_gamma([K]) = gamma(alpha K):
      q∘sigma_gamma = id_Q  and  im sigma_gamma = I_gamma.
      I_gamma != I_delta as subsets  AND  I_gamma ≅EM Q_A ≅EM I_delta.
      All four projectors e_gamma = gamma∘alpha have EXACTLY the same fibers.
  55C the section space Sec(A) with gamma <= delta iff forall a, gamma(a) ⊆ delta(a):
      complete Hasse relation, pointwise-union section? unique LUB? pointwise-
      intersection section? unique GLB?  then the measured shape (chain / diamond /
      antichain / other).

The paraconsistent object held (NOT normalized):
    different sections / different EM-images   AND   the same quotient underneath.
"""
import sys, os, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_41 import member, canon_set
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_49 as L49
import lift_50 as L50
import lift_53 as L53
import lift_54 as L54
import lift_reentry as R

ORD3 = ["t", "i", "f"]
word = L43.word
allsubs = L50.allsubs
ALGS = L53.ALGS

# ------------------------------------------------------ 55A  the evaluation quotient
def quotient_block():
    print("=== 55A: EVALUATION KERNEL, THE QUOTIENT Q_A = P(A)/≡_α, AND Q_A ≅EM A ===")
    ok = True
    data = {}
    for (name, cA, afun, bot) in ALGS:
        E = list(cA)
        PA = allsubs(E)
        byval = {}
        for K in PA:
            byval.setdefault(afun(K), []).append(K)
        classes = list(byval.values())
        # union congruence:  K≡K', L≡L'  =>  K∪L ≡ K'∪L'   (=== union well-definedness)
        cong = True
        for K in PA:
            for Kp in byval[afun(K)]:
                for L in PA:
                    for Lp in byval[afun(L)]:
                        if afun(K | L) != afun(Kp | Lp):
                            cong = False
        surj = (set(afun(K) for K in PA) == set(E))
        # alpha_bar E-M hom: alpha(K∪L) = alpha_A({alpha(K),alpha(L)})  (multiply law)
        bar = all(afun(K | L) == afun({afun(K), afun(L)}) for K in PA for L in PA)
        fib = sorted(len(byval[a]) for a in E)          # fiber signature Fib(A)
        data[name] = dict(E=E, PA=PA, byval=byval, classes=classes, afun=afun, fib=fib)
        ok &= cong and surj and bar
        print("   " + name + ":  #classes " + str(len(classes)) +
              "  class sizes " + str(sorted(len(v) for v in classes)) +
              "  alpha surjective " + str(surj) + "  union-congruence " + str(cong) +
              "  alpha_bar hom " + str(bar))
        print("        Fib(A) = " + str(fib) + "   (fiber sizes |alpha^-1(a)|)")
    same_free = all(data[n]["fib"] == data["F(2)"]["fib"] for n in ("A_t", "A_i", "A_f"))
    print("   crisp three share F(2)'s fibers? " + str(same_free) +
          "  ;  A_chain Fib = " + str(data["A_chain"]["fib"]))
    print("   => Q_A = P(A)/≡_α with alpha_bar([K])=alpha(K) is a bijective E-M hom: Q_A ≅EM A.")
    print("   => Fib(A) is an E-M isomorphism invariant of the algebra.")
    print()
    return ok, data

# ----------------------------------------------------------------- shared helpers
def lub(i, k, le, n):
    ubs = [t for t in range(n) if le[i][t] and le[k][t]]
    for m in ubs:
        if all(le[m][t] for t in ubs):
            return m
    return None

def glb(i, k, le, n):
    lbs = [t for t in range(n) if le[t][i] and le[t][k]]
    for m in lbs:
        if all(le[t][m] for t in lbs):
            return m
    return None

def classify(le, n):
    total = all(le[i][k] or le[k][i] for i in range(n) for k in range(n))
    none = all(i == k or not (le[i][k] or le[k][i]) for i in range(n) for k in range(n))
    if total:
        return "chain"
    if none:
        return "antichain"
    mins = [i for i in range(n) if all(le[i][k] for k in range(n))]
    maxs = [i for i in range(n) if all(le[k][i] for k in range(n))]
    if len(mins) == 1 and len(maxs) == 1 and n == 4:
        return "diamond(B2)"
    return "other finite poset"

# ------------------------------- 55B  sections as representative selectors of one Q_A
def selector_block(data):
    print("=== 55B: SECTIONS ARE REPRESENTATIVE SELECTORS OF ONE QUOTIENT Q_A ===")
    print("   sigma_gamma([K]) = gamma(alpha K):  q∘sigma_gamma=id_Q,  im sigma_gamma = I_gamma")
    ok = True
    for (name, cA, afun, bot) in ALGS:
        d = data[name]
        E = d["E"]
        idx = {x: i for i, x in enumerate(E)}
        PA = d["PA"]
        secs, _, _ = L54.search_sections(cA, afun)
        ims = []
        qsid = True
        samefib = True
        for g in secs:
            gamma = (lambda a, g=g: g[idx[a]])
            # q∘sigma_gamma = id_Q :  alpha(gamma(alpha K)) = alpha(K)
            qsid &= all(afun(gamma(afun(K))) == afun(K) for K in PA)
            ims.append(frozenset({gamma(a) for a in E}))
            # SAME fibers for every projector e_gamma = gamma∘alpha:
            #   e_gamma(K)=e_gamma(L)  iff  alpha(K)=alpha(L)
            e = (lambda K, g=g: g[idx[afun(K)]])
            samefib &= all((e(K) == e(L)) == (afun(K) == afun(L)) for K in PA for L in PA)
        distinct = (len(set(ims)) == len(ims))
        ok &= qsid and samefib and distinct
        print("   " + name + ":  #sections " + str(len(secs)) + "  q∘sigma=id_Q " + str(qsid) +
              "  I_gamma pairwise distinct " + str(distinct) + "  all 4 projectors same fibers " + str(samefib))
    print("   => I_gamma != I_delta as subsets of P(A)  AND  I_gamma ≅EM Q_A ≅EM I_delta:")
    print("      four embedded realizations of ONE quotient, the representative changing,")
    print("      the equivalence relation being represented NOT changing.")
    print("   => three layers:  quotient class [K],  chosen representative gamma(alpha K),  image I_gamma.")
    print()
    return ok

# ------------------------------------------ 55C  the section space and its shape
def section_space_block(data):
    print("=== 55C: THE SECTION SPACE Sec(A) -- ORDER, POINTWISE JOIN/MEET, SHAPE ===")
    print("   gamma <= delta  iff  forall a, gamma(a) ⊆ delta(a)")
    ok = True
    for (name, cA, afun, bot) in ALGS:
        E = list(cA)
        secs, _, _ = L54.search_sections(cA, afun)
        n = len(secs)
        idxS = {g: i for i, g in enumerate(secs)}
        le = [[all(secs[i][j] <= secs[k][j] for j in range(len(E))) for k in range(n)] for i in range(n)]
        covers = []
        for i in range(n):
            for k in range(n):
                if i != k and le[i][k] and not any(i != m != k and le[i][m] and le[m][k] for m in range(n)):
                    covers.append((i, k))
        pairs = 0
        union_sec = union_lub = meet_sec = meet_glb = 0
        for i in range(n):
            for k in range(i + 1, n):
                pairs += 1
                w = tuple(secs[i][j] | secs[k][j] for j in range(len(E)))
                m = tuple(secs[i][j] & secs[k][j] for j in range(len(E)))
                if w in idxS:
                    union_sec += 1
                    if lub(i, k, le, n) == idxS[w]:
                        union_lub += 1
                if m in idxS:
                    meet_sec += 1
                    if glb(i, k, le, n) == idxS[m]:
                        meet_glb += 1
        join_semi = all(lub(i, k, le, n) is not None for i in range(n) for k in range(n))
        lattice = join_semi and all(glb(i, k, le, n) is not None for i in range(n) for k in range(n))
        shape = classify(le, n)
        ok &= (union_sec == pairs) and (union_lub == pairs) and join_semi
        print("   " + name + ":  |Sec| " + str(n) + "  Hasse-edge count " + str(len(covers)) +
              "  shape " + shape)
        print("        pairs " + str(pairs) + " :  pointwise-union section? " + str(union_sec) +
              "  unique LUB? " + str(union_lub) + "  pointwise-meet section? " + str(meet_sec) +
              "  unique GLB? " + str(meet_glb))
        print("        join-semilattice? " + str(join_semi) + "   lattice? " + str(lattice))
    print("   => pointwise union is always the unique LUB inside Sec(A); measure the meet, do not")
    print("      assume it.  The four sections are classified by their MEASURED order, not by count.")
    print()
    return ok

def main():
    print("=== Stage 55: THE SECTION SPACE AND THE COMMON QUOTIENT ===")
    a, data = quotient_block()
    b = selector_block(data)
    c = section_space_block(data)
    ok = a and b and c
    print("STAGE55: the evaluation kernel K≡_α L iff alpha(K)=alpha(L) is a union congruence;")
    print("   alpha is surjective, so Q_A=P(A)/≡_α ≅EM A.  Every E-M section gamma selects a")
    print("   representative per class (sigma_gamma([K])=gamma(alpha K)): the I_gamma are DIFFERENT")
    print("   subsets of P(A) yet ALL ≅EM the SAME quotient Q_A, and all projectors e_gamma have the")
    print("   same fibers.  Sec(A) is classified by its measured order, pointwise-union = unique LUB.")
    print("   55A quotient " + str(a) + " | 55B selectors " + str(b) + " | 55C section space " + str(c) +
          " => " + str(ok))
    print("   NEXT (Stage 56): the quotient Q_A over all re-entry levels / the section-space fibre.")

if __name__ == "__main__":
    main()
