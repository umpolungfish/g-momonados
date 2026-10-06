









`quantum` — Quantum computation (fibqc, jones, braids, shor, shors_btc_2, btc_oneshot, secp256k1_unwinder, baryon_asymmetry, qft, iuft, sic, d12, d2048, dqi). Example: `help quantum`

















`help` — Help system (help <topic> for details). Example: `help fibqc`

`tools` — Real commands with no prior menu entry: opi, nested factoring, braids, GPU batches, fuzzing, provenance. Example: `help opi`

`imasm_add` — Run dynamically sized ripple-carry addition in the IMASM stream; inputs and output are LSB-first ⊤/⊥ encodings. Example: `imasm_add ⊥ ⊤⊥`

`imasm_sub` — Run dynamically sized ripple-borrow subtraction in the IMASM stream; final ⊥ indicates unsigned underflow. Example: `imasm_sub ⊤⊥ ⊥`

`imasm_mul` — Compile a width-specialized schoolbook multiplier whose partial products and carries execute as IMASM. Example: `imasm_mul ⊥⊥ ⊥⊥`

`imasm_divmod` — Compile restoring long division as IMASM; returns encoded quotient and remainder. Example: `imasm_divmod ⊥⊥⊥⊥ ⊥⊥`

`imasm_mod` — Return the remainder from the IMASM restoring-division circuit. Example: `imasm_mod ⊥⊥⊥⊥ ⊥⊥`

`imasm_gcd` — Compose restoring division into a width-specialized Euclidean closure, entirely in IMASM. Example: `imasm_gcd ⊥⊥⊥⊥ ⊥⊤⊥`

`imasm_close` — Nest IMASM multiplication inside exact product-to-N closure; all three operands are encoded tapes. Example: `imasm_close ⊥⊥ ⊥⊤⊥ ⊥⊥⊥⊥`

`imasm_edit_square` — Check the token-edit commuting square and its additive/multiplicative closure in one dynamically sized IMASM stream. Example: `imasm_edit_square ⊤⊥ ⊤⊥`

`imasm_powmod` — Compose phase-bit branches, IMASM multiplication, and restoring modulus closure into a modular-winding membrane. Example: `imasm_powmod ⊤⊥ ⊤⊥⊤⊥ ⊥⊤⊤⊤⊥`

`opi` — DQI's Optimal Polynomial Intersection: exact Lemma 9.2 eigenvalue + m to p asymptotic, settable wall-clock budget. Example: `opi run 10007 4954 10s`

`weight_ladder` — The general reduction behind opi's eigenvalue, for any m-exchangeable-trial Hamming-weight ladder, not just OPI. Example: `weight_ladder 10006 2477 0.0`

`nested_prime_factorization` — 16-morphism factorization tower, IFIX banks the complete factor record at the END (non-vacuous) (alias: npf). Example: `nested_prime_factorization factor 91`

`factor_membrane` — Factor-Separating Imscription Membrane: D2 lane split, N_lane DEFINED as P_lane*Q_lane (aliases: membrane, fmembrane). Example: `factor_membrane factor 91`

`shor_b4` — Key membrane finding: shors_algorithm x b4_factor_v2_engine click (T<->H, certified) + membrane family (aliases: shor_b4_membrane, key_membrane). Example: `shor_b4 click`

`membrane_family` — Membrane family registry: list/run factor membranes by word (alias: mfam). Example: `membrane_family list`

`doubly_nested_oneshot` — Winding-order factoring by hypernest depth: each depth wraps one more carrier and gives a fresh Brent seed. factor <N> [maxdepth] sweeps the ladder; ladder <N> [maxdepth] shows every depth's word, seed and budget (alias: dnos). Example: `doubly_nested_oneshot ladder 3215031751`

`nested_oneshot` — Winding-order factoring, nested one level (aliases: nested, nos). Example: `nested_oneshot factor 91`

`multilattice` — The FDE/QM boundary as real code: the corrected Pauli-algebra WH action, orbit 4^n, zero axioms. Example: `multilattice help`

`jones_polynomial` — Jones polynomial of a braid word (alias: jp). Example: `jp 1 2 1`

`braid_image` — Render or compute a braid word's IMASM image (alias: bi). Example: `bi 1 2 1`

`braid-grammar` — Braid word to IG-tuple grammar bridge (alias: bg). Example: `braid-grammar tuple \\`

`circuit` — Substrate round trips through the twelve-glyph alphabet: x86/RNA/wasm/AA via IMASM. Example: `circuit table`

`counterfactual` — Perturb one glyph of a word and read what moved: which invariants held, which broke, the smallest repair (alias: cf). Example: `cf help`

