"""Independent bounded-factor controls for the device joint process."""
from pathlib import Path
from math import isqrt
import random
import os
import re
import subprocess

ROOT = Path(__file__).resolve().parent
rng = random.Random(20260910)
cases = []
for m in range(3, 13):
    lo, hi = 1 << (m-1), (1 << m)-1
    for _ in range(2):
        p, q = rng.randrange(lo | 1, hi+1, 2), rng.randrange(lo | 1, hi+1, 2)
        cases.append((p*q, m))
    cases.extend([(hi*hi-2, m), (hi*hi, m)])
expected = []
for n, m in cases:
    lo, hi = 1 << (m-1), (1 << m)-1
    expected.append(next((p for p in range(lo | 1, min(hi, isqrt(n))+1, 2)
                          if n % p == 0 and lo <= n//p <= hi), None))
for m in range(15, 21):
    lo, hi = 1 << (m-1), (1 << m)-1
    for _ in range(2):
        p, q = rng.randrange(lo | 1, hi+1, 2), rng.randrange(lo | 1, hi+1, 2)
        n = p*q
        cases.append((n, m))
        expected.append(next((a for a in range(lo | 1, min(hi, isqrt(n))+1, 2)
                              if n % a == 0 and lo <= n//a <= hi), None))
cases += [(1000000016000000063, 30), (574208998828409591, 30)]
expected += [1000000007, 570425377]
commands = [f"gpu_kernel selector_joint {n} {m}" for n, m in cases]
run = subprocess.run([str(ROOT / "target" / os.environ.get("PROFILE", "release") / "g-momonados")],
                     input="\n".join([*commands, "quit", ""]),
                     text=True, capture_output=True, cwd=ROOT, timeout=360, check=True)
output = run.stdout + run.stderr
(ROOT / "measurements/selector_joint_controls.log").write_text(output)
sections = re.split(r"⊙> gpu_kernel selector_joint \d+ \d+\s*\n", output)[1:]
assert len(sections) == len(cases), output
for (n, m), want, section in zip(cases, expected, sections):
    match = re.search(r"^\s*(\d+) = (\d+) x (\d+)\s+\(verified\)", section, re.M)
    if want is None:
        assert not match and "no factor found in the canonical space" in section, section
    else:
        assert match, section
        nn, p, q = map(int, match.groups())
        assert nn == n and p == want and p*q == n, section
    assert "executed relation prefixes:" in section, section
    visits = int(re.search(r"relation prefixes: (\d+)", section)[1])
    lifts = int(re.search(r"prefix subdivisions: (\d+)", section)[1])
    assert visits == (1 << (min(m, 14)-1)) + 2*lifts, section
print(f"PASS: {len(cases)} independent bounded-factor controls")
