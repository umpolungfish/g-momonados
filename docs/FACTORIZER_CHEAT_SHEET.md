# 📋 IMSCRIBER Factorizer Cheat Sheet

**Quick Reference for Factorization Operations**  
*Version: 1.0 | Date: November 14, 2026*

---

## 🚀 Quick Start

### Factor a Number (Fastest)
```bash
vox mpqs <N>
```

### Factor with Full Validation
```bash
# 208-bit example
y_divisor factor 249880503243289860256926205840602687030138007067151469448678503

# With Lean validation
y_divisor factors 249880503243289860256926205840602687030138007067151469448678503
```

### Check Installation
```bash
# G-mOMonadOS
ls G-mOMonadOS/target/release/g-momonados

# Vox
ls Vox/target/release/vox

# Build (if needed)
cd G-mOMonadOS && cargo build --release --features hosted
cd ../Vox && cargo build --release --features resident
```

---

## 📊 Performance Reference

| Bits | Time (Trap-First) | Time (MPQS Direct) | Producer | Status |
|------|-------------------|-------------------|----------|--------|
| 200 | ~27s | ~27s | MPQS | ✅ Verified |
| 208 | ~68s | ~35s | MPQS | ✅ Verified |
| 224 | ~182s | ~63s | MPQS | ✅ Verified |
| 256 | Bound (90s) | ~? | - | ⚠️ No factor |
| 895 | Bound (90s) | Bound (90s) | - | ❌ No factor |

**Minimum Test Domain**: 200-bit near-balanced semiprimes

---

## 🎯 Commands Reference

### G-mOMonadOS Commands

| Command | Description | Example |
|---------|-------------|---------|
| `y_divisor factor N` | Resident GLUT factorization | `y_divisor factor 266175...` |
| `y_divisor factors N` | With Lean validation | `y_divisor factors 266175...` |
| `y_divisor from N D` | Verify divisor D of N | `y_divisor from 266175... 134192...` |
| `y_divisor bitfold N` | Bitfold-based factorization | `y_divisor bitfold 266175...` |
| `y_divisor winding N [B]` | Winding-based (base B) | `y_divisor winding 266175... 2` |
| `y_divisor profile N` | Profile Y evaluator | `y_divisor profile 1009` |
| `ququart_factor N` | Ququart factorization | `ququart_factor 266175...` |
| `gpu_glut N` | GPU-accelerated GLUT | `gpu_glut 266175...` |

### Vox Commands

| Command | Description | Example |
|---------|-------------|---------|
| `vox mpqs N [MH] [BD]` | MPQS factorization | `vox mpqs 266175...` |
| `vox factor N` | Shape-routed factorization | `vox factor 266175...` |
| `vox scout N` | Quick shape analysis | `vox scout 266175...` |
| `vox morphism-factor W` | Factor IMASM word | `vox morphism-factor "⊢⊞≻..."` |

---

## ⚙️ Environment Variables

### Dispatch Control

| Variable | Default | Effect |
|----------|---------|--------|
| `GM_TRAP_OFF=1` | - | Disable arm trap |
| `GM_TRAP_K=64` | 64 | Trap multiplier arms |
| `GM_TRAP_WINDOW=32` | 32 | Probe slots per arm |
| `GM_FACTOR_PORT_BOUND=48` | 48 | Max ports before MPQS |
| `GM_FACTOR_TRACE=1` | - | Enable timing trace |

### Performance

| Variable | Default | Effect |
|----------|---------|--------|
| `VOX_STEPS=1000000000000` | - | Max Vox steps |
| `QUQUART_TRACE=1` | - | Enable Ququart trace |
| `GLUT_GPU_DEVICE=0` | 0 | CUDA device index |
| `RUST_LOG=debug` | - | Rust logging level |

---

## 🔍 Configuration Presets

### Fast Factorization
```bash
# Bypass trap and port walk, go straight to MPQS
export GM_TRAP_OFF=1
export GM_FACTOR_PORT_BOUND=1
vox mpqs <N>
```

### Full Debug Trace
```bash
export GM_FACTOR_TRACE=1
export QUQUART_TRACE=1
export RUST_LOG=debug
GM_FACTOR_PORT_BOUND=96 y_divisor factor <N>
```

### Maximum Search
```bash
# Extended bounds for difficult N
export GM_TRAP_K=128
export GM_TRAP_WINDOW=64
export GM_FACTOR_PORT_BOUND=96
y_divisor factor <N>
```

### GPU Mode
```bash
export GLUT_GPU_DEVICE=0
export GLUT_CHECKPOINT=1
gpu_glut <N>
```

---

## 📝 Output Interpretation

### Success Output
```
candidate-source=resident-glut producer=mpqs-relation-source 
quotient=19897152087528681409226432122633 
SIC-residual=0 SIC-tolerance=1
Factor: 12558606485192031214854666513391 × 19897152087528681409226432122633
```

### Fields Explained
- `candidate-source`: Source mechanism (resident-glut, native, etc.)
- `producer`: Actual producing arm (mpqs-relation-source is current default)
- `quotient`: N/d (derived, not discovered)
- `SIC-residual`: Maximum residual in SIC reconstruction (0 = perfect)
- `SIC-tolerance`: Acceptance threshold (1 = integer tolerance)