`shiab` — SHIABO: holographic scale-collapse. An integer collapses to its δ boundary with μ∘δ=id recovery and winding; an IMASM word is wrapped as bulk in ⊢⊙∈…⋈∋⊡⊣ and run. Example: `shiab 42`

`hyperstack` — Heterogeneous hypernesting: load a value through a stack of different carrier types (phase, shor, fib, ...), each carrier's word the payload of the next; reads register, verdict and winding at each layer. Phase closes to T, Shor opens the both-value B, Fibonacci holds it. Example: `hyperstack 91 phase shor fib`

`basin` — Fixed-point archaeology for word maps: orbit, attractor, transient depth, cycle length, basin size. Example: `basin help`

`ouroboros-inverse` — Inverse grammar: tuple -> IMASM word -> braid word -> ... -> tuple (alias: oinv). Example: `oinv help`

`frobenius-fuzzer` — Mine the whole word space for rare, stable Frobenius-closing programs (alias: fuzz). Example: `fuzz help`

`provenance` — Epistemic type checking: every result carries a provenance, provenances form a lattice (alias: prov). Example: `prov help`

`ctc-loom` — Fixed-point enumerator: every IMASM word of a given length walked to its Belnap verdict (alias: loom). Example: `loom help`

`sk-forge` — Crystal Harvester: read a public key as an IG tuple, find the nearest O-infinity carrier, report the repair path. Example: `sk-forge help`

`demonstrate` — Turn a claim into an executable experiment, every value computed at print time, nothing stored (alias: demo). Example: `demo help`

`distance` — Hamming and weighted distance of the active IG tuple from the ZFC baseline tuple (alias: dist). Example: `distance`

`mersearch` — Parallel Mersenne prime search: FSPLIT-forked candidate space, big-integer Lucas-Lehmer (alias: msearch). Example: `mersearch run 1 1000`

`shor-qft` — Shor's algorithm's QFT step, run directly. Example: `shor-qft help`

`gpu_native` — The literal GPU-native build protocol, run for real, not simulated. Example: `gpu_native run 1024`

`gpu16_3` — Batch SIXTEEN_3 register gates on GPU: many registers at once, verified against the scalar path. Example: `gpu16_3 verify`

`gpu_gnfs` — Grammar-native GNFS on GPU — FB + sieve + GF(2) + φ-congruence (multi-limb) on device. Example: `gpu_gnfs soak 15`

`gpu_rho` — GPU Pollard's rho, fixed 256-bit Montgomery width, verified against BigUint (verify | prims | factor). Example: `gpu_rho factor 8051`

`gpu_rho_ml` — Multi-limb GPU Pollard's rho beyond gpu_rho's 256-bit cap, any width up to 2048 bits. Example: `gpu_rho_ml factor 8051`

`gpu_ecm` — Lenstra elliptic-curve factorization on GPU at any width, one Montgomery curve per thread across every device. Example: `gpu_ecm 8051 50000`

`gpu_factor` — Full GPU factorization: parallel trial division first, then ECM/rho/Pollard-Brent on whatever cofactor remains. Example: `gpu_factor 8051`

`gpu_shor` — Shor's algorithm order-finding on GPU: brute-force verify, direct order lookup, or real baby-step/giant-step. Example: `gpu_shor bsgs 7 143`

`gpu_dqi` — DQI min-weight coset search on GPU, checked against the CPU path. Example: `gpu_dqi verify 128 20`

`gpu_fde` — FDE tower theorems checked on GPU against the CPU functions. Example: `gpu_fde verify 16`

`gpu_kernel` — Execute or verify an IMASM program on GPU, checked against kernel::self_imscribe. Example: `gpu_kernel verify 256 1`

`gpu_millennium` — Static self-imscription of the Millennium-conjecture ob3ects on GPU, checked against the CPU kernel. Example: `gpu_millennium verify`

`gpu_opi` — OPI Prange trial arithmetic on GPU, checked against the CPU path. Example: `gpu_opi verify 101 10`

`gpu_vox` — vox's control-flow closure verdict run on GPU, checked against the CPU auditor. Example: `gpu_vox verify 4096 1`

`gaussian_extract` — Fermat two-square extraction: a real root of -1 mod p, then Cornacchia descent to p=a²+b² (alias: gaussian). Example: `gaussian_extract 97`

`abc` — Real arithmetic from the ABC/IUTT Lean proofs: radical, discrepancy, quality, window maximum, and a real scale-link check between window sizes. Example: `abc window 0.1 9 32`

`trilattice_factor` — Trilattice factoring toolkit — read/sieve/factor/dialect-probe/gpu-verify, with the number's own crystal address (alias: tfactor). Example: `trilattice_factor read 8051`

`prime_tool` — Prime Gödel-encoding relationship tool: sieve witness, atomicity verdict, and structural analysis (alias: prime-tool). Example: `prime_tool 296650821743515430283258444261036507151`

