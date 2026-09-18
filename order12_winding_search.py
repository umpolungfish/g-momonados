# Exhaustive search: does any bilinear/affine winding assignment mod 12,
# built purely from small integer constants, land EVERY entry of a 12x12
# matrix on a self-inverse winding (numerator mod 12 in {0,6}, i.e. 0 or 1/2
# of a turn) while also giving a genuine orthogonal +-1 matrix?
#
# W(j,k) = (alpha*j*k + beta*j + gamma*k + delta) mod 12
d = 12

def self_inverse_num(n12):
    return n12 % 12 in (0, 6)

def sign(n12):
    return 1 if n12 % 12 == 0 else -1  # 0 -> winding 0 -> +1; 6 -> winding 1/2 -> -1

found = []
tried = 0
for alpha in range(12):
    for beta in range(12):
        for gamma in range(12):
            for delta in range(12):
                tried += 1
                ok = True
                M = [[0]*d for _ in range(d)]
                for j in range(d):
                    for k in range(d):
                        n = (alpha*j*k + beta*j + gamma*k + delta) % 12
                        if not self_inverse_num(n):
                            ok = False
                            break
                        M[j][k] = sign(n)
                    if not ok: break
                if ok:
                    # check orthogonality
                    orth = True
                    for i in range(d):
                        for jj in range(d):
                            s = sum(M[i][kk]*M[jj][kk] for kk in range(d))
                            want = d if i==jj else 0
                            if s != want:
                                orth = False
                    found.append((alpha,beta,gamma,delta,orth))

print(f"tried {tried} affine/bilinear assignments mod 12")
print(f"assignments where every entry is self-inverse: {len(found)}")
orthogonal_ones = [f for f in found if f[4]]
print(f"of those, genuinely orthogonal (real Hadamard matrix): {len(orthogonal_ones)}")
if orthogonal_ones:
    a,b,g,dl,_ = orthogonal_ones[0]
    print(f"example: alpha={a} beta={b} gamma={g} delta={dl}")
