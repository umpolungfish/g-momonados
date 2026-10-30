#!/usr/bin/env python3
"""lift_56.py - Stage 56: THE SECTION FIBRE STABILIZES IN THE FREE SECTOR.

Stage 55 measured two invariants on the same 4-element carriers:
    fibre signature  Fib(A) = multiset{ |alpha^-1(a)| }
        A_t, A_i, A_f, F(2) : [2,2,2,10]        A_chain : [2,2,4,8]
    section-space shape   diamond for t,i,f,F(2) ; chain for A_chain.

Stage 56 shows both are shadows of ONE law, the free-sector law.  For the free
algebra F(X) = (P(X), union):

    Sec(F(X))  ≅  P(X)             (Boolean lattice, one binary choice per generator)
    Fib(F(X))  =  powerset-union fibre law  f(k) = Sum_j (-1)^j C(k,j) 2^(2^(k-j))

so the free sector's section geometry AND fibre geometry are BOTH determined by
the carrier alone, and G = F∘U sends every 4-element carrier to the SAME F(FOUR).

  56A the free-algebra section law: construct the 16 sections gamma_E (E subset FOUR),
      verify alpha∘gamma_E = id, gamma_E union-preserving (E-M hom), all 16 distinct,
      gamma_E <= gamma_F  iff  E subset F; gamma_(E∪F) is the unique LUB and COINCIDES
      with pointwise union; gamma_(E∩F) is the unique lattice GLB (it does NOT in
      general coincide with pointwise intersection -- a finding); complement.
      Plus the generator argument: each singleton has exactly 2 admissible images,
      so no other sections exist.
  56B the free powerset-union fibre law: compute |{K subset P(X) : union K = K0}|
      directly, match f(k); reproduce [2,2,2,10] for |X|=2 (= the Stage-55 free
      signature A_t/A_i/A_f/F(2)) and show A_chain's [2,2,4,8] is NOT of that form.
  56C the G-iteration A(0)=A, A(n+1)=G(A(n))=F(U A(n)): carrier 4 -> 16 -> 65536;
      every 4-element carrier gives G(A) ≅ F(FOUR) (incl. A_chain); |Sec(G(A))| =
      2^|U(A)|; Sec(F(X)) ≅ U(F(X)).  Structural stabilization: section space follows
      the carrier; the pre-re-entry geometry is forgotten by G.

The paraconsistent object held (NOT normalized):
    A_chain has different quotient-fibre geometry  AND  G(A_chain) forgets it,
    sending it to the same free algebra F(FOUR) as every other algebra on FOUR.
"""
import sys, os, math
from collections import Counter
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_45 import alpha, FOUR, BOT
import lift_50 as L50
import lift_53 as L53
import lift_54 as L54

allsubs = L50.allsubs
union_all = L54.union_all
ALGS = L53.ALGS


def fam_union(fam):
    """union of the members of a family (iterable of frozensets)."""
    out = frozenset()
    for s in fam:
        out = out | s
    return out


def free_section(X, E):
    """the union-preserving section gamma_E on F(X): gamma_E({x}) = {{x}} if x not
    in E else {empty,{x}}, extended by union.  Returns {K: gamma_E(K)}."""
    Xl = list(X)
    E = frozenset(E)
    tab = {}
    for K in allsubs(Xl):
        fam = set()
        for x in K:
            fam.add(frozenset([x]))
            if x in E:
                fam.add(frozenset())
        tab[K] = frozenset(fam)
    return tab

def hasse_edges(subs):
    """number of covering pairs E < F (no intermediate) among the subset-list subs."""
    edges = 0
    for E in subs:
        for F in subs:
            if E < F:
                mid = False
                for G in subs:
                    if E < G < F:
                        mid = True
                        break
                if not mid:
                    edges += 1
    return edges