`semiprime_tool` — Semiprime Gödel-encoding relationship tool: extract a factor pair and verify product closure (alias: semiprime-tool). Example: `semiprime_tool 296650821743515430283258444261036507151`

`arbitrary_factor` — Gödel-verified prime-power extraction with route trace (alias: arbitrary-factor). Example: `arbitrary_factor 340282366920938461286658806734041124249`

`arbitrary_anyon_factor` — Recursive measured anyon splits with Gödel verification and bounded route fallback. Example: `arbitrary_anyon_factor help`

`factor_operator` — Monotone factor-state constructor F_N over 𝟒={N,T,F,B}: cert certifies a pair, ambient states the moat (free-bit paradox), resolve strips the small part and crosses the moat to return factors (any width). Example: `factor_operator resolve 8051`

`native_numeral` — The word-native numeral toolkit itself: encode/decode/factor/decompose/redstep/cycle and the rest (alias: numeral). Example: `native_numeral factor 8051`

`phase_unbraid` — CUDA recycled control QPE: complete phase measurement, retained complex residues and verified factor words. Example: `phase_unbraid help`

`dyn_nest` — Optimal oneshot nesting depth: the least depth at which the Brent cycle closes on a factor (aliases: dyn, dynamic_nest). Example: `dyn_nest 91`

`yz` — Yamakawa-Zhandry verifiable-quantum-advantage retranslation (r/c/Inc) run as real code, not read as a document. Example: `yz report`

`yz_list` — Theorem 11.1's L-list mechanism from the YZ-retranslation, run for real rather than left open. Example: `yz_list help`

`anyon-sync` — Dialetheic FOUR-valued Carrier16 register walked over THE_WORD, showing where the T/F/t/f lanes sync (alias: anyon_sync). Example: `anyon-sync`

`prime_winding grounded_add` — a+b plus every operand's own real Grammar-native type, sourced from imscribe generate grounding through digit_type_tensor, not read off the arithmetic. Example: `prime_winding grounded_add 8051 8052`

`prime_winding grounded_mul` — a*b plus every operand's own real Grammar-native type, same grounding as grounded_add. Example: `prime_winding grounded_mul 8051 8052`

`prime_winding grounded_sub` — a-b plus every operand's own real Grammar-native type, same grounding as grounded_add. Example: `prime_winding grounded_sub 8052 8051`

`prime_winding grounded_mod` — a mod b plus every operand's own real Grammar-native type, same grounding as grounded_add. Example: `prime_winding grounded_mod 8052 8051`

`prime_winding grounded_divmod` — a=q*b+r plus all four numbers' own real Grammar-native type, same grounding as grounded_add. Example: `prime_winding grounded_divmod 8052 8051`

`prime_winding grounded_gcd` — gcd(a,b) plus every operand's own real Grammar-native type, same grounding as grounded_add. Example: `prime_winding grounded_gcd 8052 8051`

`prime_winding grounded_factor` — N's real prime factorization plus N's and every distinct factor's own real Grammar-native type. Example: `prime_winding grounded_factor 8051`

`gpu_catalog_crystal` — Batch the real catalog's crystal addresses on GPU: fixed-width crystal encode over every live entry. Example: `gpu_catalog_crystal`

`gpu_crystal_full_space` — Checks phase_5's 17.28-million-entry Crystal type-space claim at its actual scale, not just the live catalog. Example: `gpu_crystal_full_space`

`gpu_imasm_cycle` — The full imasm-cycle round trip, forward and reverse legs, batched on GPU. Example: `gpu_imasm_cycle`

`gpu_ipc_no_serialization` — Measures phase_5's no-serialization IPC claim for real, both paths run and compared. Example: `gpu_ipc_no_serialization`

`gpu_sixteen3_tensor_kernel` — The sixteen3_gpu_tensor_kernel ob3ect's own batched-register protocol shape, not the flat per-lane version. Example: `gpu_sixteen3_tensor_kernel`

`fold` — The fold verdict of a word — closed form, surplus, the enclosure witness, the codon lane. Example: `fold`

`erdos` — Guided walks through the Erdős manuscripts — list | schutte | landau | lcm. Example: `erdos schutte`

`proof bootstrap` — The Grammar verifying itself (7 steps, auto-play). Example: `proof bootstrap`

`prooflift` — Proof-lift report: undischarged claims and unrejoined forks as one object; `prooflift nest` runs the self-nest word, the proof of mu.delta=id itself (86065 glyphs, verdict T). Example: `prooflift nest`

`tick` — Run N manual ticks (default 1). Example: `tick 10`

`run` — Run N ticks; no arg = continuous (ESC to stop). Example: `run 100`

`watch` — Live terminal HUD (ESC to stop). Example: `watch`

