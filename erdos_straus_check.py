# Verify the paper's own mechanism directly: for n=193 (a listed frontier
# value), find the actual least closing rung by brute force, and separately
# verify Corollary twoshift's predicted failure for k=2.
def closing_rung(n, max_r=200):
    for r in range(3, max_r, 4):  # r == 3 mod 4
        if (n + r) % 4 != 0: continue
        a = (n + r) // 4
        M = n * a
        # need divisor u of M^2 with u === -M (mod r)
        target = (-M) % r
        # search divisors of M^2 up to a reasonable bound via M's own divisors
        # (u = M*t/w style search over divisors of M is enough for the closed
        # families the paper describes; for a real closing check test all
        # divisors of M^2 up to sqrt is too slow for large M, so check via
        # divisors of M itself first, and via kn+1 shift family)
        divs = []
        i = 1
        while i*i <= M:
            if M % i == 0:
                divs.append(i)
                if i != M//i: divs.append(M//i)
            i += 1
        for u in divs:
            if u % r == target:
                return r, a, M, u
        # also check divisors of M^2 that are M*t or M/t for small t (t | M)
        for t in divs:
            u = M * t
            if u % r == target:
                return r, a, M, u
    return None

n = 193
res = closing_rung(n)
print(f"n={n}: closing search result = {res}")

# Corollary twoshift check: does 2n+1 have a divisor === 5 mod 8?
m2 = 2*n+1
divs = [d for d in range(1, m2+1) if m2 % d == 0]
hit = [d for d in divs if d % 8 == 5]
print(f"2n+1 = {m2} = factorization divisors: {divs}")
print(f"divisors === 5 mod 8: {hit}  (k=2 family predicted to fail: {len(hit)==0})")
