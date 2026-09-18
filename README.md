# G-mOMonadOS

![Rust](https://img.shields.io/badge/language-Rust-CE422B?style=for-the-badge&logo=rust&logoColor=white)
![GPU](https://img.shields.io/badge/CUDA-GPU%20path-76B900?style=for-the-badge&logo=nvidia)
![License](https://img.shields.io/badge/license-Unlicense-1A1A1A?style=for-the-badge)

G-mOMonadOS is the hosted, GPU-native build of the mOMonadOS architecture.
It runs the Imscribing Grammar, IMASM words, trilattice state, quantum and
Vox tooling, and ABC/IUTT measurement readers from one Rust executable.

The central runtime loop is:

```text
THINK → ACT → OBSERVE → UPDATE
```

The hosted runtime enters through one recursively enclosed IMASM process vessel.
Each REPL command is admitted into that vessel, executes at its inner action
leaf, and emits through its terminal surface. The Crystal is the Grammar's
finite address space; the trilattice retains truth, falsity, information, and
held states. The GPU path batches the same gate operations as the CPU reference.

## Start here

```bash
cd /home/mrnob0dy666/imsgct/G-mOMonadOS
make hosted
./run_cmds.sh 'help'
./run.sh   # interactive session
```

GPU commands need an exposed CUDA device; CPU and Vox commands work without one.

## Useful commands

```text
help
fibqc help
vox help
gpu16_3 verify [n] [device]
gpu_sixteen3_tensor_kernel [n]
gpu_crystal_full_space
abc stream [eps] <cutoff...>
abc closure [eps] <cutoff...>
abc champions [eps] <max_c>
ig                     IG tuple + crystal address
classify               Nearest-catalog classification
frob                   Frobenius harness status
aleph                  Hebrew glyph encoding: aleph <word>
rh                     Riemann Hypothesis bridge
ym                     Yang-Mills mass gap bridge
temp                   Temporal logic bridge
cat                    Category theory bridge
algebra                distance|meet|join|tensor vs ZFC
cl8nk                  CLINK Layer 8: cl8nk <action> [name]
c4                     Belnap C₄ complex plane (i²=B)
cscore                 Consciousness score (dual-gate)
constants              MoDoT constant closure: fine-structure, proton-electron, lepton, boson, gravity
ovm                    OVM Computation Tools
oneshots               the 10 exotic fixed-point nestings: inner already at outer's fixed point
ctc                    nest a value in an action; closure imposed where the action has none, priced by the width it smears
collatz                the Collatz block nesting: blocks to one, the budget spectrum, and the records
straus                 the Erdős–Straus ladder: which rung r closes 4/n, and the spectrum across a range
nesting                read a point against a map: q=r2/r1 splits attracted from never-arrives where one gap cannot
carriers               census of the mu-delta=id carriers by class: one fixed point seen many ways, or a family
substrate              closure constant, content bifurcating: the conservative substrate read on both observables
stark                  Stark unit extraction: formula,fibqc,tower,exponents,verify
riemann                Riemann-SIC report; sub-actions available
distance               Hamming + weighted distance vs the ZFC baseline tuple (alias dist)
join                   join of the active IG tuple with the ZFC baseline
sigma                  sigma <n> — analyze the Sigma(n) divisor ring
ringspec               ringspec <w1> <w2> <w3> — the spectrum of a ring, in integers: bond weights around a cycle, clean bond 1, cross-link its reaction centres; three is the minimum
clay                   Clay Millennium structural status (machine-checked)
psm                    dialetheic alignment + measurement tests
entropy                entropy experiment: dS vs tier promotion
invariant              Discover invariants under transformations: ROTAT, IMSCRIB, FSPLIT/FFUSE
redteam                Adversarial testing: analyze|stress|mutate, and audit <theory> for hidden assumptions
witness                Smallest executable object standing behind a claim
counterfactual         Perturb one glyph: invariants held/broken, reversibility, smallest repair (alias cf)
basin                  Fixed-point archaeology: orbit, attractor, transient depth, exact basin size
ouroboros-inverse      Inverse grammar: shortest IMASM word imscribing a tuple, plus its braid (alias oinv)
frobenius-fuzzer       Mine the word space for programs the braid reproduces exactly (alias fuzz)
oracle                 Adversarial: hunt the cheapest structural counterexample; surviving is not proof
blackbox               Infer a law from integer observations, ranked by fit minus complexity
dialetheic-compiler    Lift a classical gate into Belnap FOUR; show where a row rests on a paradox
stark-geometer         SIC Stark arithmetic for dimension d: m_d, unit, ramified primes
dialect-necromancer    Imscribe a fragment and recover its nearest catalog ghost
braid-apocrypha        Search braid words for a target Jones magnitude; first hit is shortest
proof-braider          Lift a claim to a braid and back; PASS iff Frobenius closure survives
universe-wormhole      Minimum gate-space path between two hop frameworks, as a braid + Jones
vox-ce                 Lift EVM/WASM hex into an IMASM word and verdict its control-flow closure
consciousness-lath     Single-axis mutation that most raises the C-score with both gates open
paradox-engine         Hunt words that are dialetheias by four readings at once (B, price, gate1, C=0)
key-dissolver          SIC-narrowed bounded window before a BSGS split
compiler               Compile a braid to imasm/jones/lean, or a token word back to a braid
catalogue              Synthesize candidate operators; rank by novelty against the catalog
sk_forge               Crystal Harvester: BIP39-SIC integrated structural gap analysis against O_∞ carriers. Commands: forge, tuple, word, verify, carriers, bip39-sic, bip39-pipeline (alias sk-forge)
museum                 The permanent collection of failed constructions — append-only negative knowledge
phase                  Phase as an object: orbit spectrum, phase period, and two-word interference
demonstrate            Run a claim as an experiment: INPUT/OPERATION/OUTPUT/CHECK, computed live (alias demo)
loss                   What a transformation destroys: entropy in/out, bits destroyed, irreversible transitions
shadow                 Ontological nearest-neighbour: shared structure and the measured critical difference
provenance             Epistemic type check: dependency DAG graded by lattice MEET, not by best sibling (alias prov)
ctc-loom               Sweep the six Belnap actions over the whole word space; rank closures by price (alias loom)
cl9nk                  CLINK L9, the replicative lateral: d(L8,L9) and the ladder read from L9
crystal-scope          Substitution microscope: distance, tier, dS, gate jump, and the measured driver (alias cscope)
minimal                Shortest word achieving a target property
repair                 Ranked program/proof surgery with a proof-diff
mersearch              Mersenne search: run|ll. Composite exponents answer at once (alias msearch)
pk2sk                  PK→SK recovery: bounded-range ECDLP on secp256k1 — recover the scalar in [lo, hi) from its compressed public key, curve-gated, imscribed
fde                    FDE(n) tower navigation: embed | restrict | walk | roundtrip | trans | report — ascend/descend the truth-value lattice at any depth
rsa                    RSA decrypter via BSGS period-finding on ord_N(C): word | period | verify | <C> <N> <e> — only closes when that order is small, not for real RSA moduli
combo                  cycle a word, then run weight | banked | insert | repair on every distinct rotation it produces, formatted as one report; add 'brief' for repair's cheapest candidate only
combo2                 combo (brief) on a word, then weight | banked | insert | repair again on every distinct word the first pass's repairs produced
millennium             run weight | banked | insert on a Millennium conjecture's promotion word and print its live executed crystal address and tuple; no argument lists the seven names, 'all' runs every one
fibqc                  Fibonacci anyon QC: verify | compile | jones | knot | winding (see also qc, jp)
qc                     Compile a circuit over H T S X to a braid word; spaces optional; draw|svg|loop before the gates renders it, and two depths size the net and the recursion (aliases quantum_compile, fibqc compile)
bi                     Draw a braid word — strand diagram in the terminal, SVG with `svg`, the closed braid as a ring with `loop`; window with start:count, column height with /N (alias braid_image)
jp                     Jones polynomial at the 1/5 winding; signed Artin generators (alias jones_polynomial)
bg                     Braid word to grammar tuple (alias braid-grammar); winding is a closed form in the writhe
shor                   Belnap Shor pipeline + dialetheic Fibonacci Shor (word ⊢∈≻⋈⊞∈⊤≻⊥≺∋⊙⋈⊡⊣); N=15,21
shors_btc_2            Shor over secp256k1 ECDLP: recover a Bitcoin private key from a public key (x,y)
prime_winding          Winding period of the primes on the number line - ob3ect-backed: find | factor | cycle | tuple | verdict
oneshot_prime_winder   One-shot primality test using IMASM word ⊢∈≻⊤⋈⊙≺⊥⊞∋⊡⊣ winding certificate
dyn_nest               Dynamic Nesting Prime Finder - pipes oneshot verdict, searches optimal nesting depth d=1,2,3,...; period P(d)=5d+7, closure-derived seed
qft                    Quantum Fourier Transform: circuit | phases | iqft | iqft braid | braid, on n qubits
btc_oneshot            BTC Secret Key Oneshot Operator — structural verification & phase steps
secp256k1_unwinder     19-glyph morphism sequence for secp256k1 scalar recovery: word | steps | mapping | walk [k] | verdict [k] | tuple | constants
baryon_asymmetry       baryon asymmetry as a banked survival: the word run through the live weight + banked instruments (report | word | mapping | reading)
theta-link             IUTT housed in the paraconsistent ambient: the Θ-link closes yet holds register A (four-valued B, the Inclosure), unreachable by the Boolean adjoints (alias iutt)
winding                Period as a torus winding: order | factor | closure | factorgen (alias wperiod)
iuft                   IUFT QC gates — the 12->3 Euler-angle SU(2) encoding of an IG tuple
teich                  IUFT <-> IUTT bridge: Teichmuller deformation paths as gate trajectories
hqe                    Holonomic quasi-ergodic quantale, MBL holonomy
dyson                  Dyson beta-ensemble, double-ramified cycle
troq                   Triple-ramified ouroboric quantale
afdmc                  Asymptotic frozen-disordered monadic cohomology
hop                    Universe hopping, cross-framework transport
manifold               Topological manifold operations
triple                 Triple-frame von Neumann superoperator algebra
sic                    SIC-POVM d=12 identity, three lattice proofs
bip39                  BIP39-SIC-POVM: search | words | verify | map | gap
d12                    d=12 SIC Phase VI: tower, magnitudes, orbits, existence, duallink, z0
d2048                  d=2048 moduli tower ascent (alias d2k)
dqi                    Decoded Quantum Interferometry operator: word | period | phase | verdict <arm> | syndrome <bits> | tuple | report
cycle                  walk an IMASM word around its ROTAT orbit (glyphs only)
weight                 where the weight moves through an IMASM word
banked                 was a count cleared with nothing banked?
insert                 every one-glyph repair for an exposed word
trans                  transitions counted on the ring, closing edge included
arev                   H hop: read snapshot through the R1<->R2 mirror
ask                    kernel structural ask (dry). Full wet: host ./ask --file | -i
spine                  manuscript spine: PROVE->UNIFY->PORT x vessel (no Python)
vessel                 witness-vessel transport: Clay payloads x 88 dialects, frob-gated
vita                   one certified turn from the on-board vae_vita trunk
whoami                 IG tuple under the active ruleset
ruleset                show the active ruleset
absorption             list all absorption rules
replicative            load the program targeting O_inf_dag (R2) deliberately
vox                    Control-flow closure auditor
ruleset show           Active ruleset display
ruleset list           List all 88 dialects
ruleset verify         Invariant violation check
jump                   Cross-dialect jump: jump <U> using <c>
seal                   IFIX commit to current ruleset
whoami --ruleset       IG tuple under active ruleset
tensor                 Tensor under active absorption
meet                   Meet under active absorption
absorb_test            Test absorption rule
absorption show        List absorption rules
tstatus                T-constitution pass/fail
compound list          List 11 diaschizic compounds
compound               compound show|load <name>
psm test               Dialetheic alignment + measurement
psm frob               Frobenius identity cycle
psm kernel             Kernel-state B3 invariant loop
psm load               Inline ParaASM program (; separator)
cr3                    Theorem engine (Collatz, Goldbach, Three-Body, Burnside, ...)
p4ra                   p4rakernel Belnap+Frobenius 13-step bootstrap
cr3 --version          cr3 version info
cr3 --list             List theorems + p4rakernel modules
seals list             List all 10 sealed proofs
seals fine-structure   α⁻¹ = d²−7 + arctan(1/4)/(4√3) + α²·d (3 steps)
seals proton           m_p/m_e = d³ + d(d−3) + α-dressing (2 steps)
seals lepton           m_μ/m_e (exact rational), m_τ/m_e (2 steps)
seals boson            m_W, m_Z, m_H — π + ω forms (2 steps)
seals gravity          α_G = α¹⁸·√3 (1 step)
seals weinberg         sin²θ_W = 3/13 (exact rational, 1 step)
seals cosmology        ρ_Λ/ρ_Pl = e^{-44ω}/744 (1 step)
seals neutrino         m₁:m₂:m₃ = 1:4:16 (1 step)
seals winding          ω = 2π — all angles in windings (1 step)
seals residuals        Where every remainders comes from (1 step)
seals all              GRAND SEAL — walk through all 10
fold                   The fold verdict of a word — closed form, surplus, the enclosure witness, the codon lane
erdos                  Guided walks through the Erdős manuscripts — list | schutte | landau | lcm
proof list             List available guided proofs
proof bootstrap        The Grammar verifying itself (7 steps, auto-play)
prooflift              Proof-lift report: undischarged claims and unrejoined forks as one object; `prooflift nest` runs the self-nest word, the proof of mu.delta=id itself (86065 glyphs, verdict T)
opi                    DQI's Optimal Polynomial Intersection: exact Lemma 9.2 eigenvalue + m to p asymptotic, settable wall-clock budget
weight_ladder          The general reduction behind opi's eigenvalue, for any m-exchangeable-trial Hamming-weight ladder, not just OPI
nested_prime_factorization 16-morphism factorization tower, IFIX banks the complete factor record at the END (non-vacuous) (alias: npf)
factor_membrane        Factor-Separating Imscription Membrane: D2 lane split, N_lane DEFINED as P_lane*Q_lane (aliases: membrane, fmembrane)
shor_b4                Key membrane finding: shors_algorithm x b4_factor_v2_engine click (T<->H, certified) + membrane family (aliases: shor_b4_membrane, key_membrane)
membrane_family        Membrane family registry: list/run factor membranes by word (alias: mfam)
doubly_nested_oneshot  Winding-order factoring, nested two levels deep (alias: dnos)
nested_oneshot         Winding-order factoring, nested one level (aliases: nested, nos)
multilattice           The FDE/QM boundary as real code: the corrected Pauli-algebra WH action, orbit 4^n, zero axioms
jones_polynomial       Jones polynomial of a braid word (alias: jp)
braid_image            Render or compute a braid word's IMASM image (alias: bi)
braid-grammar          Braid word to IG-tuple grammar bridge (alias: bg)
circuit                Substrate round trips through the twelve-glyph alphabet: x86/RNA/wasm/AA via IMASM
counterfactual         Perturb one glyph of a word and read what moved: which invariants held, which broke, the smallest repair (alias: cf)
basin                  Fixed-point archaeology for word maps: orbit, attractor, transient depth, cycle length, basin size
ouroboros-inverse      Inverse grammar: tuple -> IMASM word -> braid word -> ... -> tuple (alias: oinv)
frobenius-fuzzer       Mine the whole word space for rare, stable Frobenius-closing programs (alias: fuzz)
provenance             Epistemic type checking: every result carries a provenance, provenances form a lattice (alias: prov)
ctc-loom               Fixed-point enumerator: every IMASM word of a given length walked to its Belnap verdict (alias: loom)
sk-forge               Crystal Harvester: read a public key as an IG tuple, find the nearest O-infinity carrier, report the repair path
demonstrate            Turn a claim into an executable experiment, every value computed at print time, nothing stored (alias: demo)
distance               Hamming and weighted distance of the active IG tuple from the ZFC baseline tuple (alias: dist)
mersearch              Parallel Mersenne prime search: FSPLIT-forked candidate space, big-integer Lucas-Lehmer (alias: msearch)
shor-qft               Shor's algorithm's QFT step, run directly
gpu_native             The literal GPU-native build protocol, run for real, not simulated
gpu16_3                Batch SIXTEEN_3 register gates on GPU: many registers at once, verified against the scalar path
gpu_gnfs               Grammar-native GNFS on GPU — FB + sieve + GF(2) + φ-congruence (multi-limb) on device
gpu_rho                GPU Pollard's rho, fixed 256-bit Montgomery width, verified against BigUint (verify | prims | factor)
gpu_rho_ml             Multi-limb GPU Pollard's rho beyond gpu_rho's 256-bit cap, any width up to 2048 bits
gpu_ecm                Lenstra elliptic-curve factorization on GPU at any width, one Montgomery curve per thread across every device
gpu_factor             Full GPU factorization: parallel trial division first, then ECM/rho/Pollard-Brent on whatever cofactor remains
gpu_shor               Shor's algorithm order-finding on GPU: brute-force verify, direct order lookup, or real baby-step/giant-step
gpu_dqi                DQI min-weight coset search on GPU, checked against the CPU path
gpu_fde                FDE tower theorems checked on GPU against the CPU functions
gpu_kernel             Execute or verify an IMASM program on GPU, checked against kernel::self_imscribe
gpu_millennium         Static self-imscription of the Millennium-conjecture ob3ects on GPU, checked against the CPU kernel
gpu_opi                OPI Prange trial arithmetic on GPU, checked against the CPU path
gpu_vox                vox's control-flow closure verdict run on GPU, checked against the CPU auditor
gaussian_extract       Fermat two-square extraction: a real root of -1 mod p, then Cornacchia descent to p=a²+b² (alias: gaussian)
abc                    Real arithmetic from the ABC/IUTT Lean proofs: radical, discrepancy, quality, window maximum, and a real scale-link check between window sizes
trilattice_factor      Trilattice factoring toolkit — read/sieve/factor/dialect-probe/gpu-verify, with the number's own crystal address (alias: tfactor)
factor_operator        Monotone factor-state constructor F_N over 𝟒={N,T,F,B}: cert certifies a pair, ambient states the moat (free-bit paradox), resolve strips the small part and crosses the moat to return factors (any width)
native_numeral         The word-native numeral toolkit itself: encode/decode/factor/decompose/redstep/cycle and the rest (alias: numeral)
phase_unbraid          Phase-based unbraider: the factors come out of a QFT phase readout (winding k/M -> period -> one gcd closure), no search
dyn_nest               Optimal oneshot nesting depth: the least depth at which the Brent cycle closes on a factor (aliases: dyn, dynamic_nest)
yz                     Yamakawa-Zhandry verifiable-quantum-advantage retranslation (r/c/Inc) run as real code, not read as a document
yz_list                Theorem 11.1's L-list mechanism from the YZ-retranslation, run for real rather than left open
anyon-sync             Dialetheic FOUR-valued Carrier16 register walked over THE_WORD, showing where the T/F/t/f lanes sync (alias: anyon_sync)
prime_winding grounded_add a+b plus every operand's own real Grammar-native type, sourced from imscribe generate grounding through digit_type_tensor, not read off the arithmetic
prime_winding grounded_mul a*b plus every operand's own real Grammar-native type, same grounding as grounded_add
prime_winding grounded_sub a-b plus every operand's own real Grammar-native type, same grounding as grounded_add
prime_winding grounded_mod a mod b plus every operand's own real Grammar-native type, same grounding as grounded_add
prime_winding grounded_divmod a=q*b+r plus all four numbers' own real Grammar-native type, same grounding as grounded_add
prime_winding grounded_gcd gcd(a,b) plus every operand's own real Grammar-native type, same grounding as grounded_add
prime_winding grounded_factor N's real prime factorization plus N's and every distinct factor's own real Grammar-native type
gpu_catalog_crystal    Batch the real catalog's crystal addresses on GPU: fixed-width crystal encode over every live entry
gpu_crystal_full_space Checks phase_5's 17.28-million-entry Crystal type-space claim at its actual scale, not just the live catalog
gpu_imasm_cycle        The full imasm-cycle round trip, forward and reverse legs, batched on GPU
gpu_ipc_no_serialization Measures phase_5's no-serialization IPC claim for real, both paths run and compared
gpu_sixteen3_tensor_kernel The sixteen3_gpu_tensor_kernel ob3ect's own batched-register protocol shape, not the flat per-lane version
```

`--json` forms feed the Lean certificate scripts; champion traces enumerate
displacement events. See `docs/abc_champions_trilattice.md` for the format.

## Repository map

| Path | Contents |
|---|---|
| `src/` | Rust runtime, REPL, trilattice, ABC, GPU, quantum modules |
| `scripts/` | JSON-to-Lean certificate tooling and audits |
| `docs/` | command and certificate documentation |
| `measurements/` | GPU benchmark records |

Verify: `cargo check --features hosted`, `cargo test --features hosted`.
Lean artifacts: `cd ../p4rakernel/p4ramill && lake build`.
Full notes: [`README_FULL.md`](README_FULL.md). Unlicense.

$\mu\circ\delta = \mathrm{id}$
