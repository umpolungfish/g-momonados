"""Independent solution-set counts for the symbolic GPU relation selector."""
from pathlib import Path
import os
import random
import re
import subprocess
import sys

ROOT=Path(__file__).resolve().parent
rng=random.Random(20260910)
cases=[(35,3),(49,3),(143,4),(127,4),(225,4)]
for m in range(2,13):
    lo,hi=1<<(m-1),(1<<m)-1
    p=rng.randrange(lo|1,hi+1,2)
    q=rng.randrange(lo|1,hi+1,2)
    cases.extend([(p*q,m),(hi*hi-2,m)])
if len(sys.argv)>1:
    cases=[]
    for m in map(int,sys.argv[1:]):
        assert 3<=m<=30
        n=1000000016000000063 if m==30 else ((3<<(m-2))+1)*((1<<m)-5)
        cases.append((n,m))
expected=[]
for n,m in cases:
    lo,hi=1<<(m-1),(1<<m)-1
    expected.append(2 if n==1000000016000000063 else
                    sum(n%p==0 and lo<=n//p<=hi for p in range(lo|1,hi+1,2)))
commands=[f"gpu_kernel selector_bdd_relation {n} {m}" for n,m in cases]
commands += ["gpu_kernel selector_bdd_relation 10309995 12 1024 3",
             "gpu_kernel selector_bdd_relation 143 4 16", "gpu_kernel selector_bdd_relation 35 3 16",
             "gpu_kernel selector_bdd_relation 143 4 16 3",
             "gpu_kernel run_nested ⊢⊙∈≻⊤≺⊥⋈⊞∋⊡⊣"]
suffix="_"+"_".join(sys.argv[1:]) if len(sys.argv)>1 else ""
log=ROOT/f"measurements/factor_relation_controls{suffix}.log"
with log.open("w") as sink:
    subprocess.run([str(ROOT/"target"/os.environ.get("PROFILE","release")/"g-momonados")],
                   input="\n".join([*commands,"quit",""]),text=True,stdout=sink,
                   stderr=subprocess.STDOUT,cwd=ROOT,check=True,timeout=240)
output=log.read_text()
log.write_text(output)  # Normalize the REPL's CRLF before storing the artifact.
sections=re.split(r"⊙> gpu_kernel selector_bdd_relation \d+ \d+(?: \d+)?(?: \d+)?\s*\n",output)[1:]
assert len(sections)==len(commands)-1,output
for (n,m),count,section in zip(cases,expected,sections):
    assert "INCOMPLETE" not in section,section
    got=re.search(r"relation solutions=(\d+)",section)
    assert got and int(got[1])==count,(n,m,count,section)
    witness=re.search(r"(\d+) = (\d+) x (\d+) \(verified\)",section)
    if count:
        assert witness,section
        nn,p,q=map(int,witness.groups())
        assert nn==n and p*q==n and p.bit_length()==q.bit_length()==m,section
    else:
        assert not witness and "empty factor relation" in section,section
assert "relation solutions=2" in sections[-1] and "(verified)" in sections[-1],sections[-1]
assert int(re.search(r"storage growths=(\d+)",sections[-1])[1])>0,sections[-1]
assert int(re.search(r"compactions=(\d+)",sections[-1])[1])>0,sections[-1]
assert "IMASM nesting depth=3" in sections[-1],sections[-1]
nested_ticks=int(re.search(r"nested mark ticks=(\d+)",sections[-1])[1])
operators=int(re.search(r"circuit operators=(\d+)",sections[-1])[1])
assert nested_ticks>operators*3*12,sections[-1]  # resource transitions also crossed the tower
selection_ticks=int(re.search(r"selection ticks=(\d+)",sections[-1])[1])
path_nodes=int(re.search(r"witness path nodes=(\d+)",sections[-1])[1])
assert selection_ticks==path_nodes*8*3*12,sections[-1]
limb_path=int(re.search(r"limb path nodes=(\d+)",sections[-1])[1])
limb_ticks=int(re.search(r"limb ticks=(\d+)",sections[-1])[1])
continuation=int(re.search(r"continuation=(\d+)",sections[-1])[1])
assert limb_path==path_nodes and continuation==1,sections[-1]
assert limb_ticks==selection_ticks+14*3*12,sections[-1]
assert re.search(r"relation solutions=(\d+)",sections[-1])[1]=="2",sections[-1]
for field in ("decision nodes", "graph reductions", "relation solutions", "witness path nodes"):
    pattern=rf"{field}=(\d+)"
    assert re.search(pattern,sections[-3])[1]==re.search(pattern,sections[-1])[1],(field,sections[-3],sections[-1])
assert re.search(r"\d+ = \d+ x \d+ \(verified\)",sections[-3])[0] == re.search(r"\d+ = \d+ x \d+ \(verified\)",sections[-1])[0]
temporal=sections[-4]
epochs=int(re.search(r"construction epochs=(\d+)",temporal)[1])
temporal_ops=int(re.search(r"circuit operators=(\d+)",temporal)[1])
temporal_ticks=int(re.search(r"nested mark ticks=(\d+)",temporal)[1])
temporal_selection=int(re.search(r"selection ticks=(\d+)",temporal)[1])
assert epochs==8 and "IMASM nesting depth=3" in temporal,temporal
assert temporal_ticks>=temporal_ops*3*12+temporal_selection+(epochs-1)*14*3*12,temporal
assert "10309995 = 3489 x 2955 (verified)" in temporal,temporal
assert "banked face matches CPU combo2 semantics: true" in output,output
print(f"PASS: {len(cases)} exact relation cardinalities and witnesses; nesting, compaction, and growth preserve the relation")
