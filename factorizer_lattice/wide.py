import subprocess, re, json
from sympy import factorint
from math import gcd
RUN="./run_cmds.sh"
def hexword(N):
    s=open(f"factorizer_lattice/raw_{N}.txt").read()
    m=re.search(r"hex-digit word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)",s); return m.group(1) if m else None
def event_positions(w):
    out=subprocess.run([RUN,f"cycle {w}"],capture_output=True,text=True,timeout=120).stdout
    seq=[(int(m[0]),m[1]) for m in re.findall(r"^\s*(\d+)\s+([A-Za-z]+)\s+[TBNF]\s",out,re.M)]
    pos={}
    for k,st in seq:
        if st!='T': pos.setdefault(st,[]).append(k)
    return len(seq),pos
Ns=[143,221,3233,667,899,10403,1517,187,3599,155,8051,247]
rows=[]
for N in Ns:
    w=hexword(N); L,pos=event_positions(w); f=sorted(factorint(N))
    row={"N":N,"orbit":L,"event_pos":pos,"facs":f}
    # gcd probes using positions
    allpos=[p for lst in pos.values() for p in lst]
    probes={}
    if allpos:
        for p in allpos:
            for val in (p, p+1, L-p, p*p % N if N else 0):
                g=gcd(val,N)
                if 1<g<N: probes[f'gcd({val})']=g
    row["gcd_hits"]=probes
    rows.append(row); print(row, flush=True)
json.dump(rows,open("factorizer_lattice/lattice_wide.json","w"),indent=1,default=str)
