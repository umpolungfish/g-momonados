import random, sys
# Efficient bottom-up ROBDD node count. tt: list of 0/1 length 2^n, index MSB=var0.
# Eliminate the least-significant index variable first (var n-1), pairing adjacent.
def robdd_size(tt, n):
    ids = tt
    node_ids = {}          # (level, hi, lo) -> id
    next_id = [2]
    def get(level, hi, lo):
        if hi == lo: return hi
        k = (level, hi, lo)
        v = node_ids.get(k)
        if v is None:
            v = next_id[0]; next_id[0] += 1; node_ids[k] = v
        return v
    cur = ids
    for lvl in range(n-1, -1, -1):
        half = len(cur) >> 1
        nxt = [0]*half
        for i in range(half):
            nxt[i] = get(lvl, cur[2*i], cur[2*i+1])
        cur = nxt
    return len(node_ids)

def closure_tt(N, m, r):
    mod=1<<m; n=m-3; orbit=1<<n; need=1<<(m+r)
    inv9=pow(9,-1,mod); Nm=N%mod
    P=3%mod; Q=(Nm*pow(P,-1,mod))%mod
    vals=[0]*orbit
    for k in range(orbit):
        vals[k]=1 if ((P*Q-N)%need==0) else 0
        P=(P*9)%mod; Q=(Q*inv9)%mod
    tt=[0]*orbit
    for k in range(orbit):
        i=0
        for j in range(n):
            if (k>>j)&1: i|=1<<(n-1-j)
        tt[i]=vals[k]
    return tt

from sympy import nextprime
def balanced(m):
    a=nextprime(1<<(m-1)); b=nextprime(a+ (1<<(m-3)))   # both m-bit, moderate gap
    return a*b
print("m  n  density  closureBDD  randomBDD  ratio  growth")
prev=None
for m in range(13, 25):
    N=balanced(m)
    tt=closure_tt(N,m,2)
    dens=sum(tt)/len(tt)
    real=robdd_size(tt,m-3)
    rnd=robdd_size([1 if random.random()<dens else 0 for _ in tt], m-3)
    g = "" if prev is None else f"{real/prev:.3f}"
    print(f"{m:2d} {m-3:2d} {dens:.3f}   {real:8d}   {rnd:8d}   {real/rnd:.3f}   {g}")
    prev=real
    sys.stdout.flush()
