import subprocess,re,json
from sympy import factorint
def hexword(N):
    s=open(f"G-mOMonadOS/factorizer_lattice/raw_{N}.txt").read()
    m=re.search(r"hex-digit word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)",s)
    return m.group(1) if m else None
def voxpairs(w):
    out=subprocess.run(["./Vox/vox","pairs",w],capture_output=True,text=True,timeout=60).stdout
    rows=re.findall(r"^\s*(\d+)\s+(\d+)\s+(\d+)\s+(yes|no)\s",out,re.M)
    letters=re.search(r"letters\s+(\d+)",out)
    paired=re.search(r"(\d+) paired",out)
    verd=re.search(r"verdict\s+([TBNF])",out)
    spans=[int(r[2]) for r in rows]
    return {"letters":int(letters.group(1)) if letters else None,
            "n_regions":len(rows),"paired":int(paired.group(1)) if paired else None,
            "spans":spans,"verdict":verd.group(1) if verd else None}
Ns=[15,21,35,51,77,91,143,155,187,221,247,323,437,667,899,1517,3233,3599,8051,10403]
rows=[]
for N in Ns:
    w=hexword(N)
    if not w: continue
    d=voxpairs(w); f=sorted(factorint(N))
    row={"N":N,**d,"facs":f,"hexdigits":len(hex(N))-2}
    rows.append(row); print(row,flush=True)
json.dump(rows,open("G-mOMonadOS/factorizer_lattice/lattice_vox.json","w"),indent=1,default=str)