`timer` — Run N ticks, one per PIT interrupt. Example: `timer 50`

`boot` — Load + run any program (I-XXVIII or decimal). Example: `boot VII`

`load` — Load program by Roman numeral. Example: `load XII`

`ctc` — every value in every action, with the price each closure cost. Example: `ctc`

`ctc not` — Belnap negation — fixed at Neither and Both, a 2-cycle between True and False. Example: `ctc not T`

`ctc next` — temporal step — True and False swap, Neither and Both hold. Example: `ctc next N`

`ctc collapse` — everything to Both in one step — one fixed point, whole space in its basin. Example: `ctc collapse T`

`ctc cycle` — T→F→N→B→T — no fixed point at all, so closure must be manufactured. Example: `ctc cycle T`

`ctc meet` — lattice meet against Both. Example: `ctc meet T`

`ctc join` — lattice join against Both. Example: `ctc join F`

`nesting` — the reference pairings, each predicted then run. Example: `nesting`

`nesting halve` — halve the distance to 3 — settles from anywhere, q = 0.5. Example: `nesting halve 203`

`nesting newton` — Newton on x³−2x−5 — settles fast in range, q well below 1. Example: `nesting newton 2`

`nesting shift` — add one forever — never settles, and its gap never changes. Example: `nesting shift 0`

`nesting rotate` — turn a third of a circle — a closed orbit, needs x and y. Example: `nesting rotate 1 0`

`nesting project` — flatten onto the first axis — settles in one step, needs x and y. Example: `nesting project 1 5`

`carriers` — reads the catalog: every entry meeting the closure condition, the distance between each pair, and the classes they fall into. Example: `carriers`

`substrate` — reads the sequence builder: return time and behaviour across the weight range, plus where a critical weight can exist at all. Example: `substrate`

`status` — Kernel status (tick, IP, stack, fork, frob). Example: `status`

`program` — Show loaded program + fork depth. Example: `program`

`snapshot` — Structural snapshot (sig, tier, period). Example: `snapshot`

`graph` — ASCII-art token graph with nesting. Example: `graph`

`heatmap` — B4 memory heatmap. Example: `heatmap`

`memory` — Dump B4 memory. Example: `memory`

`registers` — Show R0-R7, plus the real SIXTEEN_3 value from the last FSPLIT3/FFUSE3/EVALI. Example: `registers`

`color` — Toggle terminal colour (alias colour). Example: `color on`

`stack` — Stack depth. Example: `stack`

`canonical` — Load canonical program I-XII. Example: `canonical VII`

`continuous` — Load continuous program 1-4. Example: `continuous 3`

`novel` — Load novel program 1-3. Example: `novel 1`

`shunt` — Load shunted program 1-9. Example: `shunt 5`

`dynamic` — Dynamic mode: rebuild sequence from IgTuple each wrap. Example: `dynamic on`

`crystal` — Decode address to 12-tuple: crystal <addr>. Example: `crystal 42`

`crystal store` — Store entry: crystal store <n> [d]. Example: `crystal store my_system 42`

`crystal name` — Retrieve by name: crystal name <n>. Example: `crystal name sic_povm`

`crystal find` — List stored entries. Example: `crystal find`

`cycle` — walk an IMASM word around its ROTAT orbit (glyphs only). Example: `cycle ⊢⊙∈⊤⊥∋⋈⊡⊣`

`weight` — where the weight moves through an IMASM word. Example: `weight ⊢⊙∈⊤⊥∋⋈⊡⊣`

`banked` — was a count cleared with nothing banked?. Example: `banked ⊢⊙∈⊤⊥∋⋈⊡⊣`

`insert` — every one-glyph repair for an exposed word. Example: `insert ⊢⊙∈⊤⊥⊞∋><⋈⊡⊣`

`trans` — transitions counted on the ring, closing edge included. Example: `trans ⊢⊙∈⊤⊥∋⋈⊡⊣`

`arev` — H hop: read snapshot through the R1<->R2 mirror. Example: `arev`

`ask` — kernel structural ask (dry). Full wet: host ./ask --file | -i. Example: `ask What is the distance to CLINK L8?`

`spine` — manuscript spine: PROVE->UNIFY->PORT x vessel (no Python). Example: `spine`

`vessel` — witness-vessel transport: Clay payloads x 88 dialects, frob-gated. Example: `vessel`

`vita` — one certified turn from the on-board vae_vita trunk. Example: `vita`

`whoami` — IG tuple under the active ruleset. Example: `whoami --ruleset`

`ruleset` — show the active ruleset. Example: `ruleset`

`absorption` — list all absorption rules. Example: `absorption`

`replicative` — load the program targeting O_inf_dag (R2) deliberately. Example: `replicative`