### Producer Tags
| Tag | Meaning | Speed |
|-----|---------|-------|
| `arm-trap-mu-delta` | Trap caught it | ⚡ Fastest |
| `dialectic-source` | Dialectic found it | ⚡ Fast |
| `prepared-glut` | Native GLUT arm | 🏃 Medium |
| `mpqs-relation-source` | MPQS found it | 🐢 Medium |

### Lean Validation
```
Lean: hasT=true hasN=false hasF=false hasB=false
```
- `hasT=true` ✅ = Y/SIXTEEN_3 kernel passed
- `hasN=false` = No native factor (normal)
- `hasF=false` = No full factorization (normal for semiprime)
- `hasB=false` = No bounded verification

---

## 🎲 Common Scenarios

### Scenario 1: Factor a 208-bit Semiprime
```bash
# Direct MPQS (fastest)
vox mpqs 249880503243289860256926205840602687030138007067151469448678503

# Full resident GLUT with validation
y_divisor factors 249880503243289860256926205840602687030138007067151469448678503
```

### Scenario 2: Verify a Known Factor
```bash
# Given N and d, verify d × (N/d) = N
y_divisor from 266175155279792434973506259190185380993589458872977057421959357 13419294329574030763782528217097
```

### Scenario 3: Debug Slow Factorization
```bash
# Enable tracing and increase bounds
export GM_FACTOR_TRACE=1
export GM_FACTOR_PORT_BOUND=96
y_divisor factor <N>

# Check the trace output for bottlenecks
```

### Scenario 4: Test with Small Number
```bash
# Small test (should be fast)
y_divisor factor 91
# Expected: 7 × 13

vox mpqs 91
# Expected: 91 factor 7
```

### Scenario 5: Generate Test Input
```bash
# Python: generate 208-bit semiprime
python3 -c "
from Crypto.Util.number import getPrime
p = getPrime(104)
q = getPrime(104)
N = p * q
print(N)
print(f'# p={p}, q={q}')
"
```

---

## ⚠️ Common Issues & Fixes

### Issue: No Factor Found / Timeout
```bash
# Solution 1: Try MPQS directly
vox mpqs <N>

# Solution 2: Increase bounds
export GM_FACTOR_PORT_BOUND=96
y_divisor factor <N>

# Solution 3: Increase time
export VOX_STEPS=1000000000000
vox mpqs <N>

# Solution 4: Check if N is prime
vox scout <N>
```

### Issue: Build Failed
```bash
# Solution 1: Clean and rebuild
cargo clean
cargo build --release --features hosted

# Solution 2: Update Rust
rustup update
rustup default stable

# Solution 3: Install dependencies
sudo apt-get install build-essential cmake pkg-config libssl-dev
```

### Issue: Out of Memory
```bash
# Solution 1: Use MPQS (lower memory)
vox mpqs <N>

# Solution 2: Reduce bit length
# Solution 3: Use machine with more RAM (16+ GB recommended)
```

### Issue: GPU Not Detected
```bash
# Solution 1: Install CUDA
sudo apt-get install nvidia-cuda-toolkit

# Solution 2: Verify CUDA
nvcc --version

# Solution 3: Set device explicitly
export GLUT_GPU_DEVICE=0
gpu_glut <N>
```

---

## 📊 Verification

### Manual Verification
```bash
# Python verification
python3 -c "
N = 266175155279792434973506259190185380993589458872977057421959357
d = 13419294329574030763782528217097
q = 19835257260375005453724574658581
assert d * q == N, f'Product mismatch'
assert N % d == 0, f'Not a divisor'
print('✅ Verification passed')
"
```

### Verify from Output
```bash
# Extract factor from output and verify
OUTPUT=$(y_divisor factor <N> 2>&1)
FACTOR=$(echo "$OUTPUT" | grep "Factor:" | awk '{print $2}')
QUOTIENT=$(echo "$OUTPUT" | grep "Factor:" | awk '{print $4}')
python3 -c "N=<N>; d=$FACTOR; q=$QUOTIENT; assert d*q == N; print('✅')"
```

---

## 🔗 File Locations

| Resource | Path |
|----------|------|
| Main binary | `G-mOMonadOS/target/release/g-momonados` |
| Vox binary | `Vox/target/release/vox` |
| Source code | `G-mOMonadOS/src/` |
| Measurements | `G-mOMonadOS/measurements/` |
| This guide | `G-mOMonadOS/docs/FACTORIZER_USER_GUIDE.md` |
| Catalog | `~/.imscrbgrmr/catalog.json` |

---

## 📚 Learn More

- **Full User Guide**: `G-mOMonadOS/docs/FACTORIZER_USER_GUIDE.md`
- **Construction Doc**: `ig-docs/resident_factor_synthesis_construction_20261114.md`
- **Progress Tracking**: `ig-docs/factor_synthesis_progress.md`
- **Receipts**: `G-mOMonadOS/measurements/factor_source_carrier_20261011/`
- **Project Guidelines**: `AGENTS.md`

---

## 🎯 Quick Commands Summary

```bash
# Build
cargo build --release --features hosted

# Factor (fast)
vox mpqs <N>

# Factor (full validation)
y_divisor factors <N>

# Factor (with trace)
GM_FACTOR_TRACE=1 y_divisor factor <N>

# GPU factor
gpu_glut <N>

# Verify
y_divisor from <N> <D>

# Scout (quick analysis)
vox scout <N>
```

---

*Last updated: November 14, 2026*
*Status: 4-minute floor met for 208-bit and 224-bit targets*
