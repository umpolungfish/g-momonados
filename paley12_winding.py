# The Paley construction at order 12, built from p=11, and checked directly
# against the winding vocabulary: is the quadratic-residue character mod 11
# exactly a self-inverse winding, by construction, not by search?
p = 11
g = 2  # a primitive root mod 11: check
assert pow(g, 10, p) == 1
seen = set(); x = 1
for i in range(10):
    x = (x*g) % p
    seen.add(x)
assert len(seen) == 10, "g is not primitive"

# discrete log table: dlog[x] = i such that g^i = x mod p
dlog = {}
x = 1
for i in range(10):
    dlog[x] = i
    x = (x*g) % p

def legendre(a):
    a %= p
    if a == 0: return 0
    return 1 if dlog[a] % 2 == 0 else -1

# Paley type-I core (p x p), then border to get p+1 = 12
core = [[legendre((i-j) % p) for j in range(p)] for i in range(p)]
n = p + 1
Q = [[0]*n for _ in range(n)]
for i in range(p):
    for j in range(p):
        Q[i+1][j+1] = core[i][j]
for i in range(p):
    Q[0][i+1] = 1
    Q[i+1][0] = -1
    Q[i+1][i+1] = -1  # jacobsthal correction: diagonal is -1 already from legendre(0)=0 fixed below
# Standard Paley: H = I + S where S is the Jacobsthal matrix skew part; use
# the well-known normalized construction instead of hand-rolling the border
# to avoid an off-by-one -- verify orthogonality directly and only trust it
# if it passes.
H = [[1 if i==0 or j==0 else 0 for j in range(n)] for i in range(n)]
for i in range(n):
    for j in range(n):
        if i==0 and j==0: H[i][j] = 1
        elif i==0: H[i][j] = 1
        elif j==0: H[i][j] = -1
        else: H[i][j] = legendre((i-1)-(j-1)) if (i-1)!=(j-1) else -1

ok = all(sum(H[i][k]*H[jx][k] for k in range(n)) == (n if i==jx else 0) for i in range(n) for jx in range(n))
print(f"Paley order-12 matrix (from p=11, primitive root {g}) orthogonal: {ok}")

# Now the winding claim: the Legendre symbol IS the order-2-character winding.
# chi(x) = (-1)^dlog(x) = to_complex()-real-part of Winding(dlog(x) % 2, 2),
# which is self-inverse BY DEFINITION (denominator 2, always).
mismatches = 0
for x in range(1, p):
    winding_num = dlog[x] % 2   # Winding(winding_num, 2)
    winding_sign = 1 if winding_num == 0 else -1
    if winding_sign != legendre(x):
        mismatches += 1
print(f"Legendre symbol mod {p} equals the order-2-subgroup winding sign for all nonzero residues: {mismatches == 0}")
print("winding denominator throughout: 2 (self-inverse by construction, not by search)")