`vox` — Control-flow closure auditor: verdict <word> is the classic FOUR-valued reading (T/F/B/N); sixteen3 check <word> is the real 16-valued machine (t/f included, full step trace) — two different engines, not the same question twice. compile <seq> [--code std|mito] [--pdb <path>] runs the RNA<->protein pipeline both ways from one entry point: RNA/DNA in gives a compiled protein with real fold info (Chou-Fasman secondary structure, heuristic tertiary contacts, a real 3D backbone via B4-Ramachandran-NeRF); protein in gives RNA/DNA back out (Frobenius-preferred codon per residue, full degeneracy) with the same fold computed on the input; direction auto-detects from the input alphabet, --pdb writes a real PDB file readable back by rebis pdb. evm <hex> | wasm <hex> | classify <mn> | run <file> [--argv a,b] — runs the file as a real process from its own entry, real argv/envp/auxv stack, real read/write/open/openat/close/mmap/brk syscalls. run <sym> <file> [--args a,b] calls one function directly instead: scalar int args, one int back, no process. Static binaries on direct syscalls run for real; dynamic linking and glibc's own TLS setup are further rungs, not yet built. Example: `vox compile ATGGCCTGTGGCAAGTAA --pdb folded.pdb`

`quit` — halt the kernel (aliases exit, halt). Example: `quit`

`anyon_factor` — N-only Fibonacci anyon phase factorization; runtime owns base schedule, shot policy, and controller endpoint. Example: `anyon_factor 296650821743515430283258444261036507151`

`factor_phase` — Exact folded coherent CPU execution and pair measurement over baked IMASM source and quantile. Example: `factor_phase`

`anyon_ququart_word` — Compile full Z4 Fourier to a Fibonacci braid and measure computational, leakage, and unitarity residuals. Example: `anyon_ququart_word 296650821743515430283258444261036507151`

`fibqc` — Fibonacci anyon QC: verify | compile | jones | knot | winding (see also qc, jp). Example: `fibqc verify`

`qc` — Compile a circuit over H T S X to a braid word; spaces optional; draw|svg|loop before the gates renders it, and two depths size the net and the recursion (aliases quantum_compile, fibqc compile). Example: `qc loop HTSX 10 3`

`bi` — Draw a braid word — strand diagram in the terminal, SVG with `svg`, the closed braid as a ring with `loop`; window with start:count, column height with /N (alias braid_image). Example: `bi loop 1 2 -1 -2 1 2`

`jp` — Jones polynomial at the 1/5 winding; signed Artin generators (alias jones_polynomial). Example: `jp 1 1 1`

`bg` — Braid word to grammar tuple (alias braid-grammar); winding is a closed form in the writhe. Example: `bg tuple 1,2,1 3`

`shor` — Belnap Shor pipeline + dialetheic Fibonacci Shor (word ⊢∈≻⋈⊞∈⊤≻⊥≺∋⊙⋈⊡⊣); N=15,21. Example: `shor dialetheic 15 7`

`shors_btc_2` — Shor over secp256k1 ECDLP: recover a Bitcoin private key from a public key (x,y). Example: `shors_btc_2`

`prime_winding` — Winding period of the primes on the number line - ob3ect-backed: find | factor | cycle | tuple | verdict. Example: `prime_winding find 100`

`oneshot_prime_winder` — One-shot primality test using IMASM word ⊢∈≻⊤⋈⊙≺⊥⊞∋⊡⊣ winding certificate. Example: `oneshot_prime_winder 17`

`dyn_nest` — Dynamic Nesting Prime Finder - pipes oneshot verdict, searches optimal nesting depth d=1,2,3,...; period P(d)=5d+7, closure-derived seed. Example: `dyn_nest 1234567`

`qft` — Quantum Fourier Transform: circuit | phases | iqft | iqft braid | braid, on n qubits. Example: `qft circuit 3`

`btc_oneshot` — BTC Secret Key Oneshot Operator — structural verification & phase steps. Example: `btc_oneshot verify`

`secp256k1_unwinder` — 19-glyph morphism sequence for secp256k1 scalar recovery: word | steps | mapping | walk [k] | verdict [k] | tuple | constants. Example: `secp256k1_unwinder walk 0`

`baryon_asymmetry` — baryon asymmetry as a banked survival: the word run through the live weight + banked instruments (report | word | mapping | reading). Example: `baryon_asymmetry report`

`theta-link` — IUTT housed in the paraconsistent ambient: the Θ-link closes yet holds register A (four-valued B, the Inclosure), unreachable by the Boolean adjoints (alias iutt). Example: `theta-link`

`iuft` — IUFT QC gates — the 12->3 Euler-angle SU(2) encoding of an IG tuple. Example: `iuft list`

