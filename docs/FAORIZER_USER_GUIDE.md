# 🔢 IMSCRIBER Factorizer User Guide

**Version**: 1.0  
**Date**: November 14, 2026  
**Status**: Active Development

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Installation](#installation)
4. [Command Reference](#command-reference)
5. [Configuration](#configuration)
6. [Input Formats](#input-formats)
7. [Output Interpretation](#output-interpretation)
8. [Execution Modes](#execution-modes)
9. [Performance Guidelines](#performance-guidelines)
10. [Best Practices](#best-practices)
11. [Troubleshooting](#troubleshooting)
12. [Advanced Usage](#advanced-usage)
13. [Architecture Overview](#architecture-overview)
14. [Receipts and Verification](#receipts-and-verification)

---

## Overview

The IMSCRIBER Factorizer is a multi-stage, hybrid factorization system designed for **arbitrary factor production** of large semiprimes using the project's native mechanisms. It combines:

- **Semiclassical algorithms** (MPQS, ECM)
- **Quantum-inspired carriers** (Ququart, Anyon socket)
- **Symbolic computation** (Lean Y/SIXTEEN_3 verification)
- **Monadic state management** (IMASM word processing)

The system is **N-only** — it accepts only the source numeral N as input, with no pre-supplied factors, witnesses, or hints.

### Key Features

| Feature | Description |
|---------|-------------|
| **N-only Input** | Accepts only the source numeral N; no factor hints |
| **Verified Output** | Every factor pair is independently verified: d × q = N |
| **Resident State** | Maintains prepared, modular, and pre-measurement banks |
| **Monad Laws** | Unit, multiplication, and bind operations preserved |
| **SIC Reconstruction** | Terminal SIC witness with signed residuals |
| **Lean Validation** | Y/SIXTEEN_3 kernel bridge certification |

---

## Quick Start

### Factor a 208-bit Semiprime

```bash
# Using G-mOMonadOS directly
y_divisor factor 249880503243289860256926205840602687030138007067151469448678503

# Expected output (example):
# candidate-source=resident-glut producer=mpqs-relation-source quotient=19897152087528681409226432122633 SIC-residual=0 SIC-tolerance=1
# Factor: 12558606485192031214854666513391 × 19897152087528681409226432122633
```

### Factor using Vox MPQS (fastest path)

```bash
# Direct MPQS invocation
vox mpqs 249880503243289860256926205840602687030138007067151469448678503

# Expected output:
# 249880503243289860256926205840602687030138007067151469448678503 factor 12558606485192031214854666513391
```

### Factor with Full Trace

```bash
# Enable detailed tracing
GM_FACTOR_TRACE=1 y_divisor factor 249880503243289860256926205840602687030138007067151469448678503

# Output includes per-stage timing and step counts
```

---

## Installation

### Prerequisites

#### System Requirements
- **OS**: Linux (x86_64 tested)
- **CPU**: Modern multi-core processor
- **RAM**: 16+ GB recommended (8 GB minimum for 200-bit targets)
- **Disk**: 50+ GB for full build and cache
- **GPU**: Optional (CUDA 12.x for GPU acceleration)

#### Dependencies
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup default stable
rustup component add rust-src

# Build tools
sudo apt-get install build-essential cmake pkg-config libssl-dev

# CUDA (optional, for GPU support)
sudo apt-get install nvidia-cuda-toolkit
```

### Building from Source

```bash
# Clone the repository (if not already in imsgct)
cd /home/mrnob0dy666/imsgct

# Build G-mOMonadOS (hosted mode, default)
cd G-mOMonadOS
cargo build --release --features hosted

# Build Vox (required dependency)
cd ../Vox
cargo build --release --features resident

# Verify builds
ls target/release/g-momonados
target/release/vox
```

### Binary Locations

| Component | Binary | Location |
|-----------|--------|----------|
| G-mOMonadOS | `g-momonados` | `G-mOMonadOS/target/release/g-momonados` |
| Vox | `vox` | `Vox/target/release/vox` |
| Ququart Factor | `ququart_factor` | `G-mOMonadOS/target/release/ququart_factor` |

---

## Command Reference

### Primary Commands

#### `y_divisor` — Resident Factor Synthesis

The main factorization entry point with multiple subcommands.

```
y_divisor <subcommand> [options] [N]
```

**Subcommands**:

| Subcommand | Description | Example |
|------------|-------------|---------|
| `factor N` | Factor N using resident GLUT with MPQS escalation | `y_divisor factor 266175155279792434973506259190185380993589458872977057421959357` |
| `factors N` | Factor N with Lean Y/SIXTEEN_3 validation | `y_divisor factors 266175155279792434973506259190185380993589458872977057421959357` |
| `from N D` | Verify divisor D of N | `y_divisor from 266175... 134192...` |
| `bitfold N` | Bitfold-based factorization | `y_divisor bitfold 266175...` |
| `winding N [BASE]` | Winding-based factorization | `y_divisor winding 266175... 2` |
| `profile N` | Profile Y evaluator on N | `y_divisor profile 1009` |

**Options**:
- `both` — Enable both arms (native and joint)

#### `vox` — Vox Factorization Suite

```
vox <subcommand> [options] [N]
```

**Subcommands**:

| Subcommand | Description | Example |
|------------|-------------|---------|
| `mpqs N [MH] [BOUND]` | Multiple Polynomial Quadratic Sieve | `vox mpqs 266175155279792434973506259190185380993589458872977057421959357` |
| `factor N` | Shape-routed full factorization | `vox factor 266175...` |
| `scout N` | Read shape of N and attempt factorization | `vox scout 266175...` |
| `morphism-factor WORD` | Factor using IMASM tape word | `vox morphism-factor "native-numeral-word"` |

#### `ququart_factor` — Ququart Factorization

```
ququart_factor [options] N
```

**Options**:
- `QUQUART_TRACE=1` — Enable tracing
- Environment variables control execution parameters

#### `gpu_glut` — GPU-accelerated GLUT

```
gpu_glut N
```

**Options**:
- `GLUT_GPU_DEVICE` — Select CUDA device
- `GLUT_CHECKPOINT` — Enable checkpointing
- `GLUT_RESUME` — Resume from checkpoint

---

## Configuration

### Environment Variables

#### Dispatch Control

| Variable | Default | Description |
|----------|---------|-------------|
| `GM_TRAP_OFF` | (unset) | Disable arm trap stage (stage 1) |
| `GM_TRAP_K` | 64 | Number of multipliers in trap |
| `GM_TRAP_WINDOW` | 32 | Probe slots per arm in trap |
| `GM_FACTOR_PORT_BOUND` | 48 | Maximum ports in walk before MPQS escalation |
| `GM_FACTOR_TRACE` | (unset) | Enable per-stage timing trace |

#### Performance Tuning

| Variable | Default | Description |
|----------|---------|-------------|
| `VOX_STEPS` | - | Maximum steps for Vox execution |
| `QUQUART_TRACE` | (unset) | Enable Ququart tracing |
| `GLUT_GPU_DEVICE` | 0 | CUDA device index |

#### Debug Flags

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | (unset) | Rust logging level (debug, info, warn, error) |
| `RUST_BACKTRACE` | (unset) | Enable backtraces (0, 1, full) |

### Example Configurations

#### Fast Factorization (MPQS-focused)
```bash
# Maximize MPQS performance
export GM_TRAP_OFF=1  # Skip trap, go directly to MPQS
export GM_FACTOR_PORT_BOUND=48
y_divisor factor 266175155279792434973506259190185380993589458872977057421959357
```

#### Full Trace for Debugging
```bash
# Enable all tracing
export GM_FACTOR_TRACE=1
export QUQUART_TRACE=1
export RUST_LOG=debug
GM_FACTOR_PORT_BOUND=96 y_divisor factor 266175155279792434973506259190185380993589458872977057421959357
```

#### Trap-First Dispatch
```bash
# Default configuration - trap first, then port walk, then MPQS
export GM_TRAP_K=64
export GM_TRAP_WINDOW=32
export GM_FACTOR_PORT_BOUND=48
y_divisor factor 266175155279792434973506259190185380993589458872977057421959357
```

---

## Input Formats

### Supported Input Types

1. **Decimal Natural Numbers** (recommended)
   ```
   y_divisor factor 266175155279792434973506259190185380993589458872977057421959357
   ```

2. **IMASM Tape Words**
   ```
   vox morphism-factor "⊢⊞≻≻⊥⋈⊙⊡⊤⊙≺≻"
   ```

3. **Native Numeral Words**
   ```
   g-momonados native_numeral encode 266175155279792434973506259190185380993589458872977057421959357
   ```

4. **Hexadecimal** (via decimal conversion)
   ```
   # Convert hex to decimal first, or use IMASM encoding
   ```

### Input Size Limits

| Bit Length | Expected Time (208-bit baseline) | Status |
|------------|----------------------------------|--------|
| < 100 bits | < 1 second | ✓ Fast |
| 100-200 bits | 1-30 seconds | ✓ Verified |
| 200-256 bits | 30-90 seconds | ✓ Floor met |
| 256-512 bits | 90-300 seconds | ✓ Measured |
| 512-1024 bits | Minutes to hours | ⚠ Bounded runs |
| > 1024 bits | Hours+ | ❌ Not yet verified |

### Input Validation

The system validates:
- N is a positive integer ≥ 2
- N is odd (even N is rejected immediately)
- N is not a perfect power (detected and handled)
- N is not prime (primality tested before factorization)

---

## Output Interpretation

### Standard Output Format

```
candidate-source=resident-glut producer=mpqs-relation-source quotient=19897152087528681409226432122633 SIC-residual=0 SIC-tolerance=1
Factor: 12558606485192031214854666513391 × 19897152087528681409226432122633
```

### Output Fields Explained

| Field | Description |
|-------|-------------|
| `candidate-source` | Source of the candidate (resident-glut, native, etc.) |
| `producer` | Actual producing arm (mpqs-relation-source, arm-trap-mu-delta, prepared-glut, etc.) |
| `quotient` | Derived quotient N/d |
| `SIC-residual` | Maximum integer residual in SIC reconstruction |
| `SIC-tolerance` | Tolerance threshold for residual acceptance |

### Producer Tags

| Tag | Meaning | Typical Speed |
|-----|---------|---------------|
| `arm-trap-mu-delta` | Lehman-Riesel trap caught factor | Fastest (ms) |
| `dialectic-source` | Dialectic descent found factor | Fast (ms-s) |
| `prepared-glut` | Native GLUT arm | Medium (s) |
| `organism-prepared-glut` | Organism GLUT arm | Medium (s) |
| `mpqs-relation-source` | MPQS relation closure | Medium (s-min) |

### Verification Fields

When Lean validation is enabled (`factors` subcommand):

```
Lean: hasT=true hasN=false hasF=false hasB=false
```

| Field | Description |
|-------|-------------|
| `hasT` | Y/SIXTEEN_3 kernel bridge passed |
| `hasN` | Has native factor (rare) |
| `hasF` | Has full factorization (all factors) |
| `hasB` | Has bounded verification |

---

## Execution Modes

### Mode 1: Trap-First Dispatch (Default)

**Pipeline**:
```
Input N
  ↓
Parity check (even N rejected)
  ↓
1. Arm Trap (Lehman-Riesel winding) — GM_TRAP_K arms, GM_TRAP_WINDOW probes
  ↓ (if miss)
2. Port Walk — Up to GM_FACTOR_PORT_BOUND ports
     ├─ Native GLUT arm (advance_native)
     └─ Streaming Joint arm (advance_joint)
  ↓ (if miss)
3. MPQS Escalation — Full MPQS relation closure
  ↓
Closure Instruments
  ↓
Output
```

**Use Case**: General-purpose factorization, best for unknown N

### Mode 2: MPQS Direct

**Command**: `vox mpqs N`

**Use Case**: Fastest path for large N, bypasses other stages

### Mode 3: Full Validation

**Command**: `y_divisor factors N`

**Use Case**: When Lean Y/SIXTEEN_3 validation is required

### Mode 4: GPU Accelerated

**Command**: `gpu_glut N`

**Use Case**: When CUDA GPU is available

### Mode 5: Anyon Socket

**Setup**: 
```bash
# Start server
bash G-mOMonadOS/measurements/anyon_wiring/run_server.sh

# Submit job
python3 G-mOMonadOS/measurements/anyon_wiring/submit_case.py 208 280
```

**Use Case**: Experimental quantum-inspired factorization

---

## Performance Guidelines

### Performance by Bit Length

| Bit Length | Expected Time | Producer | Notes |
|------------|---------------|----------|-------|
| 200 | ~27s | MPQS | 208-bit measured: 26.23s |
| 208 | ~68s | MPQS | Trap-first: 68.58s |
| 224 | ~182s | MPQS | Trap-first: 182.67s |
| 256 | ~90s bound | MPQS | Reached bound, no factor |
| 895 | ~90s bound | - | Reached bound, no factor |

### Performance Tips

1. **For fast results**: Use `vox mpqs N` directly
2. **For verification**: Use `y_divisor factors N` with Lean validation
3. **For debugging**: Enable `GM_FACTOR_TRACE=1`
4. **For large N**: Increase `GM_FACTOR_PORT_BOUND` (but expect longer times)
5. **For small N**: Use `vox scout N` for quick shape analysis

### Resource Requirements

| Bit Length | RAM | Disk | CPU Cores |
|------------|-----|------|-----------|
| 200-256 | 4-8 GB | 1-2 GB | 4+ |
| 512 | 8-16 GB | 5+ GB | 8+ |
| 1024+ | 16+ GB | 10+ GB | 16+ |

---

## Best Practices

### 1. Always Verify

```bash
# Factor and verify independently
FACTOR=$(y_divisor factor N 2>&1 | grep "Factor:" | awk '{print $2}')
QUOTIENT=$(y_divisor factor N 2>&1 | grep "Factor:" | awk '{print $4}')
python3 -c "print($FACTOR * $QUOTIENT == N)"
```

### 2. Use N-only Inputs

✅ **Do**: 
```bash
y_divisor factor 266175155279792434973506259190185380993589458872977057421959357
```

❌ **Don't**: 
```bash
# Don't supply factors or hints
y_divisor factor 266175... 134192...  # WRONG - supplies factor
```

### 3. Preserve Source Binding

Always use the exact N value. The system maintains:
- Source numeral N
- Prepared register
- Modular lanes
- Banks
- Y continuation
- Clock state
- SIC coordinates

### 4. Separate Preparation and Execution

The system tracks:
- **Preparation time**: State preparation, geometry setup
- **Trigger-to-readout**: From execution start to first result
- **Trigger-to-verified-factor**: From execution start to verified output

These are **separate** and should be reported as such.

### 5. Use Fresh Semiprimes for Testing

For testing, use random balanced semiprimes ≥ 200 bits:

```bash
# Generate a 208-bit semiprime
python3 -c "
from Crypto.Util.number import getPrime
p = getPrime(104)
q = getPrime(104)
N = p * q
print(N)
print(f'# Factors: {p} * {q}')
"
```

**Minimum test domain**: 200-bit near-balanced semiprimes (reaffirmed 2026-10-12)

---

## Troubleshooting

### Common Issues

#### Issue 1: No Factor Found (Timeout)

**Symptoms**:
```
# No output, or timeout reached
```

**Causes**:
- N is prime (not a semiprime)
- N has very close factors (trap might miss)
- Bit length exceeds current capabilities
- Insufficient time bound

**Solutions**:
1. Verify N is composite:
   ```bash
   vox scout N
   ```
2. Try MPQS directly:
   ```bash
   vox mpqs N
   ```
3. Increase time bound:
   ```bash
   VOX_STEPS=1000000000000 vox mpqs N
   ```
4. For large N, use trap-first with higher bounds:
   ```bash
   GM_TRAP_K=128 GM_TRAP_WINDOW=64 GM_FACTOR_PORT_BOUND=96 y_divisor factor N
   ```

#### Issue 2: Build Failures

**Symptoms**:
```
Compilation error
```

**Solutions**:
1. Ensure dependencies:
   ```bash
   sudo apt-get install build-essential cmake pkg-config libssl-dev
   ```
2. Clean and rebuild:
   ```bash
   cargo clean
   cargo build --release --features hosted
   ```
3. Check Rust version:
   ```bash
   rustup update
   rustup default stable
   ```

#### Issue 3: Out of Memory

**Symptoms**:
```
Killed (OOM)
```

**Solutions**:
1. Reduce bit length
2. Use MPQS directly (lower memory footprint)
3. Increase swap space
4. Use machine with more RAM

#### Issue 4: GPU Not Detected

**Symptoms**:
```
No CUDA device found
```

**Solutions**:
1. Install CUDA toolkit:
   ```bash
   sudo apt-get install nvidia-cuda-toolkit
   ```
2. Verify CUDA:
   ```bash
   nvcc --version
   ```
3. Set device:
   ```bash
   GLUT_GPU_DEVICE=0 gpu_glut N
   ```

#### Issue 5: Slow Performance

**Symptoms**:
```
Taking longer than expected
```

**Solutions**:
1. Check producer in output — if it's native GLUT, try MPQS:
   ```bash
   vox mpqs N
   ```
2. Enable tracing to identify bottleneck:
   ```bash
   GM_FACTOR_TRACE=1 y_divisor factor N
   ```
3. For small N (< 150 bits), use scout:
   ```bash
   vox scout N
   ```

---

## Advanced Usage

### Custom Build Configurations

#### Build for Release

```bash
cargo build --release --features hosted
```

#### Build with GPU Support

```bash
cargo build --release --features hosted,cuda
```

#### Build Without Default Features

```bash
cargo build --release --no-default-features --features hosted
```

### Static Binary

```bash
# Build static binary
cargo build --release --features hosted,resident-static

# The binary will be in target/x86_64-unknown-linux-gnu/release/
```

### Running Tests

```bash
# Full test suite (takes ~200s)
cargo test --release --lib

# Specific test
cargo test --release --lib factor_synthesis

# Debug profile tests
cargo test --lib
```

### Benchmarking

```bash
# Use /usr/bin/time for wall clock
/usr/bin/time -v y_divisor factor N

# Or use hyperfine
hyperfine --warmup 3 "y_divisor factor N"
```

### Profile-Guided Optimization

```bash
# Build with profiling
RUSTFLAGS="-C profile-generate=/tmp/profile" cargo build --release

# Run workload
./target/release/g-momonados y_divisor factor N

# Optimize
RUSTFLAGS="-C profile-use=/tmp/profile" cargo build --release
```

---

## Architecture Overview

### System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        G-mOMonadOS                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   y_divisor     │  │  ququart_factor  │  │   gpu_glut      │  │
│  │  (main entry)   │  │  (ququart)       │  │  (GPU)          │  │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  │
│           │                    │                    │             │
│           ▼                    ▼                    ▼             │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    ResidentGlut                              │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────────────┐  │ │
│  │  │ Arm Trap    │  │ Port Walk   │  │ MPQS Escalation        │  │ │
│  │  │ (Stage 1)   │  │ (Stage 2)   │  │ (Stage 3)              │  │ │
│  │  └─────────────┘  └─────────────┘  └───────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────┘ │
│           │                                                        │
│           ▼                                                        │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    Closure Instruments                       │ │
│  │  - Proper divisor gate                                        │ │
│  │  - Parity/product boundary                                    │ │
│  │  - Terminal SIC witness                                       │ │
│  │  - Banked workspace preservation                              │ │
│  │  - Lean Y/SIXTEEN_3 bridge                                    │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Vox                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────────────┐  │
│  │  sieve      │  │ morphism    │  │ dialectic                │  │
│  │  (MPQS)     │  │  (IMASM)    │  │  (GLUT)                  │  │
│  └─────────────┘  └─────────────┘  └───────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
Input: N (decimal or IMASM word)
       ↓
┌──────────────────────┐
│ Prepare Source Carrier│ ← Source numeral N
│ - IMASM word          │
│ - Shared register    │
│ - Banks              │
│ - Y continuation     │
│ - Clock state        │
│ - SIC coordinates    │
└──────────────────────┘
       ↓
┌──────────────────────┐
│ Stage 1: Arm Trap    │ ← Lehman-Riesel winding
│ - mu/delta arms      │
│ - GM_TRAP_K arms     │
│ - GM_TRAP_WINDOW     │
└──────────────────────┘
       ↓ (if miss)
┌──────────────────────┐
│ Stage 2: Port Walk   │ ← GM_FACTOR_PORT_BOUND ports
│ - Native GLUT arm    │
│ - Streaming Joint    │
│   arm                │
└──────────────────────┘
       ↓ (if miss)
┌──────────────────────┐
│ Stage 3: MPQS        │ ← Full MPQS relation closure
│ - Sieve              │
│ - Relation closure  │
└──────────────────────┘
       ↓
┌──────────────────────┐
│ Closure              │ ← Verify and certify
│ - Proper divisor     │
│ - Parity boundary    │
│ - SIC witness        │
│ - Banked workspace   │
│ - Lean validation    │
└──────────────────────┘
       ↓
Output: d × q = N with certificates
```

### Key Modules

| Module | File | Purpose |
|--------|------|---------|
| ResidentGlut | `src/resident_glut.rs` | Main factorization orchestrator |
| FactorSynthesis | `src/factor_synthesis.rs` | Resident state management |
| ArmTrap | `src/arm_trap.rs` | Lehman-Riesel trap implementation |
| QuquartFoldedWork | `src/ququart_folded_work.rs` | WorkBag-based ququart carrier |
| QuquartSymbolic | `src/ququart_symbolic.rs` | Exact reference carrier |
| WholeRegisterPhase | `src/whole_register_phase.rs` | Streaming phase executor |

---

## Receipts and Verification

### Receipt Structure

Every successful factorization produces a receipt with:

1. **Source Binding**: N-only input hash
2. **Producer**: Actual producing arm
3. **Timing**: Preparation, trigger-to-readout, trigger-to-verified-factor
4. **SIC Residuals**: Integer and rational residuals with tolerance
5. **Verification**: Independent product check (d × q == N)
6. **Lean Evidence**: Y/SIXTEEN_3 hasT status

### Verification Commands

```bash
# Verify a factor pair
python3 -c "
N = 266175155279792434973506259190185380993589458872977057421959357
d = 13419294329574030763782528217097
q = 19835257260375005453724574658581
assert d * q == N, f'Product mismatch: {d * q} != {N}'
assert N % d == 0, f'Not a divisor'
print('✓ Verification passed')
"
```

### Historical Receipts

Key verified receipts (as of November 14, 2026):

| N (bits) | Producer | Time | Verification | Receipt File |
|----------|----------|------|--------------|--------------|
| 208-bit | mpqs-relation-source | 26.23s | hasT=true | `receipt_y_lazy_208.json` |
| 208-bit | mpqs-relation-source | 68.58s | hasT=true | `ydiv_208_trapfirst.out` |
| 224-bit | mpqs-relation-source | 182.67s | hasT=true | `ydiv_224_trapfirst.out` |

### Receipt Archive Location

```
G-mOMonadOS/measurements/factor_source_carrier_20261011/
G-mOMonadOS/measurements/anyon_wiring/
```

---

## Appendix

### A. Glossary

| Term | Definition |
|------|------------|
| **IMASM** | Intensional Mathematico-Semiotic Machine — the underlying computational model |
| **GLUT** | Generalized Logical Unification Theory — the primary factorization mechanism |
| **MPQS** | Multiple Polynomial Quadratic Sieve — classical factorization algorithm |
| **SIC** | Symmetric Informationally Complete — positive operator-valued measure |
| **Ququart** | 4-dimensional quantum state (d=4) |
| **Anyon** | Quasiparticle with non-Abelian statistics |
| **Y/SIXTEEN_3** | Lean kernel bridge for monadicity verification |
| **N-only** | Input is solely the source numeral N, no additional information |

### B. File Locations

| Resource | Location |
|----------|----------|
| Main binary | `G-mOMonadOS/target/release/g-momonados` |
| Vox binary | `Vox/target/release/vox` |
| Source code | `G-mOMonadOS/src/` |
| Measurements | `G-mOMonadOS/measurements/` |
| Documentation | `ig-docs/` |
| Catalog | `$HOME/.imscrbgrmr/catalog.json` |

### C. Related Commands

| Command | Purpose | Binary |
|---------|---------|--------|
| `g-momonados` | Main entry point | G-mOMonadOS |
| `vox` | Vox factorization suite | Vox |
| `ququart_factor` | Ququart factorization | G-mOMonadOS |
| `gpu_glut` | GPU GLUT execution | G-mOMonadOS |
| `ququart_prepare_operator` | Operator preparation | G-mOMonadOS |

### D. Version Information

```bash
# Check version
./target/release/g-momonados --version

# Or from git
git -C /home/mrnob0dy666/imsgct/G-mOMonadOS log --oneline -1
```

### E. Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes
4. Run tests: `cargo test --release --lib`
5. Document changes
6. Submit PR with receipts

---

## Support

For issues, questions, or contributions:

1. **Check the AGENTS.md** in the repository root for project-specific guidance
2. **Review existing receipts** in `G-mOMonadOS/measurements/`
3. **Consult documentation** in `ig-docs/`
4. **Verify your build** with `python3 scripts/build_state_guard.py check`

---

## Legal

This software is part of the IMSCRIBER project. All rights reserved.  
For authorized use only.  

**WARNING**: This factorizer is designed for research purposes. Do not use for cryptographic attacks without proper authorization.

---

*Last updated: November 14, 2026*
