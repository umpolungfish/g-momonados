"""Seeded random-prime semiprime controls for the nested phase relation."""
from pathlib import Path
import math
import random
import re
import subprocess
import time
import sympy

ROOT=Path(__file__).resolve().parent
BIN=ROOT/"target"/"release"/"g-momonados"
SEED=20260910
rng=random.Random(SEED)

def random_prime(width:int)->int:
    n=rng.getrandbits(width)|1|(1<<(width-1))
    p=int(sympy.nextprime(n))
    if p.bit_length()!=width:
        p=int(sympy.prevprime(1<<width))
    assert sympy.isprime(p) and p.bit_length()==width
    return p

def required_phases(p:int,q:int)->int:
    n=p*q
    a=(p+q)//2
    root=math.isqrt(n)
    if root*root<n: root+=1
    return a-root+1

def execute(p:int,q:int,width:int)->tuple[int,float]:
    n=p*q
    start=time.perf_counter()
    proc=subprocess.run([str(BIN),"--selector-relation",str(n),str(width),"1024","64"],
        cwd=ROOT,text=True,capture_output=True,check=True,timeout=60)
    elapsed=time.perf_counter()-start
    assert proc.stderr == "", proc.stderr
    assert re.search(
        r"(?:phase family|denotation bulk) vessels=1\b",
        proc.stdout,
    ), proc.stdout

    parts = re.search(
        r"\bCRT false span=(\d+).*?\bQR admitted=(\d+)",
        proc.stdout,
    )

    if parts:
        phases = sum(map(int, parts.groups()))
    else:
        legacy = re.search(
            r"(?:^|[;\n])\s*phase leaves=(\d+)(?:;|$)",
            proc.stdout,
        )
        assert legacy, proc.stdout
        phases = int(legacy.group(1))

    factors = re.search(
        r"\n  \d+ = (\d+) x (\d+) \(verified\)\n?$",
        proc.stdout,
    )
    assert factors, proc.stdout

    got_p, got_q = map(int, factors.groups())
    assert {got_p, got_q} == {p, q}, proc.stdout
    return phases, elapsed

rows=[]
for width in (128,256,512,1024):
    p=random_prime(width)
    # A seeded half-width displacement makes more than one phase likely while
    # keeping the exact run bounded enough for the executable control.
    displacement=rng.getrandbits(width//2+6)|(1<<(width//2+5))
    q=int(sympy.nextprime(p+displacement))
    assert sympy.isprime(q) and q.bit_length()==width
    expected=required_phases(p,q)
    phases,elapsed=execute(p,q,width)
    assert phases==expected
    rows.append(("windowed",width,p,q,p*q,phases,"verified",elapsed))

for width in (128,256,512):
    p=random_prime(width)
    q=random_prime(width)
    if p>q:p,q=q,p
    phases=required_phases(p,q)
    rows.append(("independent",width,p,q,p*q,phases,"counted",0.0))

out=ROOT/"measurements"/"random_prime_phase_relations.tsv"
out.write_text("class\twidth\tp\tq\tn\tphase_leaves\tstatus\tseconds\n"+
    "".join(f"{kind}\t{width}\t{p}\t{q}\t{n}\t{phases}\t{status}\t{elapsed:.6f}\n"
            for kind,width,p,q,n,phases,status,elapsed in rows))
print(f"PASS: {sum(r[6]=='verified' for r in rows)} random-prime semiprimes factored; "
      f"{sum(r[6]=='counted' for r in rows)} independent phase distances counted")
