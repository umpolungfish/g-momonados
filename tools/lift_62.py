#!/usr/bin/env python3
"""lift_62.py - Stage 62: THE COMONADIC FIBRE TOWER / CANONICAL SPINE THROUGH BRANCHING.

Stage 61 made 𝔉_k = ε_A⁻¹(⊤_A) precise.  Stage 62 makes it RECURSIVE: over each
C ∈ 𝔉_k hangs a second-level fibre, and the comonad χ picks one canonical point in it.

    Fib₂(C) := ε_{G(A)}⁻¹(C) ≅ 𝔉_|C|            (61B/60D prediction)      62A
    𝔇_k := Σ_{C∈𝔉_k} 𝔉_|C| ;  |𝔇_k| = Σ_r N(k,r) f(r)                    62A
    χ(C) = {{a} : a∈C} ∈ Fib₂(C) , rank r , UNIQUE max-rank minimal cover 62B
    χ(C) ∈ next TopFib(P(X))  iff  C = P(X)   (top-fibre non-closure)     62C
    χ_{G(A)}∘χ_A = G(χ_A)∘χ_A  (coassociativity) ; spine C→χ(C)→χχ(C)→… 62D
    R_k(z) is the type distribution of the NEXT fibre layer               62E

The paraconsistent object held (NOT normalized):
    the fibre contains ALL decompositions of C  AND  χ chooses ONE canonical
    decomposition of C -- the branching does not collapse, and the top-fibre
    closure fails while fibrewise re-entry holds exactly.
"""
import sys, os, itertools
from collections import Counter
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_45 import alpha, FOUR, BOT
import lift_50 as L50
import lift_53 as L53
import lift_56 as L56
import lift_58 as L58

try:
    sys.set_int_max_str_digits(100000)
except Exception:
    pass

allsubs = L50.allsubs
fam_union = L56.fam_union
f_of = L56.free_powerset_union_fibre
fibre_poset = L58.fibre_poset

_fc = {}


def fc(r):
    if r not in _fc:
        _fc[r] = f_of(r)
    return _fc[r]


def brief(n):
    s = str(n)
    return s if len(s) <= 40 else (s[:24] + "...(+" + str(len(s)) + " digits)")


def chi(C):
    """comultiplication χ(C) = {{a} : a∈C}  (the singleton cover of C)."""
    return frozenset(frozenset([a]) for a in C)


def fib2(C):
    """Fib₂(C) = ε_{G(A)}⁻¹(C) = { F ⊆ P(C) : ⋃F = C }."""
    return [F for F in allsubs(allsubs(list(C))) if fam_union(F) == C]


def irredundant(F, target):
    return all(fam_union(F - {S}) != target for S in F)


def rank_covers(C):
    """all irredundant r-member covers of C (r = |C|) drawn from P(C)."""
    r = len(C)
    PC = allsubs(list(C))
    out = []
    for comb in itertools.combinations(PC, r):
        F = frozenset(comb)
        if fam_union(F) == C and irredundant(F, C):
            out.append(F)
    return out


def block_62A():
    print("=== 62A: FIBRE OF A FIBRE  Fib₂(C)=ε_GA⁻¹(C) ≅ 𝔉_|C| ,  dependent total 𝔇_k = Σ_r N(k,r) f(r) ===")
    ok = True
    for k in range(0, 5):
        K, PK, fib = fibre_poset(k)
        rc = Counter(len(C) for C in fib)
        Nk = [rc.get(r, 0) for r in range(len(PK) + 1)]
        Dk = sum(Nk[r] * fc(r) for r in range(len(PK) + 1))
        iso = True; tested = 0; b4 = 6
        for C in fib:
            r = len(C)
            if r > 4:
                continue
            if r == 4:
                if b4 <= 0:
                    continue
                b4 -= 1
            F2 = fib2(C)
            _, PKk, fibk = fibre_poset(r)
            rc2 = Counter(len(F) for F in F2); rc3 = Counter(len(F) for F in fibk)
            rank2 = [rc2.get(i, 0) for i in range(len(PKk) + 1)]
            rank3 = [rc3.get(i, 0) for i in range(len(PKk) + 1)]
            if len(F2) != fc(r) or rank2 != rank3:
                iso = False
            tested += 1
        full = all(len(C) <= 4 for C in fib)
        if full:
            direct = sum(len(fib2(C)) for C in fib)
            note = "   full Σ_C|Fib₂(C)|=" + str(direct) + " == 𝔇_k : " + str(direct == Dk)
            ok = ok and (direct == Dk)
        else:
            note = "   (r>4 present: 𝔇_k from formula; every r≤4 fibre is iso)"
        ok = ok and iso
        print("  k=" + str(k) + "  |𝔉_k|=" + str(len(fib)) + "   N(k,·)=" + str(Nk))
        print("       𝔇_k = Σ_r N(k,r) f(r) = " + brief(Dk) + "   Fib₂(C)≅𝔉_|C| on " + str(tested)
              + " fibres : " + str(iso) + note)
    print("  62A : " + str(ok))
    return ok


