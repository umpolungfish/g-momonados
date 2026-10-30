# Verify: odd residues mod 2^m = s(X) * 9^k uniquely; and phase-walk finds a factor.
def order_of(g, m):
    mod = 1 << m
    x = g % mod; o = 1
    while x != 1:
        x = (x*g) % mod; o += 1
        if o > mod: return None
    return o

for m in (5, 13, 20, 30):
    mod = 1 << m
    print(f"m={m}: |group|=2^{m-1}={1<<(m-1)}, ord(9)={order_of(9,m)} (expect 2^{m-3}={1<<(m-3)})")

# representation: reps s(X) in {1,3,-1,-3}; check every odd residue hit uniquely
def rep_table(m):
    mod = 1 << m
    reps = [1 % mod, 3 % mod, (-1) % mod, (-3) % mod]
    seen = {}
    ordr = order_of(9, m)
    for xi, s in enumerate(reps):
        v = s
        for k in range(ordr):
            seen.setdefault(v, (xi,k))
            v = (v*9) % mod
    odds = [u for u in range(mod) if u & 1]
    return len(seen), len(odds)

for m in (5,13,20):
    got, tot = rep_table(m)
    print(f"m={m}: representation covers {got}/{tot} odd residues  {'UNIQUE+COMPLETE' if got==tot else 'INCOMPLETE'}")

# phase-walk factor of 62615533 = 7907*7919 (both ~13-bit)
N = 62615533
m = 13
mod = 1 << m
reps = [1%mod, 3%mod, (-1)%mod, (-3)%mod]
ordr = order_of(9, m)
inv9 = pow(9, -1, mod)
found = None
steps = 0
import math
for xi, s in enumerate(reps):
    P = s
    for k in range(ordr):
        steps += 1
        # P is a residue mod 2^13; treat as candidate odd factor value P (must be < 2^13 and divide N)
        if 1 < P < N and N % P == 0:
            found = (P, N//P, xi, k, steps); break
        P = (P*9) % mod
    if found: break
print("phase-walk factor:", found, " (P residue divides N exactly)")
