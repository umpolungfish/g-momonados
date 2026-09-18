import random
# ROBDD node count of a boolean function given as a truth table (list of 0/1),
# length 2^n, index bit j (from MSB) = variable j. Low-to-high order means
# variable 0 = lowest phase bit; we build k's bits in that order.
def robdd_size(tt):
    from functools import lru_cache
    memo={}
    def reduce(t):
        if all(v==t[0] for v in t): return ('T',t[0])
        key=t
        if key in memo: return memo[key]
        h=t[:len(t)//2]; l=t[len(t)//2:]
        rh=reduce(h); rl=reduce(l)
        if rh==rl: node=rh
        else: node=('N', rh, rl)
        memo[key]=node; return node
    root=reduce(tuple(tt))
    seen=set()
    def walk(nd):
        if nd[0]=='T' or nd in seen: return
        seen.add(nd); walk(nd[1]); walk(nd[2])
    walk(root)
    return len(seen)

def closure_tt(N, m, r):
    mod=1<<m; n=m-3; orbit=1<<n
    need=1<<(m+r)
    # phase k in [0,orbit); P=3*9^k mod 2^m; Q=N*P^-1 mod 2^m; predicate PQ==N mod 2^{m+r}
    tt=[0]*orbit
    Nm=N%mod
    P=3%mod
    # index by k with low bit = variable 0: truth table index MSB=var0 means we must
    # place k so that bit0 of k is the most significant tt index. Build by k then remap.
    vals=[0]*orbit
    for k in range(orbit):
        Q=(Nm*pow(P,-1,mod))%mod
        vals[k]=1 if ((P*Q-N)%need==0) else 0
        P=(P*9)%mod
    # remap: tt index i has var0 (bit0 of k) as MSB. i = sum bit_j(k)*2^{n-1-j}
    tt=[0]*orbit
    for k in range(orbit):
        i=0
        for j in range(n):
            if (k>>j)&1: i|=1<<(n-1-j)
        tt[i]=vals[k]
    return tt

for m in (13,14,15,16,17,18):
    tt=closure_tt(0x3B9ACA07*0x3B9ACA09 if m>=30 else (2097169*2097211), m, 2)
    dens=sum(tt)/len(tt)
    real=robdd_size(tt)
    rnd=[1 if random.random()<dens else 0 for _ in tt]
    rr=robdd_size(rnd)
    print(f"m={m} n={m-3} density={dens:.3f}  closure BDD={real}  random BDD={rr}  ratio={real/rr:.2f}")
