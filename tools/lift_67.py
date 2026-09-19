#!/usr/bin/env python3
"""
STAGE 67 — THE COUPLING ALGEBRA / MÖBIUS RECONSTRUCTION OF THE FIBRE

Self-contained exact audit of the finite numerical face plus the all-finite-R
factorization theorem suggested by the CL9NK roadmap.

No external packages are required for the exact checks. NumPy is used only
optionally for the N_6 floating spectrum.
"""
from __future__ import annotations

from collections import Counter
from fractions import Fraction
from math import comb, factorial
from typing import Dict, Iterable, List, Tuple


def C(n: int, r: int) -> int:
    return 0 if r < 0 or r > n else comb(n, r)


def N(k: int, r: int) -> int:
    """Number of r-element families of subsets of [k] whose union is [k]."""
    return sum(((-1) ** (k - i)) * C(k, i) * C(2 ** i, r) for i in range(k + 1))


def matmul(A, B):
    n, m, p = len(A), len(B), len(B[0])
    assert len(A[0]) == m
    return [[sum(A[i][t] * B[t][j] for t in range(m)) for j in range(p)] for i in range(n)]


def eye(n: int):
    return [[Fraction(int(i == j)) for j in range(n)] for i in range(n)]


def det_fraction(A) -> Fraction:
    M = [[Fraction(x) for x in row] for row in A]
    n = len(M)
    d = Fraction(1)
    for col in range(n):
        pivot = next((r for r in range(col, n) if M[r][col] != 0), None)
        if pivot is None:
            return Fraction(0)
        if pivot != col:
            M[col], M[pivot] = M[pivot], M[col]
            d = -d
        pv = M[col][col]
        d *= pv
        for j in range(col, n):
            M[col][j] /= pv
        for r in range(col + 1, n):
            f = M[r][col]
            if f:
                for j in range(col, n):
                    M[r][j] -= f * M[col][j]
    return d


def inv_fraction(A):
    n = len(A)
    M = [[Fraction(x) for x in row] + eye(n)[i] for i, row in enumerate(A)]
    for col in range(n):
        pivot = next((r for r in range(col, n) if M[r][col] != 0), None)
        if pivot is None:
            raise ValueError("singular")
        M[col], M[pivot] = M[pivot], M[col]
        pv = M[col][col]
        M[col] = [x / pv for x in M[col]]
        for r in range(n):
            if r == col:
                continue
            f = M[r][col]
            if f:
                M[r] = [a - f * b for a, b in zip(M[r], M[col])]
    return [row[n:] for row in M]


def N_matrix(R: int):
    return [[N(k, r) for r in range(R + 1)] for k in range(R + 1)]


def D_matrix(R: int):
    return [[((-1) ** (k - i)) * C(k, i) if i <= k else 0 for i in range(R + 1)] for k in range(R + 1)]


def Z_matrix(R: int):
    return [[C(k, i) if i <= k else 0 for i in range(R + 1)] for k in range(R + 1)]


def B_matrix(R: int):
    return [[C(2 ** i, r) for r in range(R + 1)] for i in range(R + 1)]


def stirling1_signed(n: int, k: int, memo={}) -> int:
    if (n, k) in memo:
        return memo[n, k]
    if n == 0 and k == 0:
        return 1
    if n == 0 or k == 0 or k > n:
        return 0
    v = stirling1_signed(n - 1, k - 1) - (n - 1) * stirling1_signed(n - 1, k)
    memo[n, k] = v
    return v


def V_matrix(R: int):
    return [[Fraction((2 ** i) ** m) for m in range(R + 1)] for i in range(R + 1)]


def S_matrix(R: int):
    # columns r express binom(x,r) in monomial basis x^m
    return [[Fraction(stirling1_signed(r, m), factorial(r)) for r in range(R + 1)] for m in range(R + 1)]


def det_formula(R: int) -> Fraction:
    num = 1
    for i in range(R + 1):
        for j in range(i + 1, R + 1):
            num *= (2 ** j - 2 ** i)
    den = 1
    for r in range(R + 1):
        den *= factorial(r)
    return Fraction(num, den)


