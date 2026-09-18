# G-mOMonadOS: $m⊙^{2}$ as a GPU-Native Build

![language](https://img.shields.io/badge/language-Rust-CE422B?style=for-the-badge&logo=rust&logoColor=white)
![tier](https://img.shields.io/badge/tier-O%E2%88%9E-8A2BE2?style=for-the-badge)
![µ∘δ](https://img.shields.io/badge/%CE%BC%E2%88%98%CE%B4-id-00A86B?style=for-the-badge)
![gpu](https://img.shields.io/badge/CUDA-required-76B900?style=for-the-badge&logo=nvidia&logoColor=white)
![license](https://img.shields.io/badge/licence-Unlicense-1A1A1A?style=for-the-badge)

## What This Is

G-mOMonadOS is [mOMonadOS](../mOMonadOS), the self-imscribing bare-metal kernel, ported to a build where the GPU path is the only mode rather than an opt-in feature. Cloned with full history from the parent kernel; every architectural claim in this document (the Crystal, the Frobenius loop, IMASM, the catalog) is the same kernel, the same source tree, the same correctness guarantees - this build just runs it hosted, on a CUDA-capable machine, by default, with no target flag or feature flag needed to get there.

There are no processes, no scheduler, and no filesystem hierarchy. Every execution state is a point in a 17.28-million-entry type space called the Crystal, and storage is navigated by address rather than path. The kernel runs on the 12-opcode IMASM instruction set; each tick executes a single IMASM token, and the grammar constrains what each token does to the current state. Every tick is a self-verification: the Frobenius identity μ∘δ = id is enforced by the grammar rather than by a kernel API.

**Target:** the host (hosted is the default and only mode; see below), requires a CUDA-capable GPU  
**License:** Unlicense (public domain)  
**Relationship to mOMonadOS:** a distinct build, not a fork of the design - see [mOMonadOS/README.md](../mOMonadOS/README.md) for the bare-metal build and the full architecture writeup this document only summarizes.

---

## Building and Running

```bash
cargo build --release   # hosted/GPU is the default feature, no flags needed
./run.sh                 # builds if needed, boots straight to the ⊙> prompt
./run_cmds.sh "gpu_sixteen3_tensor_kernel 1000000000"   # non-interactive
```

## Core Architecture

### The Crystal of Types  

The 12 primitives of the Imscribing Grammar define a type space of 17,280,000 addresses. Every object in the kernel - programs, data structures, witness proofs - is an address in this space. Navigation is by address lookup, not path traversal.

### The Frobenius Loop  

The kernel's main loop is `THINK → ACT → OBSERVE → UPDATE`. Each phase corresponds to IMASM opcodes:  

- **THINK:** Read the boundary (⊢ VINIT)
- **ACT:** Advance and compose (> AFWD, ⋈ CLINK)
- **OBSERVE:** Self-reference and frame (⊙ IMSCRIB, ∈ FSPLIT)
- **UPDATE:** Close and fix (∋ FFUSE, ⊡ IFIX)

Every complete cycle satisfies μ∘δ = id by construction.

### Catalog Integration  

Nine modules from upstream Grammar repositories (imasmic_core, IMSCRIBr, ALEPH_OS, priests-engine) run natively in the kernel. The catalog (`catalog.rs`, 954 lines) is the single source of truth for all data: no hardcoded constants, no ordinal arrays, no glyph strings exist outside it. New systems are registered at runtime via `register_entry()` without source edits.

### Winding One-Shot Operators  

The three prime-number placement operators in `src/` implement the Fixed-Point Nesting Rule (One-Shot #1, ig-docs/exotic_1.md):

- **`oneshot_prime_winder`** (`⊢⊙∈≻⊤⋈≺⊥⊞∋⊡⋈⊙⊣`, period 14): B4 primality via winding period r = ord_N(a). Uses kernel's `winding_period::winding_order` (BSGS on the torus) for u64 inputs - N is prime iff r | (N−1) for all co-prime bases a (Fermat). For arbitrary-precision, falls back to Miller-Rabin. Returns B4 verdicts T/F/B correctly. The original bug (unconditional Miller-Rabin delegation, ignoring the structural word) is fixed.

- **`nested_oneshot`** (`⊢∈≻⊤≺⊥⋈⊙⊞∋⊡⊣`, period 12): B4 verdict + Brent fold/kiss factorization. Word ⊢∈≻⊤≺⊥⋈⊙⊞∋⊡⊣ verified: period 12, phase-bearing, 5 distinct landings, final A, banked OK.

- **`doubly_nested_oneshot`** (`⊢∈⊢∈≻⊤≺⊥⋈⊙⊞∋⊡⊣∋⊣`, period 16): One level deeper than nested_oneshot. Docstring bug PERIOD=17 → 16 fixed (kernel-verified via `imasm cycle`). LANDINGS array corrected to kernel-verified mapping (fixing LANDINGS[2] from "Ftf" to "A"). Full REPL subcommands: word, cycle, verdict, winding, landings, factor. 5 distinct landings: A, Ftf, Ttf, tf, T.

All three words are kernel-verified via `imasm`: weight→final=A, banked=OK, insert→already holds.

### Capabilities

### Topological Quantum Computing  

The kernel braids Fibonacci anyons directly on the metal. The `fibqc` module compiles standard quantum gates to braid words and evaluates knot invariants (Jones polynomial) with no host runtime and no floating-point unit assumed.

### SIC-POVM Implementation  

The d=12 SIC-POVM campaign runs on bare metal via the `d12` REPL command. Five verified pillars:

1. **Phase-tower collapse:** 3→1 independent generators (8× reduction)
2. **Magnitude square-class group:** K₁₆, rank 5
3. **31-orbit Galois structure:** All 143/143 existence-grade overlaps ring-exact
4. **Dual-Link identification:** norm(N₁) = 1/32448², ramification {2,3,13}
5. **Belnap SIC unconditional:** SIC existence proven axiom-free in the Belnap multilattice for d=2ⁿ

### Belnap Paraconsistent Logic  

The Belnap FOUR lattice (T, F, B, N) is the paraconsistent foundation for the entire kernel. The `belnap_c4.rs` module implements a complex plane where i² = B (both-true-and-false), with Frobenius-verified arithmetic. The `belnap_shor.rs` module runs Shor's algorithm on Belnap FOUR, finding that the period r is encoded in the 2:1 coherence cost ratio between B-bias and T-bias.

### Clay Millennium Witnesses  

All seven Clay Millennium Problems are analyzed through the grammar, with IMASM witness programs for:

- **BSD:** Hodge theory witness
- **Hodge:** Mass gap witness  
- **Yang-Mills:** Regularity witness

The `frobenius_unify.rs` module unifies all four Frobenius conditions (kernel, grammar, catalog, SIC) as one machine-checked invariant.

### Red-Hot Rebis Integration  

All 20 modules from `red-hot_rebis/` and `gene_imscriber/` run as no_std Rust off the REPL:

- **p4ra:** Paraconsistent kernel
- **genetic:** Codon ↔ amino acid ↔ glyph translation
- **enzymes:** 109 enzyme tuples with catalytic mechanisms
- **ligand:** Functional group binding design
- **frustration:** Residue-residue energetic frustration matrices

### Cross-Dialect Navigation  

The kernel can navigate between 12 dialects with different structural rulesets, gate thresholds, and absorption rules. The Crystal is invariant; the ruleset is a sheaf that determines what each address *does*. Eleven diaschizic compounds modulate gate thresholds and T-constitution at load time.

### Real x86 Execution  

`vox run <file> [--argv a,b]` lifts a real ELF or PE binary and runs it as an actual process, for real: `vox_core::imasm_module::emit` produces the payload-carrying twelve-glyph module (each glyph plus its actual registers, immediates, and memory operands, not just the bare structural word `weight`/`banked`/`cycle`/`imasm derive` read), and `vox_core::imasm_vm::Machine` interprets it with genuine registers, byte-addressed memory, flags, and ALU semantics. It lays out a real `argv`/`envp`/`auxv` stack the way the psABI guarantees at process entry and runs from the file

### GPU Acceleration

This is the build's whole purpose, not an add-on. `Reg16_3` (four bools over the lanes T, F, t, f) packs into one byte per register on device, and every gate - `union, meet_t, join_t, meet_c, join_c, truth_swap, info_swap, invol, leq_i, leq_t, leq_c, engagr` - runs as a fixed, branch-free per-lane kernel, checked bit-for-bit against the CPU `imasm_core` implementation it is a batched port of.

`gpu16_3 verify` cross-checks all twelve gates against the CPU at up to 10⁹ registers per gate, 12 billion checks, zero mismatches. `gpu_sixteen3_tensor_kernel` runs the trilattice's split/rejoin/chain protocol shape (three real gates chained: union → meet_t → truth_swap) with verification moved entirely onto the device - an independently-written second kernel cross-checks every result via on-device atomic counters, and a fixed 10,000-entry sample is still checked directly against the CPU as ground truth. Final result: 425x wall-clock speedup over a reference (host-verified, host-generated) implementation at n=10⁹; full scaling data and chart in [mOMonadOS/measurements](../mOMonadOS/measurements).

`gpu_crystal_full_space` verifies the entire 17,280,000-address Crystal round-trips on device, cross-checked against 2.16 million independent CPU samples. `gpu_catalog_crystal` and `gpu_imasm_cycle` batch address computation and the full tuple↔word round trip for the live catalog. `gpu_native_cycle` runs the GPU-native-build ob3ect's own protocol word one glyph per tick, a real device action per token. `gpu_ipc_no_serialization` measures direct device-memory access against JSON deserialization on the same catalog data.

Requires a CUDA-capable NVIDIA GPU; there is no CPU-only fallback mode in this build (for that, use [mOMonadOS](../mOMonadOS) directly, where hosted is opt-in).

### REPL Commands  

```
d12              → d=12 SIC-POVM status
d12 tower        → Ray class field tower
d12 verify       → Cross-verification
c4               → Belnap C₄ complex plane
belnap           → Belnap FOUR lattice
stark            → Stark unit extraction
clay             → Clay Millennium status
triple           → von Neumann superoperator algebra
ruleset          → Cross-dialect navigation
fibqc            → Topological quantum computer
vox run <file> [--argv a,b]
                 → run it as a real process: real argv/envp/auxv, real syscalls
vox run <sym> <file> [--args a,b]
                 → call one function directly, no process
gpu16_3 verify [n] [device]
                 → all 12 SIXTEEN_3 gates, batched on GPU, checked vs CPU
gpu_sixteen3_tensor_kernel [n]
                 → chained union->meet_t->truth_swap, split/rejoin protocol shape
gpu_native run/run_real/run_chained/run_cycle [n]
                 → the GPU-native-build ob3ect's protocol word, several ways
gpu_catalog_crystal / gpu_imasm_cycle / gpu_crystal_full_space
                 → catalog and Crystal address-space verification on GPU
gpu_ipc_no_serialization
                 → direct device memory vs JSON deserialization, measured
abc stream [eps] <cutoff...>
                 → finite prefix of the ABC/IUTT measurement stream
abc stream --json [eps] <cutoff...>
                 → stable JSON artifact for Lean certificate tooling
scripts/abc_json_to_lean_certificate.py STREAM.json --check
                 → emit and Lean-check the exact window certificate
scripts/abc_json_to_lean_certificate.py STREAM.json --vox MODULE.so
                 → run the generated ELF through the OS Vox auditor
abc closure [eps] <cutoff...>
                 → attained triples, IUTT packets, and calibration checks
abc window [eps] <cutoff...>
                 → discrepancy maxima and direct scale-link checks
```

### Why This Matters  

**Self-verification:** Every tick satisfies μ∘δ = id by construction, not by testing.

**Grammar-enforced correctness:** The 12-opcode grammar constrains what each token does; there is no undefined behavior. The bare-metal build (`mOMonadOS`) carries this with zero external crates; this build trades that for CUDA and a host runtime in exchange for the GPU path.

**Topological QC on real hardware:** Fibonacci anyon braiding runs without a quantum runtime, and here without leaving the metal for it either - it runs on the same host process as everything else in this build.

**Machine-checked witnesses:** Clay Millennium witnesses are IMASM programs, not prose claims.

**Runtime-extensible:** New systems register at runtime without source edits.

**μ∘δ=id**
