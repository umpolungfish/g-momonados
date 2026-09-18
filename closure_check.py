# Verify the two exact closure laws along a phase orbit.
#   P' = 9P - a*2^m,   9Q' = Q + d*2^m
#   claim 1: h'_0 = h_0 XOR a_0 XOR d_0     (first overflow bit is a parity walk)
#   claim 2: h mod 4 is a FOUR register, H' = rho^{g}(H), g = dP - aQ' (mod 4)
def run(N, m, steps=200000):
    mod = 1 << m
    inv9 = pow(9, -1, mod)
    Nm = N % mod
    # seed a phase orbit: P over its 9-multiplication orbit in a coset
    P = 3 % mod
    Q = (Nm * pow(P, -1, mod)) % mod
    ok1 = True; ok2 = True
    checked = 0
    for _ in range(steps):
        h = (P*Q - N) // mod            # closure height (integer; PQ ≡ N mod 2^m holds so divisible)
        # next step
        P2 = (9*P) % mod
        a  = (9*P - P2) // mod          # wrap digit for P, 0..8
        Q2 = (Q * inv9) % mod
        d  = (9*Q2 - Q) // mod          # since 9*Q2 = Q + d*2^m
        h2 = (P2*Q2 - N) // mod
        # claim 1
        if (h2 & 1) != ((h & 1) ^ (a & 1) ^ (d & 1)):
            ok1 = False
        # claim 2: rho = +1 on Z/4 under gray 0->N,1->T,2->B,3->F; g = dP - aQ'
        g = (d*P - a*Q2) % 4
        if ((h2 & 3) != ((h & 3) + g) % 4):
            ok2 = False
        P, Q = P2, Q2
        checked += 1
        if not (ok1 or ok2): break
    return checked, ok1, ok2

for (N,m) in [(62615533,13),(999985999949,20),(1000000016000000063,30)]:
    c, ok1, ok2 = run(N,m)
    print(f"N={N} m={m}: {c} steps  claim1(parity h0=h0^a0^d0)={ok1}  claim2(h mod4 = rho^g)={ok2}")

# Full recurrence (not just mod 4):  h' = h + (d*P - a*Q')
def run_full(N, m, steps=200000):
    mod = 1 << m
    inv9 = pow(9, -1, mod)
    Nm = N % mod
    P = 3 % mod
    Q = (Nm * pow(P, -1, mod)) % mod
    h = (P*Q - N) // mod
    ok = True
    for _ in range(steps):
        P2 = (9*P) % mod; a = (9*P - P2)//mod
        Q2 = (Q*inv9) % mod; d = (9*Q2 - Q)//mod
        h_pred = h + (d*P - a*Q2)
        h_true = (P2*Q2 - N)//mod
        if h_pred != h_true: ok = False; break
        h = h_true; P, Q = P2, Q2
    return ok
for (N,m) in [(62615533,13),(999985999949,20),(1000000016000000063,30)]:
    print(f"N={N} m={m}: full recurrence h'=h+(dP-aQ') exact = {run_full(N,m)}")
