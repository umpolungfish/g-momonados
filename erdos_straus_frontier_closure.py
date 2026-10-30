# Reaches the two rungs the price-zero paper's frontier cascade names as
# needing the full M^2 exhaustion rather than a plain divisor of M: n=2521 at
# rung 23, n=196561 at rung 27. The earlier draft of this check restricted the
# search to u=M*t for t | M, the narrower cofactor family, and found nothing
# for 2521 -- that family is a strict subset of M^2's divisors, and 2521 is
# exactly the case that needs the rest of it. This enumerates every divisor of
# M^2 directly, confirms both rungs, then carries each through to an explicit
# (a, b, c) triple and checks 4/n == 1/a + 1/b + 1/c in exact rational
# arithmetic, not the congruence alone.

def factorize(n):
    f = {}
    d = 2
    while d*d <= n:
        while n % d == 0:
            f[d] = f.get(d,0)+1; n //= d
        d += 1
    if n > 1: f[n] = f.get(n,0)+1
    return f

def divisors_from_factorization(fac):
    divs = [1]
    for p, e in fac.items():
        divs = [d * p**k for d in divs for k in range(e+1)]
    return divs

def true_m2_rung(n, r):
    """Full exhaustion: does SOME divisor of M^2 close n at rung r?"""
    if (n+r) % 4 != 0: return None
    a = (n+r)//4
    M = n*a
    facM = factorize(M)
    facM2 = {p: 2*e for p, e in facM.items()}
    divsM2 = divisors_from_factorization(facM2)
    target = (-M) % r
    for u in divsM2:
        if u % r == target:
            return (u, M, a)
    return None

def search_true_m2(n, max_r):
    for r in range(3, max_r, 4):
        hit = true_m2_rung(n, r)
        if hit:
            return r, hit
    return None

for n, claimed_r in [(2521, 23), (196561, 27)]:
    print(f"=== n = {n} (paper claims rung {claimed_r} via full M^2) ===")
    direct = true_m2_rung(n, claimed_r)
    print(f"  direct check at claimed rung {claimed_r}: {direct}")
    if not direct:
        found = search_true_m2(n, 200)
        print(f"  first true-M^2 closing rung found (search to 200): {found}")

from fractions import Fraction

def full_solution(n, r, u, M):
    v = M*M // u
    b = (M+u)//r
    c = (M+v)//r
    a = (n+r)//4
    return a, b, c

print()
print("=== full identity check ===")
for n, r, u, M in [(2521, 23, 3031545357, 1603356), (196561, 27, 59177, 9660383467)]:
    a, b, c = full_solution(n, r, u, M)
    lhs = Fraction(4, n)
    rhs = Fraction(1,a) + Fraction(1,b) + Fraction(1,c)
    print(f"n={n}: a={a} b={b} c={c}   4/n == 1/a+1/b+1/c : {lhs == rhs}")