def free_section_law(X):
    """verify Sec(F(X)) ≅ P(X) for F(X) = (P(X), union)."""
    print("--- free algebra F(X),  |X| = " + str(len(X)) + " ---")
    Xl = list(X)
    subs = allsubs(Xl)                 # 2^|X| subsets E
    PX = allsubs(Xl)                   # P(X)
    PPX = allsubs(PX)                  # families = 2^(2^|X|)
    secs = {}
    for E in subs:
        secs[E] = free_section(Xl, E)
    print("  #sections constructed : " + str(len(secs)) + "   (predicted 2^|X| = " + str(2 ** len(Xl)) + ")")
    ok = True

    def le(tE, tF):
        return all(tE[K] <= tF[K] for K in PX)

    idok = True
    for E in subs:
        for K in PX:
            if fam_union(secs[E][K]) != K:
                idok = False
    print("  alpha∘gamma_E = id  for all K : " + str(idok))
    ok = ok and idok

    homok = True
    for E in subs:
        tab = secs[E]
        for fam in PPX:
            lhs = tab[fam_union(fam)]
            rhs = fam_union(frozenset(tab[K] for K in fam))
            if lhs != rhs:
                homok = False
                break
        if not homok:
            break
    print("  gamma_E union-preserving (E-M hom on all families) : " + str(homok))
    ok = ok and homok

    dist = True
    for E in subs:
        for F in subs:
            if E != F:
                sd = E ^ F
                x = next(iter(sd))
                if secs[E][frozenset([x])] == secs[F][frozenset([x])]:
                    dist = False
    print("  all " + str(len(subs)) + " gamma_E pairwise distinct : " + str(dist))
    ok = ok and dist

    leok = join_union = lubok = glbok = compok = True
    meet_pointwise = 0
    meet_total = 0
    Xset = frozenset(Xl)
    for E in subs:
        for F in subs:
            if le(secs[E], secs[F]) != (E <= F):
                leok = False
            U = E | F
            I = E & F
            ubs = [G for G in subs if le(secs[E], secs[G]) and le(secs[F], secs[G])]
            if not (U in ubs and all(le(secs[U], secs[G]) for G in ubs)):
                lubok = False
            lbs = [G for G in subs if le(secs[G], secs[E]) and le(secs[G], secs[F])]
            if not (I in lbs and all(le(secs[G], secs[I]) for G in lbs)):
                glbok = False
            for K in PX:
                if (secs[E][K] | secs[F][K]) != secs[U][K]:
                    join_union = False
                meet_total += 1
                if (secs[E][K] & secs[F][K]) == secs[I][K]:
                    meet_pointwise += 1
    for E in subs:
        cE = Xset - E
        for K in PX:
            if (secs[E][K] | secs[cE][K]) != secs[Xset][K]:
                compok = False
    print("  gamma_E <= gamma_F  iff  E subset F : " + str(leok))
    print("  gamma_(E∪F) is the unique LUB : " + str(lubok) + "   (and = pointwise union : " + str(join_union) + ")")
    print("  gamma_(E∩F) is the unique GLB : " + str(glbok) + "   (pointwise ∩ agrees on " + str(meet_pointwise) + "/" + str(meet_total) + " cells)")
    print("  complement gamma_E v gamma_(X\\E) = gamma_X : " + str(compok))
    ok = ok and leok and join_union and lubok and glbok and compok

    atoms = len(Xl)
    n = len(subs)
    edges = hasse_edges(subs)
    tag = "diamond(B2)" if atoms == 2 else ("Boolean_" + str(atoms))
    print("  section-space shape : " + str(n) + " elements, " + str(edges) + " Hasse edges -> " + tag)
    print("  Sec(F(X)) ≅ P(X) (Boolean lattice) : " + str(ok))
    return ok


def generator_argument(X):
    """no other sections exist: each singleton has exactly 2 admissible images."""
    print("  generator argument (uniqueness):")
    counts = []
    for x in X:
        base = [frozenset(), frozenset([x])]
        good = 0
        for S in allsubs(base):
            if fam_union(S) == frozenset([x]):
                good += 1
        counts.append(good)
    prod = 1
    for c in counts:
        prod *= c
    print("    admissible gamma({x}) counts per generator : " + str(counts))
    print("    product = " + str(prod) + "   = 2^|X| = " + str(2 ** len(X)))
    return prod == 2 ** len(X)


def block_56A():
    print("=== 56A: FREE-ALGEBRA SECTION LAW   Sec(F(X)) ≅ P(X)  (Boolean) ===")
    ok2 = free_section_law(["N", "T"])
    print("")
    ok4 = free_section_law(list(FOUR))
    print("")
    g = generator_argument(list(FOUR))
    print("  56A : " + str(ok2 and ok4 and g))
    return ok2 and ok4 and g

def free_powerset_union_fibre(k):
    """f(k) = number of families K subset P(X) with union = K0, |K0| = k."""
    s = 0
    for j in range(k + 1):
        s += ((-1) ** j) * math.comb(k, j) * (2 ** (2 ** (k - j)))
    return s


def free_fibre_law(X):
    """compute the fibre counts of the free algebra F(X) directly and match f(k)."""
    Xl = list(X)
    PX = allsubs(Xl)
    PPX = allsubs(PX)
    fib = Counter()
    for fam in PPX:
        fib[fam_union(fam)] += 1
    byk = {}
    for K, c in fib.items():
        byk.setdefault(len(K), []).append(c)
    determined = all(len(set(v)) == 1 for v in byk.values())
    print("--- free fibre law, F(X), |X| = " + str(len(Xl)) + " ---")
    print("  |P(X)| = " + str(len(PX)) + "   #families = 2^|P(X)| = " + str(len(PPX)))
    print("  fibre size determined by |K| only : " + str(determined))
    formok = True
    for k in sorted(byk):
        got = set(byk[k])
        pred = free_powerset_union_fibre(k)
        print("    k=" + str(k) + " C(|X|,k)=" + str(len(byk[k])) + " fibre size " + str(sorted(got)) + "   f(k)=" + str(pred) + "   match " + str(got == {pred}))
        if got != {pred}:
            formok = False
    msig = sorted(fib.values())
    print("  fibre multiset : " + str(msig))
    return determined, formok, msig


