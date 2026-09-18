# LTE claim: with k = a + 2^j b and L = j+3, once 2L >= m,
#   9^{a + 2^j b} == 9^a * (1 + b*2^L*c) (mod 2^m),  c odd,  9^{2^j}=1+2^L c.
# So P(b) is affine in b: P(b) = P0 + b*stride (mod 2^m).
def check_affine(m):
    mod = 1 << m
    import math
    j = (m + 1)//2 - 3
    if j < 0: j = 0
    L = j + 3
    step = pow(9, 1 << j, mod)          # 9^{2^j}
    assert (step - 1) % (1 << L) == 0, "9^{2^j} != 1 mod 2^L"
    c = (step - 1) >> L
    twoLc = ((1 << L) * (c % mod)) % mod
    ok = True
    orbit = 1 << (m - 3)                # order of 9
    bmax = orbit >> j if (orbit >> j) > 0 else 1
    for a in range(0, min(1<<j, 64)):   # sample low parts
        base = pow(9, a, mod)
        stride = (base * twoLc) % mod
        P0 = base
        for b in range(0, min(bmax, 512)):
            direct = pow(9, (a + (b << j)) % orbit, mod)
            affine = (P0 + b*stride) % mod
            if direct != affine: ok = False; break
        if not ok: break
    return j, L, ok, (2*L >= m)

for m in (13, 20, 30, 37):
    j, L, ok, cond = check_affine(m)
    print(f"m={m}: j={j} L={L}  2L>=m: {cond}  affine identity holds: {ok}")

# Does the affine (arithmetic-progression) order make leading-bit-valid states contiguous?
def run_stats(seq):
    runs=[]; cur=0
    for v in seq:
        if v: cur+=1
        elif cur: runs.append(cur); cur=0
    if cur: runs.append(cur)
    return (len(runs), (sum(runs)/len(runs) if runs else 0), (max(runs) if runs else 0))

def compare(m):
    mod=1<<m; lead=1<<(m-1); orbit=1<<(m-3)
    j=(m+1)//2-3; L=j+3
    step=pow(9,1<<j,mod)
    # multiply-by-9 order within one coset (rep=3)
    P=3%mod; mul_order=[]
    for k in range(orbit):
        mul_order.append(1 if (P&lead) else 0); P=(P*9)%mod
    # affine order: fix a, sweep b; concatenate blocks
    aff_order=[]
    bmax=orbit>>j
    for a in range(1<<j):
        base=(3*pow(9,a,mod))%mod
        stride=(base*(step-1))%mod   # = base*2^L*c
        P=base
        for b in range(bmax):
            aff_order.append(1 if (P&lead) else 0); P=(P+stride)%mod
    return run_stats(mul_order), run_stats(aff_order)

for m in (20, 24):
    (mr,ma,mx),(ar,aa,axx)=compare(m)
    print(f"m={m}: mul-order runs={mr} avg={ma:.2f} max={mx}   affine-order runs={ar} avg={aa:.2f} max={axx}")

# Nesting order: sweep the HIGH parameter b outer, low a inner.
# P(a,b) = s(X)*9^a * (1 + b*2^L*c). High bits ride b; fixing b, inner a stays
# in the same leading-bit state. Expect long contiguous runs = whole inner block.
def nested_order_stats(m):
    mod=1<<m; lead=1<<(m-1); orbit=1<<(m-3)
    j=(m+1)//2-3; L=j+3
    step=pow(9,1<<j,mod)
    bmax=orbit>>j
    seq=[]
    for b in range(bmax):                 # HIGH outer
        for a in range(1<<j):             # LOW inner
            P=(3*pow(9,a,mod)*(1 + b*(step-1)))%mod
            seq.append(1 if (P&lead) else 0)
    runs=[]; cur=0
    for v in seq:
        if v: cur+=1
        elif cur: runs.append(cur); cur=0
    if cur: runs.append(cur)
    total=sum(seq)
    return (1<<j), bmax, len(runs), (sum(runs)/len(runs) if runs else 0), max(runs) if runs else 0, total
for m in (20,24):
    inner,bmax,nr,avg,mx,tot=nested_order_stats(m)
    print(f"m={m}: inner block={inner} outer={bmax}  valid={tot}  runs={nr} avg={avg:.1f} max={mx}")
