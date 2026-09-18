"""Independent controls for the production arbitrary-width phase relation."""
from pathlib import Path
import re
import subprocess

ROOT=Path(__file__).resolve().parent
BIN=ROOT/"target"/"release"/"g-momonados"
cases=[
    ("143",4,3,"13","11",1),
    ("18446744400127067027",33,64,"4294967311","4294967357",1),
    ("365375409332725729550962311656937652305923146355",80,64,
     "604462909807314587353099","604462909807314587353145",1),
]
for n,width,depth,p,q,phases in cases:
    out=subprocess.run([str(BIN),"--selector-relation",n,str(width),"1024",str(depth)],
        cwd=ROOT,text=True,capture_output=True,check=True,timeout=10)
    assert out.stderr=="",out.stderr
    assert "denotation bulk vessels=1" in out.stdout,out.stdout
    assert f"internal phase leaves={phases}" in out.stdout,out.stdout
    assert (f"{n} = {p} x {q} (verified)" in out.stdout or
            f"{n} = {q} x {p} (verified)" in out.stdout),out.stdout
    assert "[boot]" not in out.stdout and "selector:" not in out.stdout,out.stdout
    if depth==64:
        assert "nested mark ticks=31" in out.stdout,out.stdout

prime=subprocess.run([str(BIN),"--selector-relation","127","4","1024","3"],
    cwd=ROOT,text=True,capture_output=True,check=True,timeout=10)
assert prime.stderr=="",prime.stderr
assert "denotation bulk vessels=1" in prime.stdout,prime.stdout
assert "internal phase leaves=0" in prime.stdout,prime.stdout
assert "empty factor relation in the declared bit bands" in prime.stdout,prime.stdout
assert not re.search(r"127 = \d+ x \d+",prime.stdout),prime.stdout
print("PASS: arbitrary-width phase relation closes only at terminal TANCH")
