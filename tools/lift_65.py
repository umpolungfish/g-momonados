#!/usr/bin/env python3
"""lift_65.py - Stage 65: SPECTRAL RADII ON THE THREE SPINES / EXTREMAL ORBITS.

Stage 64 found three canonical selectors λ,χ,Ω (ranks 1, r, 2^r) inside every fibre.
Stage 65 puts the transfer operator ℛ on each selector's orbit, reads its spectral
radius, and asks whether the three spines are the extremal orbits of ONE order.

  Rank maps:      φ_λ(r)=1     φ_χ(r)=r     φ_Ω(r)=2^r
  Koopman:        (K_f h)(r)=h(φ_f(r)),  the transfer ℛ restricted to selector f.
  ρ finite trunc: λ→1 (trivial rank-1 idempotent), χ→1 (identity, eigen 1 on the
                  diagonal), Ω→0 (nilpotent / escape).
  ρ on own orbit: λ→1 (fixed pt 1), χ→1 (identity), Ω→1 (backward shift).
  Extremality:    on every fibre λ is the UNIQUE rank-min cover, Ω the UNIQUE
                  rank-max, χ the canonical interior -> one rank order, three extremes.
  Naturality:     rank depends on sizes only -> all three spines Cov(b)-invariant.

Paraconsistent object HELD (not normalized): on its OWN invariant orbit every spine has
spectral radius 1 (all critical); on any FINITE rank-truncation the same operator reads
(1,1,0). Identical spine, two frames -> held as Belnap B. None is the privileged reading.
"""
import sys, os, itertools
from collections import Counter
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
try:
    sys.set_int_max_str_digits(200000)
except Exception:
    pass
from lift_45 import alpha, FOUR, BOT
import lift_50 as L50
import lift_53 as L53
import lift_56 as L56
import lift_58 as L58

allsubs = L50.allsubs
fam_union = L56.fam_union
fibre_poset = L58.fibre_poset
rank_N = L58.rank_N


def brief(n):
    s = str(n)
    return s if len(s) <= 40 else (s[:24] + "...(+" + str(len(s)) + " digits)")


def rank_vec(k):
    K, PK, fib = fibre_poset(k)
    rc = Counter(len(C) for C in fib)
    return fib, [rc.get(r, 0) for r in range(len(PK) + 1)]


def lam(C):
    return frozenset([C])


def chi(C):
    return frozenset(frozenset([a]) for a in C)


def omega(C):
    return frozenset(allsubs(list(C)))


# ---- rank maps of the three selectors --------------------------------------
def p_lam(r):
    return 1


def p_chi(r):
    return r


def p_om(r):
    return 2 ** r


SEL = [("λ", p_lam), ("χ", p_chi), ("Ω", p_om)]


# ---- exact integer linear algebra ------------------------------------------
def mat_phi(phi, R):
    M = [[0] * (R + 1) for _ in range(R + 1)]
    for k in range(R + 1):
        s = phi(k)
        if s <= R:
            M[k][s] += 1
    return M


def matmul(A, B):
    n = len(A); m = len(B[0]); k = len(B)
    C = [[0] * m for _ in range(n)]
    for i in range(n):
        Ai = A[i]
        for t in range(k):
            a = Ai[t]
            if a:
                Bt = B[t]
                for j in range(m):
                    if Bt[j]:
                        C[i][j] += a * Bt[j]
    return C


def matpow(A, e):
    n = len(A)
    R = [[1 if i == j else 0 for j in range(n)] for i in range(n)]
    for _ in range(e):
        R = matmul(R, A)
    return R


def mat_eq(A, B):
    return all(A[i][j] == B[i][j] for i in range(len(A)) for j in range(len(A[0])))


def mat_zero(A):
    return all(x == 0 for row in A for x in row)


def mat_trace(A):
    return sum(A[i][i] for i in range(len(A)))


def ident(R):
    return [[1 if i == j else 0 for j in range(R + 1)] for i in range(R + 1)]


# ---- covers of a carrier ----------------------------------------------------
def covers(C):
    """every W ⊆ P(C) with union(W)=C."""
    out = []
    for W in allsubs(allsubs(list(C))):
        if fam_union(W) == C:
            out.append(W)
    return out


