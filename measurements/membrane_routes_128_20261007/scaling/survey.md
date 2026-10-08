# IMASM semiprime scaling

The sweep covers 256, 512, 1024, and 1048 bits. Each Vox route received the canonical IMASM numeral word. `factor_one` received that same word baked into its build. The factorizer paths operate on Vox `Tape` numerals; output rendering may display decimal text.

The table records wall time and whether the route returned a factor pair. Operator routes exited normally but exhausted their moat budgets with a composite core, so those are not successful factorizations. The per-run caps were 30 seconds at 256 bits, 45 seconds at 512 bits, and 30 seconds for the corrected 1024-bit rerun. The malformed initial NINE_ARM invocation at 512 and 1024 was discarded and rerun with the actual `NINE_ARM` word.

| Route | 256-bit | 512-bit | 1024-bit |
|---|---|---|---|
| Vox `factor` / `smart_factor` | 30 s timeout | 45 s timeout | 30 s timeout |
| Vox `factor-membrane` / `smart_factor` | 30 s timeout | 45 s timeout | 30 s timeout |
| Vox `morphism-factor` | 30 s timeout | 45 s timeout | 30 s timeout |
| Vox operator `resolve` | 1.72 s, no factor | 3.04 s, no factor | 5.68 s, no factor |
| Vox operator `full` | 1.73 s, no factor | 3.04 s, no factor | 5.96 s, no factor |
| Vox `factor-with` / NINE_ARM | 30 s timeout | 30 s timeout | 30 s timeout |
| Vox `factor_one` | 30 s timeout | 45 s timeout | 30 s timeout |

The current G membrane runs accept canonical IMASM numeral words and carry their arithmetic through `WordTape`. The radix-four lift and ECM extractor each return closing pairs at all three requested widths on the generated witness inputs. The witness factors are 211 and a large prime, so these runs measure arbitrary-width word arithmetic and the ECM/radix-four continuation on a semiprime with a small factor.

| G route | 256-bit | 512-bit | 1048-bit |
|---|---:|---:|---:|
| ECM extractor, B1=500, B2=500, one curve | 0.063 s, pair closes | 0.132 s, pair closes | 0.338 s, pair closes |
| Radix-four lift, 2,000,000-node cap | 3.060 s, pair closes | 11.940 s, pair closes | 46.618 s, pair closes |
| 13 fixed-word membranes, 25,000 frontier steps then one ECM curve | 1.04–1.75 s, all close | 1.87–3.29 s, all close | 3.75–6.44 s, all close |
| Instant-read, squaring-cycle, Shor-order, and quantum-phase order routes, 2,048 iterations | 0.125–1.056 s, no pair | 0.239–2.016 s, no pair | 0.483–4.081 s, no pair |

The balanced controls use generated factors of equal width from `cases.json`. The radix-four lift reached its 200,000-node cap without a pair at 256, 512, or 1048 bits. The fixed-word aggregate route exhausted its 25,000-step frontier and one-curve ECM continuation without a pair at each width. ECM with B1=1000, B2=3000, and eight curves also returned no pair on those balanced controls. These are the hard controls for the witness results above.

The depth-three fixed-word route, with its corrected wiring and 25,000-step frontier, ran on the balanced 256-bit control for 120 seconds without emitting a pair; the run record is `256/membrane_fixed_nested_balanced_after_wiring.log`.

A second witness set uses the 31-bit prime factor 2147483647, with the cofactor sized to make each product 256, 512, or 1048 bits. The updated fixed-word membrane continues into ECM with B1=5000, B2=50000, and 100 curves after its 25,000-step fixed-word frontier.

| Route | 256-bit | 512-bit | 1048-bit |
|---|---:|---:|---:|
| ECM, B1=5000, B2=50000, 100 curves | 11.304 s, pair closes | 23.120 s, pair closes | 52.999 s, pair closes |
| Aggregate fixed-word then ECM | 12.473 s, pair closes | 25.996 s, pair closes | 57.531 s, pair closes |
| Radix-four lift, 200,000 nodes | 13.126 s, no pair | not run on this witness | not run on this witness |
| Radix-four lift, 2,000,000 nodes | 133.522 s, no pair | not run on this witness | not run on this witness |

