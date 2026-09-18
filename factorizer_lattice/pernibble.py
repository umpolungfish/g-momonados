import subprocess, re, json
from sympy import factorint
RUN="./run_cmds.sh"
def hexword(N):
    s=open(f"factorizer_lattice/raw_{N}.txt").read()
    m=re.search(r"hex-digit word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)",s)
    return m.group(1) if m else None
# split the hex-digit word into its per-nibble blocks (each ⊢...⊣)
def blocks(w):
    return ["⊢"+b for b in w.split("⊢") if b]
def land(block):
    # register landing of one nibble block via cycle: take cut-0 final register
    out=subprocess.run([RUN,f"cycle {block}"],capture_output=True,text=True,timeout=60).stdout
    m=re.search(r"^\s*0\s+([A-Za-z]+)\s+[TBNF]\s",out,re.M)
    return m.group(1) if m else None
Ns=[143,221,3233,667,899,10403,1517,8051,35,323]
rows=[]
for N in Ns:
    w=hexword(N)
    bs=blocks(w)
    seq=[land(b) for b in bs]
    f=sorted(factorint(N))
    row={"N":N,"hex":hex(N),"nblocks":len(bs),"per_nibble":seq,"facs":f}
    rows.append(row); print(row, flush=True)
json.dump(rows,open("factorizer_lattice/lattice_pernibble.json","w"),indent=1,default=str)