`teich` — IUFT <-> IUTT bridge: Teichmuller deformation paths as gate trajectories. Example: `teich canonical`

`hqe` — Holonomic quasi-ergodic quantale, MBL holonomy. Example: `hqe report`

`dyson` — Dyson beta-ensemble, double-ramified cycle. Example: `dyson report`

`troq` — Triple-ramified ouroboric quantale. Example: `troq report`

`afdmc` — Asymptotic frozen-disordered monadic cohomology. Example: `afdmc report`

`hop` — Universe hopping, cross-framework transport. Example: `hop report`

`manifold` — Topological manifold operations. Example: `manifold`

`triple` — Triple-frame von Neumann superoperator algebra. Example: `triple report`

`sic` — SIC-POVM d=12 identity, three lattice proofs. Example: `sic`

`bip39` — BIP39-SIC-POVM: search | words | verify | map | gap. Example: `bip39 sic verify`

`d12` — d=12 SIC Phase VI: tower, magnitudes, orbits, existence, duallink, z0. Example: `d12 tower`

`d2048` — d=2048 moduli tower ascent (alias d2k). Example: `d2048 next`

`dqi` — Decoded Quantum Interferometry operator: word | period | phase | verdict <arm> | syndrome <bits> | tuple | report. Example: `dqi report`

`ig` — IG tuple + crystal address. Example: `ig`

`classify` — Nearest-catalog classification. Example: `classify`

`frob` — Frobenius harness status. Example: `frob`

`aleph` — Hebrew glyph encoding: aleph <word>. Example: `aleph שלום`

`rh` — Riemann Hypothesis bridge. Example: `rh`

`ym` — Yang-Mills mass gap bridge. Example: `ym`

`temp` — Temporal logic bridge. Example: `temp`

`cat` — Category theory bridge. Example: `cat`

`algebra` — distance|meet|join|tensor vs ZFC. Example: `algebra distance`

`cl8nk` — CLINK Layer 8: cl8nk <action> [name]. Example: `cl8nk entry sic_povm`

`c4` — Belnap C₄ complex plane (i²=B). Example: `c4`

`cscore` — Consciousness score (dual-gate). Example: `cscore`

`constants` — MoDoT constant closure: fine-structure, proton-electron, lepton, boson, gravity. Example: `constants`

`ovm` — OVM Computation Tools. Example: `ovm list`

`oneshots` — the 10 exotic fixed-point nestings: inner already at outer's fixed point. Example: `oneshots`

`ctc` — nest a value in an action; closure imposed where the action has none, priced by the width it smears. Example: `ctc cycle T`

`collatz` — the Collatz block nesting: blocks to one, the budget spectrum, and the records. Example: `collatz 27`

`straus` — the Erdős–Straus ladder: which rung r closes 4/n, and the spectrum across a range. Example: `straus 49`

`nesting` — read a point against a map: q=r2/r1 splits attracted from never-arrives where one gap cannot. Example: `nesting halve 203`

`carriers` — census of the mu-delta=id carriers by class: one fixed point seen many ways, or a family. Example: `carriers`

`substrate` — closure constant, content bifurcating: the conservative substrate read on both observables. Example: `substrate`

`stark` — Stark unit extraction: formula,fibqc,tower,exponents,verify. Example: `stark formula 2048`

`riemann` — Riemann-SIC report; sub-actions available. Example: `riemann`

`distance` — Hamming + weighted distance vs the ZFC baseline tuple (alias dist). Example: `distance`

`join` — join of the active IG tuple with the ZFC baseline. Example: `join`

`sigma` — sigma <n> — analyze the Sigma(n) divisor ring. Example: `sigma 5`

`ringspec` — ringspec <w1> <w2> <w3> — the spectrum of a ring, in integers: bond weights around a cycle, clean bond 1, cross-link its reaction centres; three is the minimum. Example: `ringspec 1 2 2 1`

`clay` — Clay Millennium structural status (machine-checked). Example: `clay`

`psm` — dialetheic alignment + measurement tests. Example: `psm test`

`entropy` — entropy experiment: dS vs tier promotion. Example: `entropy tier`

`invariant` — Discover invariants under transformations: ROTAT, IMSCRIB, FSPLIT/FFUSE. Example: `invariant catalog under ROTAT`

`redteam` — Adversarial testing: analyze|stress|mutate, and audit <theory> for hidden assumptions. Example: `redteam audit RH`

`witness` — Smallest executable object standing behind a claim. Example: `witness bsd`

`counterfactual` — Perturb one glyph: invariants held/broken, reversibility, smallest repair (alias cf). Example: `counterfactual \\u{22a2}\\u{2208}\\u{22a4}\\u{220b}\\u{22a3} rotate 1`