The radix-four 256-bit run with the 31-bit factor reached 2,000,001 states without a pair. Its lift therefore does not reach this factor width at a reasonable cost under the measured state caps. The ECM and aggregate fixed-word results returned the 31-bit factor and a cofactor whose product closes to the input. Factor values and raw outputs are retained in `ecm_witness_p31/`.

The fixed-word carrier now nests each factor vessel at the preceding IMSCRIB site. The wiring diagram reports three ∈ splits and three ∋ fuses with a closed walk. The previous lateral three-unit word had seven splits and nine fuses and its diagram reported an open walk. On the 31-bit-factor cases, the nested fixed-word frontier reached ECM continuation, which produced 2,147,483,647 and the matching cofactor in 14.381, 28.796, and 64.375 seconds at 256, 512, and 1048 bits. Each membrane reports `product_closes=true`; `membrane_fixed_nested_after_wiring.log` retains the factor words and producer.

The radix-four search now probes equal-width factors first and checks each two-bit product prefix by truncating the canonical `WordTape` directly. On the 211-factor witnesses, it returned closing pairs at 256, 512, and 1048 bits in 4.561, 12.511, and 40.501 seconds with a 2,000,000-node cap. The balanced 256-bit control returned no pair within 2,000,001 states in 107.697 seconds. Per-size outputs are in `membrane_radix4_width_first_control.log` and `membrane_radix4_balanced_width_first.log`.

The order routes are classical word-level order-cycle attempts, not executions on a quantum device. The Shor-order, squaring-cycle, and quantum-phase binaries share one `WordTape` squaring-cycle implementation; instant-read retains its separate leaping walk. On the 31-bit-factor inputs, the shared route found no collision within 2,048 steps at 256, 512, or 1048 bits; runtimes were 0.125, 0.243, and 0.469 seconds. The Vox 256-bit balanced smart-factor and MPQS word routes each reached their 120-second test cap without output. The Vox factor routes continue through the same `smart_factor` implementation, so their entry points are not independent algorithms.

The shared G order route now executes the closed IMASM frame `⊢⊙⊙∈⊤≻⊥≻≻∋⊡⋈⊣`: it seeds both lanes, advances the tortoise once and the hare twice, fuses the lanes, then checks and fixes the cycle step. The wiring diagram reports one split-fuse pair and a closed walk. On the balanced 256-, 512-, and 1048-bit inputs, the word-driven route completed 2,048 steps without a collision in 0.122, 0.228, and 0.448 seconds.

The G ECM route now executes the nested stage word `⊢⊙∈≻∈⊞∋⋈∋⊡⊣`; each prime-scalar update executes `⊢≻⋈⊣` to advance the Montgomery point and check its denominator. Both wiring diagrams report closed walks. On the 31-bit-factor witnesses, ECM returned 2,147,483,647 with closing products at 256, 512, and 1048 bits in 11.800, 24.397, and 56.115 seconds. `ecm_word_sweep.json` and each `membrane_ecm_word.log` retain the canonical factor words and timings.

The G GNFS entry point accepts a canonical IMASM numeral word and returns factor and cofactor words. Its balanced 256-bit run used the automatic B=200000 bound. CUDA was unavailable, so the host fallback collected 19 of 36,167 required relations in 576 of 400,000 sieve blocks before the 300-second cap. Bounds 2,000 and 5,000 finished with zero relations in 19.4 and 99.5 seconds; bound 10,000 reached its 120-second cap with zero relations. The route's input boundary is word-native, while the GNFS kernel converts the numeral to `BigUint` internally.

Inputs, logs, and timing records are in the matching size directories. `cases.json` retains the balanced control factors; `ecm_witness_cases.json` retains the generated witness factors. The reference factors were not passed to any tested route.