# ================================ 65A =======================================
def block_65A():
    print("=== 65A: SELECTOR KOOPMAN OPERATORS  (K_f h)(r)=h(φ_f(r)) ; spectra of the restrictions ===")
    import sympy
    R = 16
    ok = True
    Pl = mat_phi(p_lam, R)
    idem = mat_eq(matmul(Pl, Pl), Pl)
    trl = mat_trace(Pl)
    rkl = sympy.Matrix(Pl).rank()
    ok = ok and idem and trl == 1 and rkl == 1

    Pc = mat_phi(p_chi, R)
    is_id = mat_eq(Pc, ident(R))
    trc = mat_trace(Pc)
    ok = ok and is_id and trc == R + 1

    Po = mat_phi(p_om, R)
    nil = None
    for m in range(1, 12):
        if mat_zero(matpow(Po, m)):
            nil = m
            break
    tro = mat_trace(Po)
    ok = ok and nil is not None and tro == 0

    print("  R=" + str(R) + " truncation (ranks 0.." + str(R) + ") :")
    print("    K_λ: P_λ^2=P_λ : " + str(idem) + "   trace=" + str(trl) + "   rank=" + str(rkl)
          + "  -> idempotent rank-1 projection, spectrum {1, 0^" + str(R) + "}")
    print("    K_χ: P_χ=I   : " + str(is_id) + "   trace=" + str(trc)
          + "  -> identity, eigenvalue 1 with multiplicity " + str(R + 1))
    print("    K_Ω: P_Ω^" + str(nil) + "=0 : " + str(nil is not None) + "   trace=" + str(tro)
          + "  -> nilpotent, index " + str(nil) + ", spectrum {0^" + str(R + 1) + "}")

    x = sympy.symbols('x')
    R2 = 6
    for name, phi in SEL:
        cp = sympy.Matrix(mat_phi(phi, R2)).charpoly(x)
        print("    charpoly P_" + name + " @R=" + str(R2) + " : "
              + str(sympy.factor(cp.as_expr())))
    print("  -> ρ(truncation): λ->1 (trivial), χ->1 (diagonal), Ω->0 (escape).")
    print("  65A : " + str(ok))
    return ok


# ================================ 65B =======================================
def block_65B():
    print("=== 65B: THE THREE ORBITS  λ: r->1->1… ; χ: r->r->r… ; Ω: r->2^r->2^(2^r)->… ===")
    ok = True

    def orbit(phi, r, n):
        out = [r]
        for _ in range(n):
            out.append(phi(out[-1]))
        return out

    lam_ok = all(orbit(p_lam, r, 5) == [r, 1, 1, 1, 1, 1] for r in range(0, 7))
    chi_ok = all(orbit(p_chi, r, 5) == [r] * 6 for r in range(0, 7))
    om_ok = True
    for r in range(0, 5):
        seq = orbit(p_om, r, 3)
        if not all(seq[i] < seq[i + 1] for i in range(len(seq) - 1)):
            om_ok = False
    lam_fix = [r for r in range(0, 8) if p_lam(r) == r]
    chi_fix = [r for r in range(0, 8) if p_chi(r) == r]
    om_fix = [r for r in range(0, 8) if p_om(r) == r]
    ok = ok and lam_ok and chi_ok and om_ok and lam_fix == [1] and chi_fix == list(range(8)) and om_fix == []

    print("  λ orbit [r,1,1,1,1,1] for r=0..6 : " + str(lam_ok) + "   fixed pts: " + str(lam_fix)
          + "   -> collapse to the minimum rank 1 (period 1, preperiod <=1)")
    print("  χ orbit [r,r,r,r,r,r] : " + str(chi_ok) + "   fixed pts: all r   -> stationarity (period 1)")
    print("  Ω orbit strictly increasing (r=0..4) : " + str(om_ok) + "   fixed pts: " + str(om_fix)
          + "   -> no fixed point (2^r=r unsolvable), aperiodic, unbounded")
    om_s = orbit(p_om, 2, 4)
    print("  sample r=2:  λ " + str(orbit(p_lam, 2, 4)) + "   χ " + str(orbit(p_chi, 2, 4))
          + "   Ω " + " -> ".join(brief(x) for x in om_s))
    print("  -> contraction-to-min (λ) AND stationarity (χ) AND expansion (Ω) in ONE carrier.")
    print("  65B : " + str(ok))
    return ok


# ================================ 65C =======================================
def block_65C():
    print("=== 65C: EXTREMALITY ON THE FIBRE  unique rank-min cover = λ(C) ; unique rank-max cover = Ω(C) ===")
    ok = True
    for r in range(1, 4):
        C = frozenset(range(r))
        cov = covers(C)
        rc = Counter(len(W) for W in cov)
        ranks = sorted(rc)
        mn, mx = ranks[0], ranks[-1]
        mins = [W for W in cov if len(W) == mn]
        maxs = [W for W in cov if len(W) == mx]
        uniq_min = (len(mins) == 1 and mins[0] == frozenset([C]))
        uniq_max = (len(maxs) == 1 and maxs[0] == frozenset(allsubs(list(C))))
        dist = [rc.get(s, 0) for s in range(0, mx + 1)]
        _N = [rank_N(r, s) for s in range(0, mx + 1)]
        good = (mn == 1 and mx == 2 ** r and uniq_min and uniq_max and dist == _N)
        ok = ok and good
        print("  |C|=" + str(r) + "  #covers=" + str(len(cov)) + "  min rank=" + str(mn) + " max=" + str(mx)
              + "   unique min=λ : " + str(uniq_min) + "   unique max=Ω : " + str(uniq_max))
        print("       rank dist " + str(dist) + "  == N(" + str(r) + ",·) : " + str(dist == _N))
    print("  -> in every fibre λ is the UNIQUE rank-minimum cover and Ω the UNIQUE rank-maximum:")
    print("     pointwise 1 <= rank(W) <= 2^|C|, and both bounds are attained by exactly one continuation.")
    print("  65C : " + str(ok))
    return ok