`basin` — Fixed-point archaeology: orbit, attractor, transient depth, exact basin size. Example: `basin \\u{22a2}\\u{2208}\\u{22a4}\\u{220b} --action REPAIR`

`ouroboros-inverse` — Inverse grammar: shortest IMASM word imscribing a tuple, plus its braid (alias oinv). Example: `oinv`

`frobenius-fuzzer` — Mine the word space for programs the braid reproduces exactly (alias fuzz). Example: `fuzz --len 3`

`oracle` — Adversarial: hunt the cheapest structural counterexample; surviving is not proof. Example: `oracle rotat-register`

`blackbox` — Infer a law from integer observations, ranked by fit minus complexity. Example: `blackbox 1 1 2 3 5 8 13`

`dialetheic-compiler` — Lift a classical gate into Belnap FOUR; show where a row rests on a paradox. Example: `dialetheic-compiler xor`

`stark-geometer` — SIC Stark arithmetic for dimension d: m_d, unit, ramified primes. Example: `stark-geometer 12`

`dialect-necromancer` — Imscribe a fragment and recover its nearest catalog ghost. Example: `dialect-necromancer the boundary imscribes the bulk`

`braid-apocrypha` — Search braid words for a target Jones magnitude; first hit is shortest. Example: `braid-apocrypha --target 0.618034`

`proof-braider` — Lift a claim to a braid and back; PASS iff Frobenius closure survives. Example: `proof-braider roundtrip Imscribing.Frobenius`

`universe-wormhole` — Minimum gate-space path between two hop frameworks, as a braid + Jones. Example: `universe-wormhole hqe fibonacci`

`vox-ce` — Lift EVM/WASM hex into an IMASM word and verdict its control-flow closure. Example: `vox-ce evm 0x600160025b00`

`consciousness-lath` — Single-axis mutation that most raises the C-score with both gates open. Example: `consciousness-lath ⊢∈><⊤⋈⊙⊞∋⊡⊣`

`paradox-engine` — Hunt words that are dialetheias by four readings at once (B, price, gate1, C=0). Example: `paradox-engine --min-price 3`

`key-dissolver` — SIC-narrowed bounded window before a BSGS split. Example: `key-dissolver 03f01d 40`

`compiler` — Compile a braid to imasm/jones/lean, or a token word back to a braid. Example: `compiler braid 1 2 1 --to imasm`

`catalogue` — Synthesize candidate operators; rank by novelty against the catalog. Example: `catalogue synthesize --top 5`

`sk_forge` — Crystal Harvester: BIP39-SIC integrated structural gap analysis against O_∞ carriers. Commands: forge, tuple, word, verify, carriers, bip39-sic, bip39-pipeline (alias sk-forge). Example: `sk_forge bip39-sic`

`museum` — The permanent collection of failed constructions — append-only negative knowledge. Example: `museum open`

`phase` — Phase as an object: orbit spectrum, phase period, and two-word interference. Example: `phase interference \\u{22a2}\\u{2208}\\u{22a4}\\u{220b} \\u{22a2}\\u{22a4}\\u{2208}\\u{220b}`

`demonstrate` — Run a claim as an experiment: INPUT/OPERATION/OUTPUT/CHECK, computed live (alias demo). Example: `demonstrate mu-delta 1 2 -1`

`loss` — What a transformation destroys: entropy in/out, bits destroyed, irreversible transitions. Example: `loss collapse`

`shadow` — Ontological nearest-neighbour: shared structure and the measured critical difference. Example: `shadow hsoa`

`provenance` — Epistemic type check: dependency DAG graded by lattice MEET, not by best sibling (alias prov). Example: `prov RH`

`ctc-loom` — Sweep the six Belnap actions over the whole word space; rank closures by price (alias loom). Example: `ctc-loom --len 3`

`cl9nk` — CLINK L9, the replicative lateral: d(L8,L9) and the ladder read from L9. Example: `cl9nk chain`

`crystal-scope` — Substitution microscope: distance, tier, dS, gate jump, and the measured driver (alias cscope). Example: `cscope`

`minimal` — Shortest word achieving a target property. Example: `minimal reach O_inf`

`repair` — Ranked program/proof surgery with a proof-diff. Example: `repair \\u{22a2}\\u{2208}`

`mersearch` — Mersenne search: run|ll. Composite exponents answer at once (alias msearch). Example: `msearch ll 2213`

`pk2sk` — PK→SK recovery: bounded-range ECDLP on secp256k1 — recover the scalar in [lo, hi) from its compressed public key, curve-gated, imscribed. Example: `pk2sk selftest`

`fde` — FDE(n) tower navigation: embed | restrict | walk | roundtrip | trans | report — ascend/descend the truth-value lattice at any depth. Example: `fde walk 2 3 4 3 2 1`

