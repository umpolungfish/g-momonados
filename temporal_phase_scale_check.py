"""Scale the production phase relation across width and made phase time."""
from pathlib import Path
import re
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parent
BIN=ROOT/"target"/"release"/"g-momonados"
DEPTH=64
sys.set_int_max_str_digits(0)

def run(n:int,width:int,expected_p:int,expected_q:int)->tuple[int,float,int]:
    start=time.perf_counter()
    decimal=str(n)
    stdin_mode=len(decimal)>100000
    args=[str(BIN),"--selector-relation-stdin" if stdin_mode else "--selector-relation"]
    if not stdin_mode: args.append(decimal)
    args.extend([str(width),"1024",str(DEPTH)])
    proc=subprocess.run(args,input=decimal if stdin_mode else None,
        cwd=ROOT,text=True,capture_output=True,check=True,timeout=60)
    elapsed=time.perf_counter()-start
    assert proc.stderr=="",proc.stderr
    assert "phase family vessels=1" in proc.stdout,proc.stdout
    match=re.search(r"phase leaves=(\d+); nested mark ticks=(\d+)",proc.stdout)
    assert match,proc.stdout
    phases,ticks=map(int,match.groups())
    factors=re.search(r"\n  (\d+) = (\d+) x (\d+) \(verified\)\n?$",proc.stdout)
    assert factors,proc.stdout
    nn,p,q=map(int,factors.groups())
    assert nn==n and p*q==n and {p,q}=={expected_p,expected_q},proc.stdout
    assert ticks>=phases*14*14*DEPTH,(phases,ticks)
    return phases,elapsed,ticks

rows=[]
for width in (128,256,512,1024,2048,4096,8192,16384,32768,65536,131072,
              262144,524288,1048576,2097152,4194304):
    p=(1<<(width-1))+11
    q=p+46
    phases,elapsed,ticks=run(p*q,width,p,q)
    assert phases==1
    rows.append((width,(p*q).bit_length(),phases,ticks,elapsed))

# Force substantial made time without changing the bounded-state shape.
width=256
a=3<<(width-2)
b=(1<<136)+1
p=a-b
q=a+b
phases,elapsed,ticks=run(p*q,width,p,q)
assert phases>1000,phases
rows.append((width,(p*q).bit_length(),phases,ticks,elapsed))

report=ROOT/"measurements"/"factor_relation_phase_scale.log"
report.write_text("width product_bits phase_leaves nested_marks seconds\n"+
    "".join(f"{w} {bits} {ph} {ticks} {secs:.6f}\n" for w,bits,ph,ticks,secs in rows))
print(f"PASS: one-leaf closure through 4194304-bit factors; made-time control={phases} leaves")
