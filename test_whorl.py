#!/usr/bin/env python3
"""Reproducible Gödel / inclosure / frame-mediation hypothesis tests.
Standard library only. Run from the G-mOMonadOS repository root.
Pure-math tests are independent; CLI integration requires `cargo` and the godel binary.
"""
import argparse, csv, json, math, random, re, shutil, statistics, subprocess, time
from pathlib import Path
from collections import Counter

TAU=2*math.pi
MATRICES={'M16':((-47,256),(-9,49)), 'M13':((-35,144),(-9,37))} # M13 overridden by supplied atlas if needed

def bits(n):
    if n<0: raise ValueError('natural required')
    return [(n>>i)&1 for i in range(n.bit_length())]

def braid(a,b):
    return sum(((a>>i)&1)<<(2*i) | ((b>>i)&1)<<(2*i+1) for i in range(max(a.bit_length(),b.bit_length())))

def unbraid(n):
    a=b=0
    for i in range((n.bit_length()+1)//2):
        a |= ((n>>(2*i))&1)<<i
        b |= ((n>>(2*i+1))&1)<<i
    return a,b

def horn(R,r,a,b):
    ph,th=TAU*a,TAU*b
    t=R+r*math.cos(th)
    return (t*math.cos(ph),t*math.sin(ph),r*math.sin(th))

def normal(a,b):
    ph,th=TAU*a,TAU*b
    return (math.cos(th)*math.cos(ph),math.cos(th)*math.sin(ph),math.sin(th))

def centreline_read(R,r,p,n):
    if R<=0 or r<=0: return None
    cx,cy=p[0]-r*n[0],p[1]-r*n[1]
    a=(math.atan2(cy,cx)/TAU)%1
    co=n[0]*math.cos(TAU*a)+n[1]*math.sin(TAU*a)
    b=(math.atan2(n[2],co)/TAU)%1
    return a,b

def phase_err(a,b): return abs(((a-b+.5)%1)-.5)
def delta(a,b): return max(abs(x-y) for x,y in zip(a,b))
def factor_trial(n,limit=100000):
    if n%2==0:return 2
    for d in range(3,min(math.isqrt(n),limit)+1,2):
        if n%d==0:return d
    return None

def miller_rabin_64(n):
    if n<2:return False
    for p in [2,3,5,7,11,13,17,19,23,29,31,37]:
        if n%p==0:return n==p
    s=0; d=n-1
    while d%2==0:s+=1;d//=2
    for a in [2,325,9375,28178,450775,9780504,1795265022]:
        a%=n
        if a==0:continue
        x=pow(a,d,n)
        if x in (1,n-1):continue
        for _ in range(s-1):
            x=x*x%n
            if x==n-1:break
        else:return False
    return True

def pollard(n,seed=0,iterations=200000):
    if n%2==0:return 2
    rng=random.Random(seed)
    for _ in range(12):
        x=rng.randrange(2,n-1);y=x;c=rng.randrange(1,n-1)
        for _ in range(iterations//12):
            x=(x*x+c)%n;y=(y*y+c)%n;y=(y*y+c)%n
            d=math.gcd(abs(x-y),n)
            if 1<d<n:return d
            if d==n:break
    return None

def run_cli(repo,action,n,timeout):
    binary=repo/'target'/'debug'/'godel'
    if binary.exists():cmd=[str(binary),action,str(n)]
    else:cmd=['cargo','run','--quiet','--features','hosted','--bin','godel','--',action,str(n)]
    start=time.perf_counter()
    try:
        p=subprocess.run(cmd,cwd=repo,encoding='utf8',errors='replace',stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=timeout)
        return {'status':'ok' if p.returncode==0 else 'error','elapsed_s':time.perf_counter()-start,'returncode':p.returncode,'stdout':p.stdout,'stderr':p.stderr[-1500:]}
    except (subprocess.TimeoutExpired,OSError) as e:
        return {'status':'timeout_or_unavailable','elapsed_s':time.perf_counter()-start,'error':str(e)}

def field(s,k):
    m=re.search(r'^'+re.escape(k)+r'\s+(.+?)\s*$',s,re.M)
    return m.group(1) if m else None

def record(rows,group,test,passed,detail,**kwargs):
    rows.append({'group':group,'test':test,'status':'PASS' if passed is True else 'FAIL' if passed is False else 'UNRESOLVED','detail':detail,**kwargs})

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--repo',type=Path,help='G-mOMonadOS root containing Cargo.toml')
    ap.add_argument('--numbers',nargs='+',type=int,default=[91, 1000009, 777761462097757])
    ap.add_argument('--samples',type=int,default=120)
    ap.add_argument('--timeout',type=int,default=40)
    ap.add_argument('--out',type=Path,default=Path('whorl_results'))
    ap.add_argument('--skip-cli',action='store_true')
    args=ap.parse_args();args.out.mkdir(parents=True,exist_ok=True)
    rows=[];rng=random.Random(271828)
    # A. Exact braid round-trips and canonical source identity.
    for n in args.numbers+[0,1,2,3,255,256]+[rng.getrandbits(rng.randint(1,128)) for _ in range(args.samples)]:
        a,b=unbraid(n)
        record(rows,'encoding','unbraid_braid',braid(a,b)==n,f'n={n}; Γ={a}; Λ={b}')
        record(rows,'encoding','braid_unbraid',unbraid(braid(a,b))==(a,b),f'Γ={a}; Λ={b}')
    # B. Geometry: exact parameter carrier, distinct vs collapsed fibres, centerline recovery.
    for ratio in [.2,.5,.85,.999999,1.,1.000001,1.5,2.5]:
        R=ratio;r=1.
        for i in range(args.samples):
            a=rng.random();b=rng.random()
            p=horn(R,r,a,b);nor=normal(a,b)
            ab=centreline_read(R,r,p,nor)
            tolerance=3e-8 if ratio<1e-5 else 1e-9
            ok=ab is not None and phase_err(a,ab[0])<tolerance and phase_err(b,ab[1])<tolerance
            record(rows,'geometry','augmented_centerline_readback',ok,f'R/r={ratio}; a={a:.9f}; b={b:.9f}')
        # Detect spatial collapse at horn throat; spindle poles when applicable.
        if ratio==1:
            points=[horn(R,r,a,.5) for a in (0,.13,.41,.77)]
            record(rows,'geometry','horn_throat_collapse',max(delta(points[0],p) for p in points)<1e-10,'Distinct azimuths coincide spatially at b=1/2')
        if ratio<1:
            theta=math.acos(-ratio)/TAU
            points=[horn(R,r,a,theta) for a in (0,.13,.41,.77)]
            record(rows,'geometry','spindle_pole_collapse',max(delta(points[0],p) for p in points)<1e-10,f'R/r={ratio}; upper pole')
    # C. Integer SL(2,Z) winding and inverse on carried lifts, including singular angular addresses.
    matrices={'M16':((-47,256),(-9,49))}
    for name,((aa,ab),(ba,bb)) in matrices.items():
        det=aa*bb-ab*ba
        record(rows,'winding','unimodular_determinant',abs(det)==1,f'{name}; determinant={det}')
        for i in range(args.samples):
            x,y=(rng.randint(-100,100),rng.randint(-100,100)) if i%3==0 else (rng.random(),.5 if i%3==1 else rng.random())
            u=aa*x+ab*y;v=ba*x+bb*y
            recovered=((bb*u-ab*v)/det,(-ba*u+aa*v)/det)
            record(rows,'winding','lift_inverse',max(abs(x-recovered[0]),abs(y-recovered[1]))<1e-10,f'{name}; x={x}; y={y}')
    # C2. Open -> horn -> spindle -> spherical limit: carrier action is exact,
    # while spatial evaluation may be non-injective. Compare across radii.
    # M16 is from the supplied atlas; phase winding uses full angle lifts.
    (ma,mb),(mc,md)=matrices['M16']
    for _ in range(args.samples):
        a,b=rng.random(),rng.random()
        u,v=ma*a+mb*b,mc*a+md*b
        reference=(u%1,v%1)
        for ratio in (0.,.2,.7,1.,1.4,2.):
            # R=0 gives a spherical *image*, not a bijective sphere chart.
            R=ratio;r=1.
            p=horn(R,r,u,v)
            expected=horn(R,r,*reference)
            record(rows,'frame-cycle','winding_embedding_commutes',delta(p,expected)<1e-10,
                   f'ratio={ratio}; angular lifts retained')
            if R>0:
                read=centreline_read(R,r,p,normal(u,v))
                record(rows,'frame-cycle','inverse_augmented_winding',
                       read is not None and phase_err(read[0],reference[0])<1e-9 and phase_err(read[1],reference[1])<1e-9,
                       f'ratio={ratio}; augmented readback')
    # The R=0 spherical image duplicates parameter addresses:
    for a in (.03,.27,.51):
        b=.16
        p=horn(0,1,a,b)
        q=horn(0,1,a+.5,(.5-b)%1)
        record(rows,'geometry','spherical_image_noninjective',delta(p,q)<1e-10,
               f'R=0; distinct parameters ({a},{b}) and ({a+.5},{(.5-b)%1})')
    # D. The algebraic distinction: mu delta = id, but arbitrary F is not identity.
    for _ in range(args.samples):
        x=rng.getrandbits(40);a,b=unbraid(x)
        transformed=braid(a^1,b)
        record(rows,'operation','nontrivial_reentry',transformed!=x and braid(*unbraid(transformed))==transformed,f'input={x}; transformed={transformed}')
    # E. Inspect actual CLI commands and verify claims independently.
    if args.repo and not args.skip_cli:
        if not (args.repo/'Cargo.toml').is_file():
            raise SystemExit('ERROR: --repo must contain Cargo.toml')
        for n in args.numbers:
            for command in ('analyze','unbraid','frame','shiab','separate'):
                result=run_cli(args.repo,command,n,args.timeout)
                with (args.out/f'{n}_{command}.log').open('w') as f:
                    f.write(result.get('stdout','')+'\n--- STDERR ---\n'+result.get('stderr',result.get('error','')))
                record(rows,'cli',command,result['status']=='ok',f'n={n}; wall={result["elapsed_s"]:.4f}s',elapsed_s=round(result['elapsed_s'],6))
                if result['status']!='ok':continue
                out=result['stdout']
                if command=='unbraid':
                    try:
                        a=int(field(out,'Γ lane'));b=int(field(out,'Λ lane'));returned=int(field(out,'braid.recovered'))
                        ok=(a,b)==unbraid(n) and returned==n and braid(a,b)==n
                        record(rows,'cli-verified','unbraid_exact',ok,f'n={n}; Γ={a}; Λ={b}')
                    except (TypeError,ValueError):record(rows,'cli-verified','unbraid_exact',None,'Missing fields')
                if command=='shiab':
                    try:
                        a=int(field(out,'δ.even-lane'));b=int(field(out,'δ.odd-lane'));returned=int(field(out,'μ(δ(bulk))'))
                        record(rows,'cli-verified','shiab_retraction',braid(a,b)==n==returned,f'n={n}')
                    except (TypeError,ValueError):record(rows,'cli-verified','shiab_retraction',None,'Missing fields')
                if command=='separate':
                    raw=field(out,'prime.factor')
                    if raw and raw.isdigit():
                        p=int(raw);q,rem=divmod(n,p)
                        record(rows,'cli-verified','exact_factor_closure',p>1 and q>1 and rem==0,f'{n}={p}*{q}; remainder={rem}')
                        if n<2**64:record(rows,'cli-verified','factors_probable_prime',miller_rabin_64(p) and miller_rabin_64(q),f'p={p}; q={q}; deterministic 64-bit MR')
                    else: record(rows,'cli-verified','exact_factor_closure',None,f'n={n}; no factor returned')
    else:record(rows,'cli','integration',None,'Skipped: pass --repo PATH to run the actual Rust CLI')
    # F. Comparable independent baseline, bounded to avoid runaway trials.
    for n in args.numbers:
        t=time.perf_counter();p=factor_trial(n,100000);trial_s=time.perf_counter()-t
        record(rows,'baseline','bounded_trial_division',None if p is None else n%p==0,f'n={n}; factor={p}; limit=100000',elapsed_s=round(trial_s,6))
        t=time.perf_counter();p=pollard(n,seed=271828);rho_s=time.perf_counter()-t
        record(rows,'baseline','pollard_rho',None if p is None else 1<p<n and n%p==0,f'n={n}; factor={p}',elapsed_s=round(rho_s,6))
    with (args.out/'results.json').open('w') as f:json.dump({'seed':271828,'numbers':args.numbers,'rows':rows},f,indent=2)
    with (args.out/'results.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=['group','test','status','detail','elapsed_s'],extrasaction='ignore');w.writeheader();w.writerows(rows)
    stats=Counter(r['status'] for r in rows)
    print('Experiment results:',dict(stats),'output:',args.out.resolve())
    print('NOTE: No geometry-driven factoring effect is claimed by these tests. To establish one, implement a frame-dependent F and benchmark it against matched controls.')
    return 1 if stats['FAIL'] else 0
if __name__=='__main__':raise SystemExit(main())
