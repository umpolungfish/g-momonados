# G-mOMonadOS / $Gm⊙^{2}$

![Rust](https://img.shields.io/badge/language-Rust-CE422B?style=for-the-badge&logo=rust&logoColor=white)
![GPU](https://img.shields.io/badge/CUDA-GPU%20path-76B900?style=for-the-badge&logo=nvidia)
![License](https://img.shields.io/badge/license-Unlicense-1A1A1A?style=for-the-badge)

$Gm⊙^2$ is the hosted, GPU-native build of the $m⊙^2$ architecture.

It runs the Imscribing Grammar, IMASM words, trilattice state, quantum and
Vox tooling, and ABC/IUTT measurement readers from one Rust executable.

## What This Is

G-mOMonadOS is [mOMonadOS](../mOMonadOS), the self-imscribing bare-metal kernel, ported to a build where the GPU path is the only mode rather than an opt-in feature. Cloned with full history from the parent kernel; every architectural claim in this document (the Crystal, the Frobenius loop, IMASM, the catalog) is the same kernel, the same source tree, the same correctness guarantees - this build just runs it hosted, on a CUDA-capable machine, by default, with no target flag or feature flag needed to get there.

There are no processes, no scheduler, and no filesystem hierarchy. Every execution state is a point in a 17.28-million-entry type space called the Crystal, and storage is navigated by address rather than path. The kernel runs on the 12-opcode IMASM instruction set; each tick executes a single IMASM token, and the grammar constrains what each token does to the current state. Every tick is a self-verification: the Frobenius identity μ∘δ = id is enforced by the grammar rather than by a kernel API.

**Target:** the host (hosted is the default and only mode; see below), requires a CUDA-capable GPU  
**License:** Unlicense (public domain)  
**Relationship to mOMonadOS:** a distinct build, not a fork of the design - see [mOMonadOS/README.md](../mOMonadOS/README.md) for the bare-metal build and the full architecture writeup this document only summarizes.

---

## Building and Running

```bash
cd /home/mrnob0dy666/imsgct/G-mOMonadOS
cargo build --release   # hosted/GPU is the default feature, no flags needed
./run.sh                 # builds if needed, boots straight to the ⊙> prompt
./run_cmds.sh "gpu_sixteen3_tensor_kernel 1000000000"   # non-interactive
```

Local Cargo builds deny Rust warnings (`-D warnings`) through `.cargo/config.toml`.

GPU commands need an exposed CUDA device; CPU and Vox commands work without one.

Verify: `cargo check --features hosted`, `cargo test --features hosted`.

Lean artifacts: `cd ../p4rakernel/p4ramill && lake build`.

Full notes: [`README_FULL.md`](README_FULL.md). Unlicense.

---

## Core Architecture

### The Crystal of Types

The 12 primitives of the Imscribing Grammar define a type space of 17,280,000 addresses. Every object in the kernel - programs, data structures, witness proofs - is an address in this space. Navigation is by address lookup, not path traversal.

Address calculation:
```
address = Σᵢ (index[i] × stride[i])
strides = [5184000, 1728000, 576000, 144000, 48000, 12000, 4000, 800, 200, 50, 10, 1]
```

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

---

## Capabilities

### Topological Quantum Computing

The kernel braids Fibonacci anyons directly on the metal. The `fibqc` module compiles standard quantum gates to braid words and evaluates knot invariants (Jones polynomial) with no host runtime and no floating-point unit assumed.

### SIC-POVM Implementation

The hosted ququart path retains all sixteen SIC projector masses and certifies
dual reconstruction of the complete complex control Gram matrix. Each folded
measurement records the recovered matrix, maximum residual, and fixed-point
rounding tolerance before selecting its computational digit. The baked readout
verifier recomputes those fields from the source-bound frame. `SixteenOutcome`
also exposes the checked lane/native label permutation through `kernel_mask`.

Build the focused reader with `cargo build -j1 --bin sic-tool`. The
`sic-tool anyon-program` command composes the resident carrier operations from
a JSON program on stdin. Its `fourier` action takes an explicit `inverse`
flag. A `rejoin` event emits its Gram matrix and projector masses.
The `clock` action binds a certified retained-negation table to a named
proposition's recorded support and refutation. Each `tick` executes the shared
integer update, checks it against the cached orbit, and selects the coherent
Fourier direction from the resulting phase. `clock_read` checks distant ticks
without unfolding the program again. The retained update swaps runtime t/F
and preserves f/T under the corpus's lane/native equivalence. The same compiled
update can be lifted and executed as a saved Vox IMASM module.
`sic-tool gram-reconstruct` reads a JSON record with `source`, `gram`, and
`masses`, all numeric values as decimal strings, then recomputes the complete
reconstruction certificate. Each Gram entry is a `[real, imaginary]` pair in
the convention `inner_product_work_row_work_col`.
The baked shared-work SIC outcome records both Gram-derived masses and the
rounded branch masses used by its sampler. Their maximum difference is checked
against the source-bound tolerance. Feed the Gram-derived masses to
`gram-reconstruct`; the branch masses remain in the witness so sampling can be
audited independently. `prepare_ququart.py --native-fourier --dynamic-work`
prepares a source-bound factor membrane with the fixed-point Fourier matrix
rechecked at readout. Vox `profile-native` can execute and profile its baked
binary while preserving the factor-producing arm in the terminal report.

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

`vox run <file> [--argv a,b]` lifts a real ELF or PE binary and runs it as an actual process, for real: `vox_core::imasm_module::emit` produces the payload-carrying twelve-glyph module (each glyph plus its actual registers, immediates, and memory operands, not just the bare structural word `weight`/`banked`/`cycle`/`imasm derive` read), and `vox_core::imasm_vm::Machine` interprets it with genuine registers, byte-addressed memory, flags, and ALU semantics. It lays out a real `argv`/`envp`/`auxv` stack the way the psABI guarantees at process entry and runs from the file.

### GPU Acceleration

This is the build's whole purpose, not an add-on. `Reg16_3` (four bools over the lanes T, F, t, f) packs into one byte per register on device, and every gate - `union, meet_t, join_t, meet_c, join_c, truth_swap, info_swap, invol, leq_i, leq_t, leq_c, engagr` - runs as a fixed, branch-free per-lane kernel, checked bit-for-bit against the CPU `imasm_core` implementation it is a batched port of.

- `gpu16_3 verify [n] [device]` → all 12 SIXTEEN_3 gates, batched on GPU, checked vs CPU
- `gpu_sixteen3_tensor_kernel [n]` → chained union->meet_t->truth_swap, split/rejoin protocol shape
- `gpu_native run/run_real/run_chained/run_cycle [n]` → the GPU-native-build ob3ect's protocol word, several ways
- `gpu_catalog_crystal / gpu_imasm_cycle / gpu_crystal_full_space` → catalog and Crystal address-space verification on GPU
- `gpu_ipc_no_serialization` → direct device memory vs JSON deserialization, measured
- `gpu_factor` → Full GPU factorization: parallel trial division first, then ECM/rho/Pollard-Brent on whatever cofactor remains
- `gpu_shor` → Shor's algorithm order-finding on GPU: brute-force verify, direct order lookup, or real baby-step/giant-step

Requires a CUDA-capable NVIDIA GPU; there is no CPU-only fallback mode in this build (for that, use [mOMonadOS](../mOMonadOS) directly, where hosted is opt-in).

---

## IMASM Arithmetic Membranes

The `imasm_*` arithmetic commands take LSB-first numeral tapes, with `⊥` for a one-bit and `⊤` for a zero-bit. For example, `⊥⊥` is three and `⊥⊤⊥` is five. Addition, subtraction, multiplication, restoring division, Euclidean gcd, product closure, and modular exponentiation run as ParaASM instruction streams. The compiler specializes circuit topology to tape widths; the numeral values enter as B4 READ cells and the arithmetic gates, branches, and closures execute inside the IMASM stream.

`imasm_edit_square <x> ` checks the concrete edit/valuation square. Inputs are LSB-first tapes (`⊤=0`, `⊥=1`); the stream verifies the four token insertions, computes `A=x+1`, `D2=x+s`, `B=A+s`, then checks `A+B=x·D2`. Its `C` output is the canonical token word carried by the witness. For `x=s=2`, use `imasm_edit_square ⊤⊥ ⊤⊥`.

The same stream also emits the encoded `g_F` exponent profile for primes `2,3,5`, separately across `(A,B,C)=(3,5,8)` and `(D1,D2,C)=(2,4,8)`, plus the `g_I` supports `rad(3·5·8)=30` and `rad(2·4·8)=2`. `lane_witness` is true only when the dynamic IMASM edit and arithmetic closure succeeds; the profile itself is a baked-in encoded witness for this concrete square, not a general factorization routine. The support coordinates remain distinct: they are not asserted equal.

The arithmetic values and the IUTT four-register state are distinct typed objects. In the Lean kernel, each Θ branch map has type `IUTT.State → IUTT.State`; it does not itself accept a natural number, prime-exponent vector, or radical. Thus this command currently checks the arithmetic/edit membrane, not a theorem identifying its additive and multiplicative arithmetic data with IUTT's state transport.

```text
imasm_mul ⊥⊥ ⊥⊤⊥          # 3 × 5
imasm_divmod ⊥⊥⊥⊥ ⊥⊥      # 15 ÷ 3
imasm_gcd ⊥⊥⊥⊥ ⊥⊤⊥        # gcd(15, 5)
imasm_close ⊥⊥ ⊥⊤⊥ ⊥⊥⊥⊥ # close 3 × 5 = 15
imasm_powmod ⊤⊥ ⊤⊥⊤⊥ ⊥⊤⊤⊤⊥ # 2^10 mod 17
```

---

## REPL Command Reference

Commands are organized by functional area. Each REPL command is admitted into the recursively enclosed IMASM process vessel, executes at its inner action leaf, and emits through its terminal surface.

### Core System
- `help` → Help system (help <topic> for details)
- `whoami` → IG tuple under the active ruleset
- `ruleset` → show the active ruleset
- `exit/quit` → Leave the REPL

### Grammar & Crystal
- `ig` → IG tuple + crystal address
- `classify` → Nearest-catalog classification
- `frob` → Frobenius harness status
- `aleph` → Hebrew glyph encoding: aleph <word>
- `crystal` → Crystal FS (decode, store, find, name)
- `crystal-scope` → Substitution microscope: distance, tier, dS, gate jump, and the measured driver (alias cscope)

### Mathematics & Logic
- `algebra` → distance|meet|join|tensor vs ZFC
- `c4` → Belnap C₄ complex plane (i²=B)
- `cscore` → Consciousness score (dual-gate)
- `sigma` → sigma <n> — analyze the Sigma(n) divisor ring
- `join` → join of the active IG tuple with the ZFC baseline
- `distance` → Hamming + weighted distance vs the ZFC baseline tuple (alias dist)
- `ringspec` → ringspec <w1> <w2> <w3> — the spectrum of a ring, in integers

### Constants & Measurement
- `constants` → MoDoT constant closure
- `ovm` → OVM Computation Tools
- `abc` → Real arithmetic from the ABC/IUTT Lean proofs: radical, discrepancy, quality, window maximum, and a real scale-link check between window sizes
- `substrate` → closure constant, content bifurcating

### Quantum & Topological
- `fibqc` → Fibonacci anyon QC: verify | compile | jones | knot | winding
- `qc` → Compile a circuit over H T S X to a braid word; spaces optional; draw|svg|loop before the gates renders it, and two depths size the net and the recursion (aliases quantum_compile, fibqc compile)
- `bi` → Draw a braid word — strand diagram in the terminal, SVG with `svg`, the closed braid as a ring with `loop`; window with start:count, column height with /N (alias braid_image)
- `jp` → Jones polynomial at the 1/5 winding; signed Artin generators (alias jones_polynomial)
- `bg` → Braid word to grammar tuple (alias braid-grammar); winding is a closed form in the writhe
- `shor` → Belnap Shor pipeline + dialetheic Fibonacci Shor (word ⊢∈≻⋈⊞∈⊤≻⊥≺∋⊙⋈⊡⊣); N=15,21
- `qft` → Quantum Fourier Transform: circuit | phases | iqft | iqft braid | braid, on n qubits
- `d12` → d=12 SIC-POVM status
- `d12 tower` → Ray class field tower
- `d12 verify` → Cross-verification
- `dqi` → Decoded Quantum Interferometry operator: word | period | phase | verdict <arm> | syndrome <bits> | tuple | report
- `iuft` → IUFT QC gates — the 12->3 Euler-angle SU(2) encoding of an IG tuple
- `teich` → IUFT <-> IUTT bridge: Teichmuller deformation paths as gate trajectories
- `hqe` → Holonomic quasi-ergodic quantale, MBL holonomy
- `dyson` → Dyson beta-ensemble, double-ramified cycle
- `troq` → Triple-ramified ouroboric quantale
- `afdmc` → Asymptotic frozen-disordered monadic cohomology
- `sic` → SIC-POVM d=12 identity, three lattice proofs
- `d2048` → d=2048 moduli tower ascent (alias d2k)

### IMASM Operations
- `imasm` → IMASM word walks (cycle, weight, banked, insert, trans, arev)
- `cycle` → walk an IMASM word around its ROTAT orbit (glyphs only)
- `weight` → where the weight moves through an IMASM word
- `banked` → was a count cleared with nothing banked?
- `insert` → every one-glyph repair for an exposed word
- `trans` → transitions counted on the ring, closing edge included
- `arev` → H hop: read snapshot through the R1<->R2 mirror

### Kernel & System
- `ask` → kernel structural ask (dry). Full wet: host ./ask --file | -i
- `spine` → manuscript spine: PROVE->UNIFY->PORT x vessel (no Python)
- `vessel` → witness-vessel transport: Clay payloads x 88 dialects, frob-gated
- `vita` → one certified turn from the on-board vae_vita trunk
- `tstatus` → T-constitution pass/fail

### Membranes & Factorization
- `factor_membrane` → Factor-Separating Imscription Membrane: D2 lane split, N_lane DEFINED as P_lane*Q_lane (aliases: membrane, fmembrane)
- `shor_b4` → Key membrane finding: shors_algorithm x b4_factor_v2_engine click (T<->H, certified) + membrane family (aliases: shor_b4_membrane, key_membrane)
- `membrane_family` → Membrane family registry: list/run factor membranes by word (alias: mfam)
- `doubly_nested_oneshot` → Winding-order factoring, nested two levels deep (alias: dnos)
- `nested_oneshot` → Winding-order factoring, nested one level (aliases: nested, nos)
- `multilattice` → The FDE/QM boundary as real code: the corrected Pauli-algebra WH action, orbit 4^n, zero axioms
- `trilattice_factor` → Trilattice factoring toolkit — read/sieve/factor/dialect-probe/gpu-verify, with the number's own crystal address (alias: tfactor)
- `factor_operator` → Monotone factor-state constructor F_N over 𝟒={N,T,F,B}: cert certifies a pair, ambient states the moat (free-bit paradox), resolve strips the small part and crosses the moat to return factors (any width)
- `native_numeral` → The word-native numeral toolkit itself: encode/decode/factor/decompose/redstep/cycle and the rest (alias: numeral)
- `phase_unbraid` → Phase-based unbraider: the factors come out of a QFT phase readout (winding k/M -> period -> one gcd closure), no search
- `dyn_nest` → Optimal oneshot nesting depth: the least depth at which the Brent cycle closes on a factor (aliases: dyn, dynamic_nest)
- `prime_winding` → Winding period of the primes on the number line - ob3ect-backed: find | factor | cycle | tuple | verdict
- `oneshot_prime_winder` → One-shot primality test using IMASM word ⊢∈≻⊤⋈⊙≺⊥⊞∋⊡⊣ winding certificate
- `dyn_nest` → Dynamic Nesting Prime Finder - pipes oneshot verdict, searches optimal nesting depth d=1,2,3,...; period P(d)=5d+7, closure-derived seed

### Programs & Execution
- `programs` → Program loading (list, canonical, continuous, novel, shunt)
- `exec` → Execution (run, tick, watch, timer, boot)
- `status` → Status (program, snapshot, graph, heatmap, registers)
- `tick [N]` → Run N kernel ticks (default 1). Each tick is one full THINK→ACT→OBSERVE→UPDATE cycle.
- `run [N]` → Run N additional ticks from the current position. Unlike `tick`, `run` is the continuous execution path — use it when you want the kernel to evolve without watching each step.
- `continuous <1-4>` → Load one of the 4 continuous programs (XIII–XVI). Resets IP to 0.
- `novel <1-3>` → Load one of the 3 novel programs (XVII–XIX). Resets IP to 0.
- `shunt <0-8>` → Load one of the 9 shunted programs (XX–XXVIII) by index. Resets IP to 0.
- `canonical <I–XII>` → Load one of the 12 canonical programs by Roman numeral. Resets IP to 0.

### Memory & Registers
- `memory [start] [count]` → Dump B4 memory cells as N/T/F/B. Default: 16 cells from address 0.
- `registers` → Show R0–R7 as B4 values.
- `stack` → Show current stack depth.

### Cross-Dialect
- `ruleset show` → Active ruleset display
- `ruleset list` → List all 88 dialects
- `ruleset verify` → Invariant violation check
- `jump` → Cross-dialect jump: jump <U> using <c>
- `seal` → IFIX commit to current ruleset
- `whoami --ruleset` → IG tuple under active ruleset

### Absorption & Compounds
- `absorption` → list all absorption rules
- `replicative` → load the program targeting O_inf_dag (R2) deliberately
- `absorb_test` → Test absorption rule
- `absorption show` → List absorption rules
- `compound list` → List 11 diaschizic compounds
- `compound` → compound show|load <name>

### Proof & Verification
- `proof list` → List available guided proofs
- `proof bootstrap` → The Grammar verifying itself (7 steps, auto-play)
- `prooflift` → Proof-lift report: undischarged claims and unrejoined forks as one object; `prooflift nest` runs the self-nest word, the proof of mu.delta=id itself (86065 glyphs, verdict T)
- `cr3` → Theorem engine (Collatz, Goldbach, Three-Body, Burnside, ...)
- `p4ra` → p4rakernel Belnap+Frobenius 13-step bootstrap
- `seals list` → List all 10 sealed proofs
- `seals fine-structure` → α⁻¹ = d²−7 + arctan(1/4)/(4√3) + α²·d (3 steps)
- `seals proton` → m_p/m_e = d³ + d(d−3) + α-dressing (2 steps)
- `seals lepton` → m_μ/m_e (exact rational), m_τ/m_e (2 steps)
- `seals boson` → m_W, m_Z, m_H — π + ω forms (2 steps)
- `seals gravity` → α_G = α¹⁸·√3 (1 step)
- `seals weinberg` → sin²θ_W = 3/13 (exact rational, 1 step)
- `seals cosmology` → ρ_Λ/ρ_Pl = e^{-44ω}/744 (1 step)
- `seals neutrino` → m₁:m₂:m₃ = 1:4:16 (1 step)
- `seals winding` → ω = 2π — all angles in windings (1 step)
- `seals residuals` → Where every remainders comes from (1 step)
- `seals all` → GRAND SEAL — walk through all 10

### Experimentation & Analysis
- `demonstrate` → Run a claim as an experiment: INPUT/OPERATION/OUTPUT/CHECK, computed live (alias demo)
- `witness` → Smallest executable object standing behind a claim
- `counterfactual` → Perturb one glyph
- `basin` → Fixed-point archaeology
- `ouroboros-inverse` → Inverse grammar
- `frobenius-fuzzer` → Mine the word space for programs the braid reproduces exactly (alias fuzz)
- `oracle` → Adversarial
- `blackbox` → Infer a law from integer observations, ranked by fit minus complexity
- `dialetheic-compiler` → Lift a classical gate into Belnap FOUR
- `stark-geometer` → SIC Stark arithmetic for dimension d: m_d, unit, ramified primes
- `dialect-necromancer` → Imscribe a fragment and recover its nearest catalog ghost
- `braid-apocrypha` → Search braid words for a target Jones magnitude; first hit is shortest
- `proof-braider` → Lift a claim to a braid and back; PASS iff Frobenius closure survives
- `universe-wormhole` → Minimum gate-space path between dialects, as a braid + Jones
- `vox-ce` → Lift EVM/WASM hex into an IMASM word and verdict its control-flow closure
- `consciousness-lath` → Single-axis mutation that most raises the C-score with both gates open
- `paradox-engine` → Hunt words that are dialetheias by four readings at once (B, price, gate1, C=0)
- `compiler` → Compile a braid to imasm/jones/lean, or a token word back to a braid
- `catalogue` → Synthesize candidate operators; rank by novelty against the catalog
- `museum` → The permanent collection of failed constructions — append-only negative knowledge
- `phase` → Phase as an object: orbit spectrum, phase period, and two-word interference
- `loss` → What a transformation destroys: entropy in/out, bits destroyed, irreversible transitions
- `shadow` → Ontological nearest-neighbour: shared structure and the measured critical difference
- `provenance` → Epistemic type check: dependency DAG graded by lattice MEET, not by best sibling (alias prov)
- `ctc-loom` → Sweep the six Belnap actions over the whole word space; rank closures by price (alias loom)
- `cl9nk` → CLINK L9, the replicative lateral: d(L8,L9) and the ladder read from L9
- `minimal` → Shortest word achieving a target property
- `repair` → Ranked program/proof surgery with a proof-diff
- `mersearch` → Mersenne search: run|ll. Composite exponents answer at once (alias msearch)
- `fde` → FDE(n) tower navigation: embed | restrict | walk | roundtrip | trans | report — ascend/descend the truth-value lattice at any depth
- `combo` → cycle a word, then run weight | banked | insert | repair on every distinct rotation it produces, formatted as one report; add 'brief' for repair's cheapest candidate only
- `combo2` → combo (brief) on a word, then weight | banked | insert | repair again on every distinct word the first pass's repairs produced
- `millennium` → run weight | banked | insert on a Millennium conjecture's promotion word and print its live executed crystal address and tuple; no argument lists the seven names, 'all' runs every one
- `entropy` → entropy experiment: dS vs tier promotion
- `invariant` → Discover invariants under transformations: ROTAT, IMSCRIB, FSPLIT/FFUSE
- `redteam` → Adversarial testing
- `ctc` → nest a value in an action
- `collatz` → the Collatz block nesting
- `straus` → the Erdős–Straus ladder
- `nesting` → read a point against a map
- `carriers` → census of the mu-delta=id carriers by class
- `klines` → KLINES analysis
- `klines2` → KLINES analysis v2
- `yz` → Yamakawa-Zhandry verifiable-quantum-advantage retranslation (r/c/Inc) run as real code, not read as a document
- `yz_list` → Theorem 11.1's L-list mechanism from the YZ-retranslation, run for real rather than left open
- `anyon-sync` → Dialetheic FOUR-valued Carrier16 register walked over THE_WORD, showing where the T/F/t/f lanes sync (alias: anyon_sync)
- `gaussian_extract` → Fermat two-square extraction: a real root of -1 mod p, then Cornacchia descent to p=a²+b² (alias: gaussian)

### Grounded Operations (Prime Winding)
- `prime_winding grounded_add` → a+b
- `prime_winding grounded_mul` → a*b
- `prime_winding grounded_sub` → a-b
- `prime_winding grounded_mod` → a mod b
- `prime_winding grounded_divmod` → a=q*b+r 
- `prime_winding grounded_gcd` → gcd(a,b)
- `prime_winding grounded_factor`

### GPU-Specific
- `gpu_native` → The literal GPU-native build protocol, run for real, not simulated
- `gpu16_3` → Batch SIXTEEN_3 register gates on GPU: many registers at once, verified against the scalar path
- `gpu_gnfs` → Grammar-native GNFS on GPU — FB + sieve + GF(2) + φ-congruence (multi-limb) on device
- `gpu_rho` → GPU Pollard's rho, fixed 256-bit Montgomery width, verified against BigUint (verify | prims | factor)
- `gpu_rho_ml` → Multi-limb GPU Pollard's rho beyond gpu_rho's 256-bit cap, any width up to 2048 bits
- `gpu_ecm` → Lenstra elliptic-curve factorization on GPU at any width, one Montgomery curve per thread across every device
- `gpu_factor` → Full GPU factorization: parallel trial division first, then ECM/rho/Pollard-Brent on whatever cofactor remains
- `gpu_shor` → Shor's algorithm order-finding on GPU: brute-force verify, direct order lookup, or real baby-step/giant-step
- `gpu_dqi` → DQI min-weight coset search on GPU, checked against the CPU path
- `gpu_fde` → FDE tower theorems checked on GPU against the CPU functions
- `gpu_kernel` → Execute or verify an IMASM program on GPU, checked against kernel::self_imscribe
- `gpu_millennium` → Static self-imscription of the Millennium-conjecture ob3ects on GPU, checked against the CPU kernel
- `gpu_opi` → OPI Prange trial arithmetic on GPU, checked against the CPU path
- `gpu_vox` → vox's control-flow closure verdict run on GPU, checked against the CPU auditor
- `gpu_catalog_crystal` → 
- `gpu_crystal_full_space` → 
- `gpu_imasm_cycle` → 
- `gpu_ipc_no_serialization` → 
- `gpu_sixteen3_tensor_kernel` → 

### Legacy & Compatibility
- `triple` → Triple-frame von Neumann superoperator algebra
- `manifold` → Topological manifold operations
- `hop` → Universe hopping, cross-framework transport
- `temp` → Temporal logic bridge
- `cat` → Category theory bridge
- `ym` → Yang-Mills mass gap bridge
- `rh` → Riemann Hypothesis bridge
- `psm` → dialetheic alignment + measurement tests
- `psm test` → Dialetheic alignment + measurement
- `psm frob` → Frobenius identity cycle
- `psm kernel` → Kernel-state B3 invariant loop
- `psm load` → Inline ParaASM program (; separator)
- `oneshots` → the 10 exotic fixed-point nestings
- `ctc` → nest a value in an action

---

## Repository Map

| Path | Contents |
|---|---|
| `src/` | Rust runtime, REPL, trilattice, ABC, GPU, quantum modules |
| `scripts/` | JSON-to-Lean certificate tooling and audits |
| `docs/` | command and certificate documentation |
| `measurements/` | GPU benchmark records |
| `ob3ects/` | Generated ob3ect artifacts |
| `examples/` | Example programs and use cases |

---

## Verification

**Self-verification:** Every tick satisfies μ∘δ = id by construction, not by testing.

**Grammar-enforced correctness:** The 12-opcode grammar constrains what each token does; there is no undefined behavior. The bare-metal build (`mOMonadOS`) carries this with zero external crates; this build trades that for CUDA and a host runtime in exchange for the GPU path.

**Topological QC on real hardware:** Fibonacci anyon braiding runs without a quantum runtime, and here without leaving the metal for it either - it runs on the same host process as everything else in this build.

**Machine-checked witnesses:** Clay Millennium witnesses are IMASM programs, not prose claims.

**Runtime-extensible:** New systems register at runtime without source edits.

---

## The Strange Loop

What makes this system Gödel-complete here is the STRANGE LOOP. The 49 types judge programs; the types ARE programs; the judge's verdict is itself expressible as a word the same tools would judge. There is no metalanguage outside IMASM from which to describe IMASM, because the description would be another word, and the same engine would judge it.

The loop closes over three domains at once: a compiled function, a .pyc sequence, an EVM blob, a gene — each is a word the language already speaks, read off the substrate that carries it (V⊙x). A number — any number — is a word the language already speaks, built from its own bits and computed on by its own kernel (the native numeral). The same engine that judges your reasoning at the prompt judges the code on disk and the arithmetic on the numeral.

---

$\mu\circ\delta = \mathrm{id}$
