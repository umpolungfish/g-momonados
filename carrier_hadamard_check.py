# The sixteen-value carrier IS P({T,F,t,f}), the powerset under symmetric
# difference, isomorphic to (Z/2)^4. Its own character table -- read the
# marks as bit-vectors and pair them by (-1)^(x . y) -- is the Sylvester
# construction, native to the carrier's group structure, not retrofitted.
# Check directly: is it a genuine +-1 orthogonal Hadamard matrix of order 16?

marks = ['T','F','t','f']  # the four independent bits, Definition 3.1

def bits(subset_idx):
    # subset_idx in 0..15, bit i set means marks[i] present
    return [(subset_idx >> i) & 1 for i in range(4)]

n = 16
H = [[0]*n for _ in range(n)]
for x in range(n):
    bx = bits(x)
    for y in range(n):
        by = bits(y)
        dot = sum(a*b for a,b in zip(bx,by)) % 2
        H[x][y] = -1 if dot == 1 else 1

# check H H^T = nI
ok = True
for i in range(n):
    for j in range(n):
        s = sum(H[i][k]*H[j][k] for k in range(n))
        want = n if i==j else 0
        if s != want:
            ok = False

entries_pm1 = all(H[i][j] in (1,-1) for i in range(n) for j in range(n))
print(f"order 16 native carrier character table: entries all +-1: {entries_pm1}")
print(f"H H^T == 16*I exactly: {ok}  (genuine Hadamard matrix, order {n})")

# Now check negation's role: does negation (T<->F, t<->f swap) act as a
# symmetry of this matrix -- i.e. is H invariant (up to row/col permutation)
# under the carrier's own negation involution?
def negate(idx):
    b = bits(idx)
    # swap bit0(T)<->bit1(F), bit2(t)<->bit3(f)
    nb = [b[1], b[0], b[3], b[2]]
    return sum(nb[i] << i for i in range(4))

perm = [negate(x) for x in range(n)]
Hneg = [[H[perm[i]][perm[j]] for j in range(n)] for i in range(n)]
same = (Hneg == H)
print(f"H invariant under the carrier's own negation (as a simultaneous row/col permutation): {same}")

print()
print("=== d=12: what the Weyl-Heisenberg structure already carries natively ===")
import cmath
d = 12
w = cmath.exp(2j*cmath.pi/d)
F = [[w**(j*k) for k in range(d)] for j in range(d)]  # unnormalized DFT_12

def inner(a, b):
    return sum(a[i]*b[i].conjugate() for i in range(len(a)))

ortho = all(abs(inner(F[i], F[j])) < 1e-9 for i in range(d) for j in range(d) if i != j)
modulus_one = all(abs(abs(F[i][j]) - 1.0) < 1e-9 for i in range(d) for j in range(d))
print(f"DFT_12 entries all modulus 1: {modulus_one}")
print(f"DFT_12 rows pairwise orthogonal: {ortho}")
print("-> a genuine COMPLEX (Butson-type) Hadamard matrix of order 12,")
print("   native to the WH group at d=12 already in the kernel.")
print("   This is NOT the object the real Hadamard conjecture is about:")
print("   the conjecture is real +-1 entries only. Complex Hadamard matrices")
print("   of every order d exist trivially (the DFT always works); the real")
print("   +-1 case at order 12 is already separately known via Paley (p=11).")
