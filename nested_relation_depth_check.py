"""Verify exact factor-relation invariance while IMASM nesting deepens."""
from pathlib import Path
import re
import subprocess

ROOT=Path(__file__).resolve().parent
N=10309995
WIDTH=12
CAPACITY=1024
DEPTHS=(0,1,2,3,4,8,16,32,64)
commands=[f"gpu_kernel selector_relation {N} {WIDTH} {CAPACITY} {d}" for d in DEPTHS]
log=ROOT/"measurements"/"factor_relation_nested_depth_control.log"
with log.open("w") as sink:
    subprocess.run([str(ROOT/"target"/"release"/"g-momonados")],
        input="\n".join([*commands,"quit",""]),text=True,stdout=sink,
        stderr=subprocess.STDOUT,cwd=ROOT,check=True,timeout=300)
output=log.read_text()
log.write_text(output)
sections=re.split(r"⊙> gpu_kernel selector_relation \d+ \d+ \d+ \d+\s*\n",output)[1:]
assert len(sections)==len(DEPTHS),output

fields=("decision nodes","graph reductions","relation solutions",
        "witness path nodes","node storage","storage growths","compactions",
        "device launches")
baseline={field:int(re.search(rf"{field}=(\d+)",sections[0])[1]) for field in fields}
baseline_factor=re.search(r"\d+ = \d+ x \d+ \(verified\)",sections[0])[0]
ticks_per_depth=None
for depth,section in zip(DEPTHS,sections):
    assert "INCOMPLETE" not in section,section
    for field,value in baseline.items():
        assert int(re.search(rf"{field}=(\d+)",section)[1])==value,(depth,field,section)
    assert re.search(r"\d+ = \d+ x \d+ \(verified\)",section)[0]==baseline_factor,section
    nested=int(re.search(r"nested mark ticks=(\d+)",section)[1])
    selected=int(re.search(r"selection ticks=(\d+)",section)[1])
    if depth==0:
        assert nested==selected==0,section
    else:
        unit=nested//depth
        ticks_per_depth=ticks_per_depth or unit
        assert nested==ticks_per_depth*depth,section
        assert selected==baseline["witness path nodes"]*8*12*depth,section

print(f"PASS: depths {DEPTHS[0]}..{DEPTHS[-1]} preserve one exact relation; "
      f"nested traffic is {ticks_per_depth} marks per depth")
