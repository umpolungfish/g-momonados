"""Check the device envelope with independent Python integer enumeration."""
from pathlib import Path
import ctypes
import random
import subprocess

ROOT = Path(__file__).resolve().parent
source = ROOT / "measurements/selector_curve_host.cpp"
library = ROOT / "measurements/selector_curve_host.so"
source.write_text('''
#define __device__
#define JOINT_ROOT_BITS 14
struct Dim { unsigned x; } blockIdx, blockDim, threadIdx;
void atomicMin(unsigned long long *p,unsigned long long v) { if(v<*p)*p=v; }
void atomicAdd(unsigned long long *p,unsigned long long v) { *p+=v; }
#include "../src/gpu_joint.cuh"
extern "C" long long count(unsigned long long n,unsigned long long p,
 unsigned long long q,unsigned t,unsigned long long lo,unsigned long long hi) {
 return (long long)jstrip_count(jstrip(n,jnode(p,q,t),lo,hi),0,(hi-lo)/(1ULL<<t));
}
extern "C" bool select_curve(unsigned long long n,unsigned long long p,
 unsigned t,unsigned m,unsigned long long *factor) {
 JNode node=jnode(p,(n*jinv(p,(1ULL<<t)-1))&((1ULL<<t)-1),t);
 JBox box; JU products=0; *factor=n;
 if(!jbounds(n,node,m,box)) return true;
 return jcurve(n,node,box,*factor,products);
}
''')
subprocess.run(["c++", "-shared", "-fPIC", "-O2", "-fsanitize=undefined",
                "-fno-sanitize-recover=all", str(source), "-o", str(library)], check=True)
lib = ctypes.CDLL(str(library))
lib.count.argtypes = [ctypes.c_ulonglong]*3 + [ctypes.c_uint] + [ctypes.c_ulonglong]*2
lib.count.restype = ctypes.c_longlong
lib.select_curve.argtypes = [ctypes.c_ulonglong]*2 + [ctypes.c_uint]*2 + [ctypes.POINTER(ctypes.c_ulonglong)]
lib.select_curve.restype = ctypes.c_bool
rng = random.Random(20260910)
for trial in range(3000):
    m = rng.randrange(3, 31)
    t = rng.randrange(2, m+1)
    s = 1 << t
    a = rng.randrange((1 << (m-1)) | 1, 1 << m, 2)
    b = rng.randrange((1 << (m-1)) | 1, 1 << m, 2)
    n, p, q = a*b, a % s, b % s
    lo = (1 << (m-1)) + ((p-(1 << (m-1))) % s)
    length = min(65, ((1 << m)-1-lo)//s+1)
    if length == 0:
        continue
    hi = lo+s*(length-1)
    inv = pow(p, -1, s)
    slope = -q*inv % s
    offset = ((n-p*q)//s)*inv % s
    qb = q+s*(offset+slope*((lo-p)//s))
    center = lo+s*(length//2)
    ud, ld = lo*hi, center*center
    um, lm = ud*s*s, ld*s*s
    ua, ub = -n*s-ud*s*slope, n*hi-ud*qb
    la, lb = -n*s-ld*s*slope, n*(2*center-lo)-ld*qb
    expected = sum((ub+ua*i)//um + (-lb-la*i)//lm+1 for i in range(length))
    actual = lib.count(n, p, q, t, lo, hi)
    assert actual == expected, (trial, m, t, actual, expected)
print("PASS: 3000 device-envelope counts versus direct integer enumeration; UBSan clean")
resolved = 0
for trial in range(3000):
    m = rng.randrange(3, 31)
    t = rng.randrange(max(2, m-10), m+1)
    s, lo, hi = 1 << t, 1 << (m-1), (1 << m)-1
    a, b = rng.randrange(lo | 1, hi+1, 2), rng.randrange(lo | 1, hi+1, 2)
    n = a*b if trial % 2 else rng.randrange(lo*lo | 1, hi*hi+1, 2)
    p = a % s
    expected = next((v for v in range(lo+(p-lo) % s, hi+1, s)
                     if v > 1 and n % v == 0 and v <= n//v and lo <= n//v <= hi), n)
    factor = ctypes.c_ulonglong(n)
    if lib.select_curve(n, p, t, m, ctypes.byref(factor)):
        resolved += 1
        assert factor.value == expected, (trial, n, m, t, p, factor.value, expected)
print(f"PASS: {resolved}/3000 envelopes resolved, every result matches exhaustive bounded factors")
