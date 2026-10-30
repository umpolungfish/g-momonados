def curve(N, m):
    mod=1<<m; lo=1<<(m-1); Nm=N%mod
    base=[]
    for P in range(lo|1, mod, 2):
        Q=(Nm*pow(P,-1,mod))%mod
        if Q>=lo: base.append((P,Q))
    out={}
    for r in (0,1,4,8,12,16,20):
        need=1<<(m+r); out[r]=sum(1 for (P,Q) in base if (P*Q-N)%need==0)
    return len(base), out
for name,N,m in [("near-square",4398205895659,22),("balanced far",7751246492807,22)]:
    tot,out=curve(N,m)
    print(f"{name:13s} base={tot}  " + " ".join(f"r={r}:{out[r]}" for r in (0,1,4,8,12,16,20)))
