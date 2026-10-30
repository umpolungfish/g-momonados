# Hensel sieve: requiring first r overflow bits of h=(PQ-N)/2^m to vanish
# is PQ == N (mod 2^{m+r}). Count survivors per r over the phase space; expect halving.
def hensel_curve(N, m):
    mod = 1 << m
    lo = 1 << (m-1)
    Nm = N % mod
    # phase states = odd m-bit P with forced Q an m-bit integer
    base = []
    for P in range(lo|1, mod, 2):
        invP = pow(P, -1, mod)
        Q = (Nm * invP) % mod
        if Q >= lo:                      # both leading bits set
            base.append((P, Q))
    counts = {}
    for r in (0,1,4,8,12,16,20):
        need = 1 << (m + r)
        c = 0
        for (P,Q) in base:
            if (P*Q - N) % need == 0:
                c += 1
        counts[r] = c
    return len(base), counts

for (N, m) in [(62615533,13), (999985999949,20)]:
    total, counts = hensel_curve(N, m)
    print(f"N={N} m={m}: {total} leading-bit phase states")
    prev = total
    for r in (0,1,4,8,12,16,20):
        c = counts[r]
        frac = c/total if total else 0
        print(f"   r={r:2d}: {c:8d} survive   ({100*frac:8.4f}% of states)")