`rsa` — RSA decrypter via BSGS period-finding on ord_N(C): word | period | verify | <C> <N> <e> — only closes when that order is small, not for real RSA moduli. Example: `rsa word`

`combo` — cycle a word, then run weight | banked | insert | repair on every distinct rotation it produces, formatted as one report; add 'brief' for repair's cheapest candidate only. Example: `combo ⊢∈≻⊤≺⊥⋈⊞⊙⋈∈≻⊤≺⊥⋈⊞⋈∋⊡⊣⊙ brief`

`combo2` — combo (brief) on a word, then weight | banked | insert | repair again on every distinct word the first pass's repairs produced. Example: `combo2 ⊢∈≻⊤≺⊥⋈⊞⊙⋈∈≻⊤≺⊥⋈⊞⋈∋⊡⊣⊙`

`millennium` — run weight | banked | insert on a Millennium conjecture's promotion word and print its live executed crystal address and tuple; no argument lists the seven names, 'all' runs every one. Example: `millennium rh_positivity_promotion`

`pk2sk search` — recover the private scalar from a compressed public key when the scalar lies in [lo, hi): BSGS meet-in-the-middle, gated by the curve itself. Example: `pk2sk search 03f01d6b9018ab421dd410404cb869072065522bf85734008f105cf385a023a80f 12000 13000`

`pk2sk selftest` — recover the fixed selftest key (SK = 0x1000000b8ef) from its public key alone. Example: `pk2sk selftest`

`rebis codon` — Codon ↔ AA bidirectional. Example: `rebis codon AUG`

`rebis translate` — Gene → protein pipeline. Example: `rebis translate ATG...`

`rebis reverse` — Protein → mRNA → DNA. Example: `rebis reverse MKY...`

`rebis frob` — Frobenius filtration (64 codons). Example: `rebis frob`

`rebis genetics` — 7-stage genetic code verification. Example: `rebis genetics`

`rebis hadron` — Belnap hadron analysis. Example: `rebis hadron`

`rebis serpent` — Serpent rod motif analysis. Example: `rebis serpent`

`rebis pipeline` — IG promotion pipeline. Example: `rebis pipeline`

`rebis strata` — Codon stratum counts. Example: `rebis strata`

`rebis asm` — Genetic ParaASM programs. Example: `rebis asm`

`rebis tuples` — 7-stage generative tuple pipeline. Example: `rebis tuples`

`rebis clu` — CLU power-law clustering. Example: `rebis clu`

`rebis exotic` — Exotic hadron Frobenius verification. Example: `rebis exotic`

`rebis pdb` — PDB structure validation. Example: `rebis pdb 1CRN`

`rebis antibody` — Antibody CDR design. Example: `rebis antibody`

`rebis material` — IG material forge & metamaterials. Example: `rebis material`

`rebis sidechain` — AA sidechain × environment algebra (20×4). Example: `rebis sidechain`

`rebis ligand` — Ligand design from catalytic sites. Example: `rebis ligand`

`rebis decay` — Nuclear decay as IMASM winding. Example: `rebis decay`

`rebis bio` — Biological computation. Example: `rebis bio`

`rebis tx` — Therapeutics (chemo, pill, antidote). Example: `rebis tx`

`ruleset show` — Active ruleset display. Example: `ruleset show`

`ruleset verify` — Invariant violation check. Example: `ruleset verify`

`jump` — Cross-dialect jump: jump <U> using <c>. Example: `jump 42 using clay`

`seal` — IFIX commit to current ruleset. Example: `seal`

`whoami --ruleset` — IG tuple under active ruleset. Example: `whoami --ruleset`

`tensor` — Tensor under active absorption. Example: `tensor`

`meet` — Meet under active absorption. Example: `meet`

`absorb_test` — Test absorption rule. Example: `absorb_test`

`absorption show` — List absorption rules. Example: `absorption show`

`tstatus` — T-constitution pass/fail. Example: `tstatus`

`compound list` — List 11 diaschizic compounds. Example: `compound list`

`compound` — compound show|load <name>. Example: `compound show I`

`psm test` — Dialetheic alignment + measurement. Example: `psm test`

`psm frob` — Frobenius identity cycle. Example: `psm frob`

`psm kernel` — Kernel-state B3 invariant loop. Example: `psm kernel 5`

`psm load` — Inline ParaASM program (; separator). Example: `psm load ENGAGR %r0; FSPLIT %r0 %r1 %r2; FFUSE %r1 %r2 %r0; HALT`

`cr3` — Theorem engine (Collatz, Goldbach, Three-Body, Burnside, ...). Example: `cr3`

`p4ra` — p4rakernel Belnap+Frobenius 13-step bootstrap. Example: `p4ra`

`cr3 --version` — cr3 version info. Example: `cr3 --version`