def stage55_signatures():
    """recompute the Stage-55 measured fibre signatures from the ALGS."""
    print("--- Stage-55 measured fibre signatures  Fib(A) = multiset |alpha^-1(a)| ---")
    out = {}
    for (name, cA, afun, bot) in ALGS:
        PA = allsubs(list(cA))
        cnt = Counter()
        for K in PA:
            cnt[afun(K)] += 1
        sig = sorted(cnt.values())
        out[name] = sig
        print("    " + name + " : " + str(sig) + "   carrier |A|=" + str(len(cA)))
    return out


def free_discriminator(sigs):
    """the free powerset-union fibre law reproduces [2,2,2,10] and rejects A_chain."""
    free2 = sorted([free_powerset_union_fibre(0), free_powerset_union_fibre(1),
                    free_powerset_union_fibre(1), free_powerset_union_fibre(2)])
    print("--- free-sector discriminator ---")
    print("  free-law signature for |X|=2 : " + str(free2))
    ok = True
    for name in ["A_t", "A_i", "A_f", "F(2)"]:
        if name in sigs:
            m = (sigs[name] == free2)
            print("    " + name + " matches free law : " + str(m))
            ok = ok and m
    ch = sigs.get("A_chain")
    print("    A_chain : " + str(ch) + "   free-law form : " + str(ch == free2))
    return ok, (ch != free2)


def block_56B():
    print("=== 56B: FREE POWERSET-UNION FIBRE LAW   f(k)=Sum_j (-1)^j C(k,j) 2^(2^(k-j)) ===")
    det2, form2, msig2 = free_fibre_law(["N", "T"])
    print("")
    det4, form4, msig4 = free_fibre_law(list(FOUR))
    print("")
    sigs = stage55_signatures()
    print("")
    disc_ok, chain_diff = free_discriminator(sigs)
    ok = det2 and form2 and det4 and form4 and disc_ok and chain_diff
    print("  56B : " + str(ok))
    return ok, sigs

def block_56C(sigs):
    print("=== 56C: G-ITERATION   G(A)=F∘U(A),   carrier 4 -> 16 -> 65536 ===")
    ok = True
    print("  underlying carriers U(A) (as ELEMENT SETS; cardinality is what F sees):")
    cards = {}
    for (name, cA, afun, bot) in ALGS:
        cards[name] = len(cA)
        print("    " + name + " : |U(A)|=" + str(len(cA)))
    same_card = all(cards[n] == 4 for n in cards)
    print("  every U(A) has cardinality 4  =>  G(A) ≅ F(FOUR) for every A (incl. A_chain) : " + str(same_card))
    ok = ok and same_card

    G1_sec = 2 ** 4
    G2_carrier = 2 ** 4
    G2_sec = 2 ** G2_carrier
    print("  G(A) = F(U A) = F(FOUR):")
    print("    carrier of G(A) = P(FOUR)          |P(FOUR)|      = " + str(G2_carrier))
    print("    |Sec(G(A))| = 2^|U A| = 2^4       = " + str(G1_sec) + "   = |P(FOUR)|   (Boolean_4)")
    print("  G^2(A) = F(U G(A)) = F(P(FOUR)):")
    print("    carrier P(P(FOUR))                 |.|            = " + str(G2_sec))
    print("    |Sec(G^2(A))| = 2^|P(FOUR)| = 2^16 = " + str(G2_sec) + "   = |P(P(FOUR))|")
    print("  structural stabilization :  Sec(F(X)) ≅ P(X) = U(F(X))  (Boolean)")
    print("    |Sec(F(X))| = 2^|X| = |U F(X)|  ->  the section space tracks the carrier.")

    ch = sigs.get("A_chain")
    fr = sigs.get("F(2)")
    print("  pre-re-entry geometry forgotten by G :")
    print("    A_chain Fib = " + str(ch) + "  !=  free law " + str(fr) + ",  yet G(A_chain) ≅ G(F(2)) ≅ F(FOUR)")
    print("    => G forgets the quotient-fibre difference; the free-cover face is uniform.")
    forget_ok = same_card and (ch != fr)
    ok = ok and forget_ok
    print("  56C : " + str(ok))
    return ok


def main():
    okA = block_56A()
    print("")
    okB, sigs = block_56B()
    print("")
    okC = block_56C(sigs)
    print("")
    total = okA and okB and okC
    print("  STAGE 56 RESULT : " + str(total))
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
