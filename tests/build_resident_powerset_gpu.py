"""Build the current production CUDA source with retained N-only 128-bit input."""
from pathlib import Path
import subprocess
import hashlib
import json
root = Path(__file__).resolve().parents[1]
base = root / 'measurements/powerset_resident_20261015'
arithmetic = (root / 'src/gpu_ecm_word.cu').read_text()
start = arithmetic.index('// n is odd, so binary gcd')
end = arithmetic.index('__device__ __noinline__ bool proper', start)
source = ((base / 'source_header.cuh').read_text()
    + arithmetic.split('// One warp evaluates', 1)[0] + arithmetic[start:end]
    + '\n__device__ bool proper(const Word &g,const Word &n){return !zero(g)&&!one(g)&&compare(g,n)<0;}\n'
    + (root / 'src/idempotent_gpu.cu').read_text()
    + (root / 'tests/resident_powerset_gpu_host.cuh').read_text())
(base / 'resident_test.cu').write_text(source)
subprocess.run(['nvcc', '--std=c++17', '-arch=sm_86', '-O2', str(base / 'resident_test.cu'), '-o', str(base / 'resident_test')], check=True)
files = ['src/idempotent_gpu.cu', 'src/gpu_ecm_word.cu', 'tests/resident_powerset_gpu_host.cuh', 'measurements/powerset_resident_20261015/source_header.cuh', 'measurements/powerset_resident_20261015/resident_test']
(base / 'bindings.json').write_text(json.dumps({name: hashlib.sha256((root/name).read_bytes()).hexdigest() for name in files}, indent=2)+'\n')
