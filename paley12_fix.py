p = 11
g = 2
dlog = {}
x = 1
for i in range(p-1):
    dlog[x] = i
    x = (x*g) % p

def legendre(a):
    a %= p
    if a == 0: return 0
    return 1 if dlog[a] % 2 == 0 else -1

n = p + 1

def build(diag_val, corner, row0, col0):
    H = [[0]*n for _ in range(n)]
    H[0][0] = corner
    for j in range(1, n): H[0][j] = row0
    for i in range(1, n): H[i][0] = col0
    for i in range(1, n):
        for j in range(1, n):
            a, b = i-1, j-1
            H[i][j] = diag_val if a == b else legendre(b - a)
    return H

def is_hadamard(H):
    return all(sum(H[i][k]*H[jx][k] for k in range(n)) == (n if i==jx else 0)
               for i in range(n) for jx in range(n))

for diag_val in (1, -1):
    for corner in (1, -1):
        for row0 in (1, -1):
            for col0 in (1, -1):
                H = build(diag_val, corner, row0, col0)
                if is_hadamard(H):
                    print(f"orthogonal with diag={diag_val} corner={corner} row0={row0} col0={col0}")
