"""Check the recorded depth-64 factor-relation width sweep."""
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parent
text=(ROOT/"measurements"/"factor_relation_nested_width_sweep.log").read_text()
sections=re.split(r"⊙> gpu_kernel selector_relation \d+ \d+ \d+ \d+\s*\n",text)[1:]
expected={13:(50309115,6145,8187),14:(201281531,12289,16379),
          15:(805216251,24577,32763),16:(3221045243,49153,65531),
          17:(12884541435,98305,131067),18:(51538886651,196609,262139)}
assert len(sections)==len(expected),text
for section,(width,(n,p,q)) in zip(sections,expected.items()):
    assert "INCOMPLETE" not in section,section
    assert f"N={n} m={width}" in section and "IMASM nesting depth=64" in section,section
    assert int(re.search(r"relation solutions=(\d+)",section)[1])==2,section
    path=int(re.search(r"witness path nodes=(\d+)",section)[1])
    selected=int(re.search(r"selection ticks=(\d+)",section)[1])
    assert path==2*width and selected==path*8*64*12,section
    got=re.search(r"(\d+) = (\d+) x (\d+) \(verified\)",section)
    assert got and tuple(map(int,got.groups()))==(n,p,q),section
print("PASS: widths 13..18 close at IMASM depth 64 with exact pairs and selection-tick laws")
