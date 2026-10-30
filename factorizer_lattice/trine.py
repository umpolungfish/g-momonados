import subprocess, re, json
from sympy import factorint
RUN="./run_cmds.sh"
def hexword(N):
    s=open(f"factorizer_lattice/raw_{N}.txt").read()
    m=re.search(r"hex-digit word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)",s)
    return m.group(1) if m else None
def quadseq(w):
    out=subprocess.run([RUN,f"cycle {w}"],capture_output=True,text=True,timeout=120).stdout
    return [m[1] for m in re.findall(r"^\s*(\d+)\s+([A-Za-z]+)\s+[TBNF]\s",out,re.M)]
def best_trine(seq):
    # choose offset 0/1/2 minimizing straddled non-T runs
    best=None
    for off in range(3):
        s=seq[off:]
        trines=[s[i:i+3] for i in range(0,len(s)-2,3)]
        straddle=0; sig=[]
        for t in trines:
            nt=[x for x in t if x!='T']
            if not nt: sig.append('.')
            elif len(set(nt))==1 and len(nt)==3: sig.append(nt[0])      # full pure trine
            elif len(set(nt))==1: sig.append(nt[0].lower()); straddle+=1 # partial
            else: sig.append('?'); straddle+=1
        if best is None or straddle<best[0]: best=(straddle,off,sig)
    return best
Ns=[15,21,35,51,77,91,143,155,187,221,247,323,437,667,899,1517,3233,3599,8051,10403]
rows=[]
for N in Ns:
    w=hexword(N)
    if not w: continue
    seq=quadseq(w); st,off,sig=best_trine(seq)
    events=[x for x in sig if x not in '.']
    f=sorted(factorint(N))
    row={"N":N,"off":off,"straddle":st,"events":events,"nev":len(events),"facs":f,"len":len(seq)}
    rows.append(row); print(row, flush=True)
json.dump(rows,open("factorizer_lattice/lattice_trine.json","w"),indent=1,default=str)
