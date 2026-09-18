from sympy import isprime

def codebook_survivors(N, m):
    mod = 1 << m
    lo, hi = 1 << (m-1), 1 << m          # m-bit range
    primes = [p for p in range(lo|1, hi, 2) if isprime(p)]
    pset = set(primes)
    Nm = N % mod
    survivors = []
    truth = []
    for P in primes:
        invP = pow(P, -1, mod)
        Q = (Nm * invP) % mod
        if Q in pset:                     # forced complement decodes to an m-bit prime
            survivors.append((P, Q))
            if P * Q == N:
                truth.append((P, Q))
    return len(primes), len(survivors), truth

for (N, m, want_states, want_surv) in [
    (62615533, 13, 464, 64),
    (999985999949, 20, 38635, 2830),
]:
    nprimes, nsurv, truth = codebook_survivors(N, m)
    print(f"N={N} m={m}: {nprimes} m-bit primes (want ~{want_states}), "
          f"{nsurv} prime-compatible states (want {want_surv}), true pairs found: {truth}")