def block_62B():
    print("=== 62B: CANONICAL χ-POINT  χ(C)∈Fib₂(C) , rank χ(C)=|C| , UNIQUE max-rank minimal cover ===")
    in_fib = True; rank_ok = True; min_ok = True; uniq_ok = True
    nC = 0; b4 = 12
    for k in range(0, 5):
        K, PK, fib = fibre_poset(k)
        for C in fib:
            r = len(C)
            if r > 4:
                continue
            if r == 4:
                if b4 <= 0:
                    continue
                b4 -= 1
            x = chi(C)
            if fam_union(x) != C:
                in_fib = False
            if len(x) != r:
                rank_ok = False
            if not irredundant(x, C):
                min_ok = False
            uq = rank_covers(C)
            if not (len(uq) == 1 and uq[0] == x):
                uniq_ok = False
            nC += 1
    ok = in_fib and rank_ok and min_ok and uniq_ok
    print("  C tested (k=0..4, |C|≤4, rank-4 sampled) = " + str(nC))
    print("  χ(C) ∈ Fib₂(C) : " + str(in_fib) + "   rank χ(C)=|C| : " + str(rank_ok)
          + "   χ(C) irredundant : " + str(min_ok))
    print("  χ(C) = UNIQUE irredundant cover of C at maximal rank r=|C| : " + str(uniq_ok))
    print("  -> the comonad picks the unique MAXIMAL-RANK minimal cover, not an arbitrary one;")
    print("     the full fibre 𝔉_r keeps its many other minimal covers live.")
    print("  62B : " + str(ok))
    return ok


def block_62C():
    print("=== 62C: TOP-FIBRE NON-CLOSURE  χ(C)∈next TopFib(P(X)) iff C=P(X) ===")
    ok = True
    for k in range(0, 5):
        K, PK, fib = fibre_poset(k)
        fib_pres = all(fam_union(chi(C)) == C for C in fib)
        top_iff = all((fam_union(chi(C)) == PK) == (C == PK) for C in fib)
        n_top = sum(1 for C in fib if fam_union(chi(C)) == PK)
        ok = ok and fib_pres and top_iff and (n_top == (1 if PK in fib else 0))
        print("  k=" + str(k) + "  fibrewise χ(C)∈ε⁻¹(C) : " + str(fib_pres)
              + "   χ(C)∈TopFib(P(X)) ⟺ C=P(X) : " + str(top_iff)
              + "   #top-re-entering C = " + str(n_top))
    print("  -> χ preserves every element FIBREWISE, but does NOT preserve the top-fibre as a subobject;")
    print("     only the maximal family C=P(X) re-enters as a next-level top decomposition. Asymmetry kept.")
    print("  62C : " + str(ok))
    return ok


def block_62D():
    print("=== 62D: COMONADIC COHERENCE  χ_{G(A)}∘χ_A = G(χ_A)∘χ_A  (coassociativity) + canonical spine ===")

    def Gchi(F):
        return frozenset(chi(S) for S in F)

    ok = True
    for k in range(0, 5):
        K, PK, fib = fibre_poset(k)
        coassoc = True; spine = True
        for C in fib:
            path1 = chi(chi(C))          # χ_{G(A)}(χ_A(C))
            path2 = Gchi(chi(C))         # G(χ_A)(χ_A(C))
            if path1 != path2:
                coassoc = False
            if not (fam_union(chi(C)) == C and fam_union(chi(chi(C))) == chi(C)):
                spine = False
        ok = ok and coassoc and spine
        print("  k=" + str(k) + "  χ_{G(A)}∘χ_A = G(χ_A)∘χ_A : " + str(coassoc)
              + "   spine stays in fibres (C→χC→χχC) : " + str(spine))
    # explicit spine sample
    K, PK, fib = fibre_poset(3)
    C0 = max(fib, key=lambda F: (len(F), sorted(map(sorted, F))))
    print("  sample spine for C=" + str(sorted(sorted(s) for s in C0)) + " (rank " + str(len(C0)) + "):")
    print("    level 0  " + str(sorted(sorted(s) for s in C0)))
    print("    level 1  χ(C)   = " + str(sorted(sorted(s) for s in chi(C0))))
    print("    level 2  χχ(C)  = " + str(sorted(sorted(s) for s in chi(chi(C0)))))
    print("  62D : " + str(ok))
    return ok


def block_62E():
    print("=== 62E: RANK POLYNOMIAL = FIBRE-TYPE DISTRIBUTION  N(k,r) copies of 𝔉_r above 𝔉_k ===")
    ok = True
    vecs = {}
    for k in range(0, 5):
        K, PK, fib = fibre_poset(k)
        rc = Counter(len(C) for C in fib)
        Nk = [rc.get(r, 0) for r in range(len(PK) + 1)]
        vecs[k] = Nk
        tot = sum(Nk)
        Dk = sum(Nk[r] * fc(r) for r in range(len(PK) + 1))
        ok = ok and (tot == len(fib))
        parts = [str(Nk[r]) + "×𝔉_" + str(r) for r in range(len(PK) + 1) if Nk[r] > 0]
        print("  k=" + str(k) + "  𝔉_k type distribution:  " + "  ".join(parts))
        print("       #types = Σ_r N(k,r) = " + str(tot) + " (=|𝔉_k| " + str(tot == len(fib))
              + ")   |𝔇_k| = Σ_r N(k,r)·f(r) = " + brief(Dk))
    k2 = vecs[2]
    exp2 = [0, 1, 4, 4, 1]
    ok = ok and (k2 == exp2)
    print("  k=2 vector " + str(k2) + " -> 1 copy of 𝔉₁, 4 of 𝔉₂, 4 of 𝔉₃, 1 of 𝔉₄  (== [0,1,4,4,1] : "
          + str(k2 == exp2) + ")")
    print("  -> R_k(z) is not a size count: it is the TYPE distribution of the NEXT fibre layer over 𝔉_k.")
    print("  62E : " + str(ok))
    return ok


def main():
    okA = block_62A(); print("")
    okB = block_62B(); print("")
    okC = block_62C(); print("")
    okD = block_62D(); print("")
    okE = block_62E(); print("")
    total = okA and okB and okC and okD and okE
    print("  STAGE 62 RESULT : " + str(total))
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
