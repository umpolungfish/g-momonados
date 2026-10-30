#!/usr/bin/env python3
"""
STAGE 111 — BINARY TRAFFIC DECODER / EXPLICIT INVERSE

Native Stage-110 sector:
    execution-order weights = (1,2,4,8)
    C = 31 + tau
    R = 16 + tau
    tau in {0,...,15}

Because tau is the binary numeral of the schedule bits,
the aggregate map has an explicit inverse.

For schedule b=(b1,b2,b3,b4):
    tau = b1 + 2 b2 + 4 b3 + 8 b4.

Decoder:
    b1 = bit_0(tau)
    b2 = bit_1(tau)
    b3 = bit_2(tau)
    b4 = bit_3(tau)

Equivalently:
    b_i = floor(tau / 2^(i-1)) mod 2.

Thus aggregate traffic is not merely injective; it is a complete
schedule code on this measured binary-weight sector.
"""

from itertools import product

weights=(1,2,4,8)

def encode(bits):
    tau=sum(b*w for b,w in zip(bits,weights))
    return (31+tau,16+tau)

def decode(C,R):
    assert C-R == 15
    tau=C-31
    assert tau == R-16
    assert 0 <= tau < 16
    return tuple((tau >> i) & 1 for i in range(4))

rows=list(product((0,1), repeat=4))

for b in rows:
    cr=encode(b)
    assert decode(*cr)==b

for tau in range(16):
    b=tuple((tau>>i)&1 for i in range(4))
    assert sum(x*w for x,w in zip(b,weights))==tau

print("STAGE 111 — BINARY TRAFFIC DECODER / EXPLICIT INVERSE")
print("="*88)
print("111A native Stage-110 code:")
print("  tau=C-31=R-16")
print("  tau=b1+2b2+4b3+8b4")
print()
print("111B explicit inverse:")
print("  b_i = floor(tau / 2^(i-1)) mod 2")
print()
print("111C roundtrip audit:")
print("  decode(encode(b))=b for all 16 schedules: TRUE")
print()
print("111D code table:")
for tau in range(16):
    b=tuple((tau>>i)&1 for i in range(4))
    C,R=31+tau,16+tau
    print(f"  tau={tau:2d}: C/R={C}/{R} -> bits={b}")
print()
print("111E information statement:")
print("  on this binary-weight sector, aggregate traffic retains the")
print("  full 4-bit schedule information.")
print()
print("111F projection distinction:")
print("  aggregate traffic is lossless on schedules")
print("  AND endpoint projection still collapses all 16 to T×16.")
print()
print("PARACONSISTENT LANDING")
print("  aggregate code is fully invertible")
print("  AND the final endpoint remains completely non-injective.")
print()
print("STAGE 111 RESULT : True")
