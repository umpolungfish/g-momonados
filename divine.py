#!/usr/bin/env python3
import math

N = 233108530344407544527637656910680524145619812480305449042948611968495918245135782867888369318577116418213919268572658314913060672626911354027609793166341626693946596196427744273886601876896313468704059066746903123910748277606548649151920812699309766587514735456594993207
G = 589020456127887575227372528456363790008947214242429697172079797618372595564569907904261822456541945789857722463921275922906344734372871
L = 217224540400747971320119266447907195989202555568608769537293089515198518506389384916279337574613065132765541409945108817923771150396613

def isqrt_exact(x):
    if x < 0: return None
    r = math.isqrt(x)
    return r if r*r == x else None

found = []

# Candidate expressions in G and L (linear + quadratic, small coeffs)
candidates = []
for a in range(-16, 17):
    for b in range(-16, 17):
        if a == 0 and b == 0: continue
        candidates.append((f"{a}*G + {b}*L", a*G + b*L))
for a in range(-4, 5):
    for b in range(-4, 5):
        for c in range(-4, 5):
            if a==0 and b==0 and c==0: continue
            candidates.append((f"{a}*G^2 + {b}*G*L + {c}*L^2", a*G*G + b*G*L + c*L*L))

# Test each candidate as S = p+q
for label, val in candidates:
    if val <= 0: continue
    d2 = val*val - 4*N
    d = isqrt_exact(d2)
    if d is not None:
        p, q = (val - d)//2, (val + d)//2
        if p*q == N and p > 1:
            found.append(("S", label, val, p, q))

# Test each candidate as D = q-p
for label, val in candidates:
    if val <= 0: continue
    s2 = val*val + 4*N
    s = isqrt_exact(s2)
    if s is not None:
        p, q = (s - val)//2, (s + val)//2
        if p*q == N and p > 1:
            found.append(("D", label, val, p, q))

# Also test direct gcd (in case a lane or combo is a factor)
for label, val in candidates:
    g = math.gcd(abs(val), N)
    if 1 < g < N:
        found.append(("GCD", label, val, g, N//g))

for kind, label, val, p, q in found:
    print(f"[{kind}] {label} = {val}")
    print(f"    p = {p}")
    print(f"    q = {q}")

if not found:
    print("nothing found in this candidate set")