def det_q_product(R: int) -> Fraction:
    num = 2 ** (R * (R - 1) * (R + 1) // 6)
    for d in range(1, R + 1):
        num *= (2 ** d - 1) ** (R + 1 - d)
    den = 1
    for r in range(1, R + 1):
        den *= factorial(r)
    return Fraction(num, den)


def cover_union(family: int, k: int) -> int:
    u = 0
    s = 0
    f = family
    while f:
        if f & 1:
            u |= s
        s += 1
        f >>= 1
    return u


def all_covers(k: int) -> List[int]:
    nsub = 2 ** k
    full = (1 << k) - 1
    out = []
    for fam in range(1 << nsub):
        u = 0
        for subset in range(nsub):
            if (fam >> subset) & 1:
                u |= subset
        if u == full:
            out.append(fam)
    return out


def minimal_covers(k: int) -> Tuple[List[int], Dict[int, bool]]:
    nsub = 2 ** k
    full = (1 << k) - 1
    is_cover = {}
    for fam in range(1 << nsub):
        u = 0
        for subset in range(nsub):
            if (fam >> subset) & 1:
                u |= subset
        is_cover[fam] = (u == full)
    mins = []
    for fam, ok in is_cover.items():
        if not ok:
            continue
        minimal = True
        bits = fam
        while bits:
            lsb = bits & -bits
            if is_cover[fam ^ lsb]:
                minimal = False
                break
            bits ^= lsb
        if minimal:
            mins.append(fam)
    return mins, is_cover


def union_semilattice_with_mobius(gens: Iterable[int]) -> Dict[int, int]:
    # Crosscut expansion: coefficient of union U in prod_g (1 - [g]).
    coeff: Dict[int, int] = {0: 1}
    for g in gens:
        old = dict(coeff)
        for u, c in old.items():
            v = u | g
            coeff[v] = coeff.get(v, 0) - c
    return coeff


def rank_poly_from_mobius(k: int, mu: Dict[int, int]) -> List[int]:
    nsub = 2 ** k
    coeff = [0] * (nsub + 1)
    for U, m in mu.items():
        if U == 0:
            continue
        q = U.bit_count()
        # -mu(U) * z^q (1+z)^(nsub-q)
        for j in range(nsub - q + 1):
            coeff[q + j] += -m * C(nsub - q, j)
    return coeff


def mobius_recurrence_check(mu: Dict[int, int]) -> bool:
    keys = list(mu)
    for U in keys:
        s = sum(mu[V] for V in keys if (V | U) == U)  # V subset U as family bitsets
        if s != (1 if U == 0 else 0):
            return False
    return True


def transitive_closure(adj: List[List[bool]]) -> List[List[bool]]:
    R = len(adj)
    reach = [row[:] for row in adj]
    for i in range(R):
        reach[i][i] = True
    for m in range(R):
        for i in range(R):
            if reach[i][m]:
                for j in range(R):
                    reach[i][j] = reach[i][j] or reach[m][j]
    return reach


def outer(col, row):
    return [[col[i] * row[j] for j in range(len(row))] for i in range(len(col))]


def matsub(A, B):
    return [[a - b for a, b in zip(ra, rb)] for ra, rb in zip(A, B)]


def matscale(c, A):
    return [[c * x for x in row] for row in A]


def E_matrix(NR, r: int, s: int):
    n = len(NR)
    col = [NR[k][r] for k in range(n)]
    row = [1 if j == s else 0 for j in range(n)]
    return outer(col, row)


def audit_coupling_commutators(R: int = 4) -> bool:
    NR = N_matrix(R)
    for r in range(R + 1):
        for s in range(R + 1):
            E_rs = E_matrix(NR, r, s)
            for t in range(R + 1):
                for u in range(R + 1):
                    E_tu = E_matrix(NR, t, u)
                    lhs = matsub(matmul(E_rs, E_tu), matmul(E_tu, E_rs))
                    rhs = matsub(matscale(N(s, t), E_matrix(NR, r, u)),
                                 matscale(N(u, r), E_matrix(NR, t, s)))
                    if lhs != rhs:
                        return False
    return True


def fmt_int(n: int) -> str:
    return f"{n:,}"


def main():
    print("STAGE 67 — COUPLING ALGEBRA / MÖBIUS RECONSTRUCTION")
    print("=" * 72)

    # 67A: exact finite kernel measurements
    expected_dets = [1, 3, 84, 70560, 5879623680, 164618926754365440]
    measured = []
    for R in range(1, 7):
        d = det_fraction(N_matrix(R))
        assert d.denominator == 1
        measured.append(d.numerator)
    assert measured == expected_dets
    print("67A finite determinants R=1..6: TRUE")
    for R, d in enumerate(measured, 1):
        print(f"  det N_{R} = {fmt_int(d)}")

    # 67B: factorization / all finite-R theorem audit
    for R in range(0, 11):
        NR = [[Fraction(x) for x in row] for row in N_matrix(R)]
        D = [[Fraction(x) for x in row] for row in D_matrix(R)]
        Z = [[Fraction(x) for x in row] for row in Z_matrix(R)]
        B = [[Fraction(x) for x in row] for row in B_matrix(R)]
        V = V_matrix(R)
        S = S_matrix(R)
        assert matmul(D, B) == NR
        assert matmul(D, Z) == eye(R + 1)
        assert matmul(Z, D) == eye(R + 1)
        assert matmul(V, S) == B
        assert matmul(matmul(D, V), S) == NR
        assert det_fraction(NR) == det_formula(R) == det_q_product(R)
        assert det_formula(R) != 0
    print("67B N_R = D_R B_R = D_R V_R S_R, audited R=0..10: TRUE")
    print("  all-finite-R proof: det N_R = prod_{i<j}(2^j-2^i) / prod_r r! != 0")
    print("  Boolean roundtrip: D_R Z_R = Z_R D_R = I")

    # N6 spectrum is an optional floating audit
    N6 = N_matrix(6)
    try:
        import numpy as np
        vals = np.linalg.eigvals(np.array(N6, dtype=float))
        vals = sorted((float(v.real) for v in vals), key=lambda x: x)
        rho = max(abs(v) for v in vals)
        print(f"67C spectral radius rho(N_6) = {rho:.10f}")
        print("  eig(N_6) ≈", [round(v, 9) for v in vals])
    except Exception as e:
        print("67C spectrum skipped (NumPy unavailable):", e)

    # 67D: reachability + all finite parabolic dimension
    for R in range(1, 13):
        adj = [[N(r, s) > 0 for s in range(R + 1)] for r in range(R + 1)]
        reach = transitive_closure(adj)
        for r in range(R + 1):
            for s in range(R + 1):
                expected = (r == 0) or (s > 0)
                assert reach[r][s] == expected
        reachable_pairs = sum(reach[r][s] for r in range(R + 1) for s in range(R + 1))
        assert reachable_pairs == 1 + R + R * R
        NR = N_matrix(R)
        inv = inv_fraction(NR)
        assert all(NR[r][0] == 0 for r in range(1, R + 1))
        assert all(inv[r][0] == 0 for r in range(1, R + 1))
    print("67D positive-sector reachability / dim = R^2+R+1, audited R=1..12: TRUE")
    print("  theorem mechanism: N(r,s)>0 for r>=1 and 1<=s<=2^r")

    # 67E: coupling commutator
    assert audit_coupling_commutators(4)
    print("67E [E_rs,E_tu] = N(s,t)E_ru - N(u,r)E_ts, exhaustive R=4: TRUE")

    # 67F: k=4 fibre, minimal covers, union semilattice and Möbius data
    covers4 = all_covers(4)
    mins4, is_cover4 = minimal_covers(4)
    assert len(covers4) == 64594
    assert len(mins4) == 49
    mu4 = union_semilattice_with_mobius(mins4)
    assert len(mu4) == 23992
    assert mobius_recurrence_check(mu4)
    dist = Counter(v for U, v in mu4.items() if U != 0)
    expected_dist = Counter({-3: 17, -2: 636, -1: 6359, 0: 9968, 1: 6358, 2: 636, 3: 17})
    assert dist == expected_dist
    poly4 = rank_poly_from_mobius(4, mu4)
    expected_poly4 = [N(4, r) for r in range(17)]
    assert poly4 == expected_poly4
    assert sum(poly4) == 64594
    print("67F k=4 fibre / Möbius reconstruction: TRUE")
    print("  |F_4| = 64,594")
    print("  |Min_4| = 49")
    print("  |L_4| = 23,992")
    print("  mu distribution (U>empty):", dict(sorted(dist.items())))
    print("  R_4 coefficients:", poly4)

    # join-semilattice but not meet-semilattice witness
    # family bit s denotes subset s of [4]
    U = (1 << 1) | (1 << 2) | (1 << 4) | (1 << 8)   # {{0},{1},{2},{3}}
    Vw = (1 << 3) | (1 << 4) | (1 << 8)             # {{0,1},{2},{3}}
    W = U & Vw                                         # {{2},{3}}
    assert U in mu4 and Vw in mu4
    assert W not in mu4
    assert not is_cover4[W]
    print("67G L_4 join-semilattice AND not meet-closed: TRUE")
    print("  witness U∩V={{2},{3}} is not a cover and not in L_4")

    # closed-form k=6 values
    f6 = sum(N(6, r) for r in range(65))
    assert f6 == 18446744047940725978
    print("67H closed-form k=6 rank distribution: TRUE")
    print("  f(6) =", fmt_int(f6))
    print("  tail N(6,60..64) =", [N(6, r) for r in range(60, 65)])

    print("\nPARACONSISTENT LANDING")
    print("  transformed IMASM closure T AND bare identity protocol B are retained as distinct measured faces")
    print("  L_4 is not meet-closed AND its Möbius overlap calculus reconstructs N(4,r) exactly")
    print("  N_R is a weighted coupling kernel AND an invertible change-of-basis matrix for every finite R")
    print("\nSTAGE 67 RESULT : True")


if __name__ == "__main__":
    main()
