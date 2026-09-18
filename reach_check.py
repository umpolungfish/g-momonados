# Verify: toggling phase bit i of k changes P only in bits >= i+3; lower bits of
# P are invariant under that toggle. This is the LTE reach window that should
# explain why testing low k-bits first (high2low order) wins.
def check(m, trials=2000):
    mod = 1 << m
    import random
    n = m - 3
    ok = True
    worst = None
    for _ in range(trials):
        k = random.randrange(1 << n)
        i = random.randrange(n)
        P0 = (3 * pow(9, k, mod)) % mod
        P1 = (3 * pow(9, k ^ (1 << i), mod)) % mod
        low_mask = (1 << (i + 3)) - 1
        if (P0 & low_mask) != (P1 & low_mask):
            ok = False
            worst = (k, i, P0 & low_mask, P1 & low_mask)
            break
    return ok, worst

for m in (13, 20, 30):
    ok, worst = check(m)
    print(f"m={m}: reach-window invariance holds = {ok}" + (f"  counterexample {worst}" if not ok else ""))