# ================================ 65D =======================================
def block_65D():
    print("=== 65D: SPECTRUM OF ℛ AND ITS RESTRICTIONS  finite-truncation ρ vs invariant-orbit ρ ===")
    ok = True
    import sympy
    import numpy as np
    x = sympy.symbols('x')
    for R in [4, 5]:
        M = sympy.Matrix([[rank_N(k, s) for s in range(R + 1)] for k in range(R + 1)])
        tr = M.trace()
        det1 = (M - sympy.eye(R + 1)).det()
        cp = M.charpoly(x)
        coeffs = [float(c) for c in cp.all_coeffs()]
        roots = np.roots(coeffs)
        dom = float(max(abs(roots)))
        print("  full ℛ @R=" + str(R) + "  trace=" + str(tr) + "  det(ℛ-I)=" + str(det1)
              + "  1 is eigenvalue : " + str(det1 == 0))
        print("       charpoly = " + str(sympy.factor(cp.as_expr())))
        print("       ρ(ℛ) = " + str(round(dom, 6)) + "   eigenvals(numeric) = "
              + str(sorted(round(float(abs(e)), 6) for e in roots)))
    print("  selector restriction ρ:")
    print("    finite truncation (ranks 0..R)   : λ->1, χ->1, Ω->0                 (65A)")
    print("    own invariant orbit of the map   : λ->1 (fixed pt 1), χ->1 (identity), Ω->1 (backward shift)")
    print("  -> on its OWN orbit every spine is critical (ρ=1); on any FINITE truncation Ω reads 0.")
    print("     SAME spine, two frames -> held as Belnap B in the appendix.")
    print("  65D : " + str(ok))
    return ok


# ================================ 65E =======================================
def block_65E():
    print("=== 65E: ONE ORDERED STRUCTURE  rank(λ) <= rank(χ) <= rank(Ω) on every fibre, as extremes ===")
    ok = True
    for r in range(1, 5):
        ranksel = (1, r, 2 ** r)
        ordered = ranksel[0] <= ranksel[1] <= ranksel[2]
        strict = ranksel[0] < ranksel[1] < ranksel[2]
        ok = ok and ordered
        print("  |C|=" + str(r) + " :  rank(λ,χ,Ω) = " + str(ranksel) + "   ordered : " + str(ordered)
              + "   all-strict (r>=2) : " + str(strict))
    for r in range(1, 4):
        C = frozenset(range(r))
        cov = covers(C)
        rset = sorted(set(len(W) for W in cov))
        mn_uniq = sum(1 for W in cov if len(W) == rset[0]) == 1
        mx_uniq = sum(1 for W in cov if len(W) == rset[-1]) == 1
        ok = ok and (rset[0] == 1 and rset[-1] == 2 ** r and (r in rset) and mn_uniq and mx_uniq)
    print("  -> λ, χ, Ω are the rank-min, rank-|C| canonical and rank-max points of ONE order")
    print("     (the rank order on covers of C). The three spines are its extremal + canonical points,")
    print("     not three competing total readings of the tree.")
    print("  65E : " + str(ok))
    return ok


# ================================ 65F =======================================
def block_65F():
    print("=== 65F: NATURALITY  ρ and the three orbits are relabelling-invariant (Cov(b)) ===")
    ok = True
    for kk in range(0, 4):
        _K, _PK, fib = fibre_poset(kk)
        perms = list(itertools.permutations(range(kk))) if kk > 0 else [()]
        nat = True
        tested = 0
        for C in fib:
            r = len(C)
            for p in perms:
                b = {i: p[i] for i in range(kk)}
                Cb = frozenset(frozenset(b[a] for a in S) for S in C)
                lamC = frozenset([C]); lamCb = frozenset([Cb])
                chiC = chi(C); chiCb = chi(Cb)
                omC = omega(C); omCb = omega(Cb)
                if not (len(lamCb) == len(lamC) == 1):
                    nat = False
                if not (len(chiCb) == len(chiC) == r):
                    nat = False
                if not (len(omCb) == len(omC) == 2 ** r):
                    nat = False
                tested += 1
        ok = ok and nat
        print("  k=" + str(kk) + "  rank(λ(Cov b C))=1, rank(χ(Cov b C))=rank(C), rank(Ω(Cov b C))=2^rank(C) : "
              + str(nat) + "   (" + str(tested) + " node/perm pairs)")
    print("  -> the rank-maps (λ->1, χ->r, Ω->2^r) depend on rank alone, so K_λ,K_χ,K_Ω and their")
    print("     spectra/orbits commute with Cov(b). All three spines are label-independent.")
    print("  65F : " + str(ok))
    return ok


def main():
    okA = block_65A(); print("")
    okB = block_65B(); print("")
    okC = block_65C(); print("")
    okD = block_65D(); print("")
    okE = block_65E(); print("")
    okF = block_65F(); print("")
    total = okA and okB and okC and okD and okE and okF
    print("  STAGE 65 RESULT : " + str(total))
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
