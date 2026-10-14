# Execution chart, 2026-10-11/12 session

The production clocks below are trigger-to-readout and trigger-to-verified-divisor; preparation is separate. `N` is the only production input. Factor pairs are emitted results, not preparation inputs. Timeout rows have no verified factors.

## Source-carrier production

| Case and route | Bits | N | Verified factors | Preparation (s) | Readout (s) | Verified (s) | Outcome |
|---|---:|---|---|---:|---:|---:|---|
| `fresh_208`, source carrier | 208 | `212696074052644444730799156432393919343637051450205685227262203` | `10915760577478145067687293710409 × 19485227121185481410915489087267` | 0.002980 | 21.840482 | 21.840542 | MPQS relation source; exact product verified |
| `fresh_208`, product-prefix constraints | 208 | `212696074052644444730799156432393919343637051450205685227262203` | `10915760577478145067687293710409 × 19485227121185481410915489087267` | 1.858853 | 24.000369 | 24.000417 | MPQS relation source; exact product verified |
| `regression_224`, source carrier | 224 | `24937384895847894358120264124930833426221437163545452015667815601231` | `4972967419589394946757959119780619 × 5014588432173342030119256548931149` | 0.003142 | 73.611898 | 73.611946 | MPQS relation source; exact product verified |
| `regression_224`, dispatch rerun | 224 | `24937384895847894358120264124930833426221437163545452015667815601231` | `4972967419589394946757959119780619 × 5014588432173342030119256548931149` | 0.003116 | 70.078989 | 70.079022 | MPQS relation source; exact product verified |
| `regression_224`, integrated dispatch rerun | 224 | `24937384895847894358120264124930833426221437163545452015667815601231` | `4972967419589394946757959119780619 × 5014588432173342030119256548931149` | 0.003063 | 71.292463 | 71.292494 | MPQS relation source; exact product verified; Vox/wirer binding passed |
| `fresh_balanced_256`, source carrier | 256 | `58735271039119824379958558361762334408072518206897009238344722563515510177171` | none emitted | 0.003366 | no readout | 90 s bound | timeout; no factor receipt |
| `requested_1`, shared constraints | 332 | `7014129566030312171023600324145522815701572066391356680469502560191571052957947310214883282904546197` | none emitted | 1.949475 | no readout | 90 s bound | timeout; no factor receipt |
| `requested_2`, shared constraints | 332 | `4703321929167710870661316031225298828673962295985672422378224625494988791218448088490050276044105413` | none emitted | 1.997522 | no readout | 90 s bound | timeout; no factor receipt |
| `requested_3`, shared constraints | 332 | `6808825106036859938401754400988677246918168107554524646825660800678549039145488967267690298252666801` | none emitted | 1.999192 | no readout | 90 s bound | timeout; no factor receipt |
| `regression_895`, source carrier | 895 | `233108530344407544527637656910680524145619812480305449042948611968495918245135782867888369318577116418213919268572658314913060672626911354027609793166341626693946596196427744273886601876896313468704059066746903123910748277606548649151920812699309766587514735456594993207` | none emitted | 0.026933 | no readout | 90 s bound | timeout; no factor receipt |
| `regression_895`, retained 10-minute run | 895 | `233108530344407544527637656910680524145619812480305449042948611968495918245135782867888369318577116418213919268572658314913060672626911354027609793166341626693946596196427744273886601876896313468704059066746903123910748277606548649151920812699309766587514735456594993207` | none emitted | not recorded | no readout | 600 s requested bound | process absent afterward; external exit status was not captured |

The observation recorded zero factor lines and 119 progress samples.

## 895-bit sieve admission trace

The retained `active_base_regression_895` executable has SHA-256 `97a0a7a951d65d61d153933e01a6c977dd7ceffbb30e0b3ec7c8ecb09d654136`. Its source snapshot predates `VOX_SIEVE_ADMISSION` and the full-tape `prime_power_marks_tape` scoring path. Its overlong native traces recorded zero candidates, but they do not measure the current scoring path.

The rebuilt N-only executable `primepower_20261012_regression_895` has SHA-256 `b767aa2ad073bed9167cc75d37bb85afc40f1bfdf2f70116fa5ba86cf2ca07db` and source SHA-256 `94d5646e3ccb0b7ef083216649f48d1d74664c09c5ed07791bac0f5383d1dc24`. Vox identified its executed carrier as `WITNESS -> POWER -> EXTRACT -> SQUFOF -> P_MINUS -> P_PLUS -> LEHMAN -> ECM -> SIEVE_SOURCE -> FIX`. The binary wirer reports complete instruction coverage, all indirect transfers resolved, Frobenius closure, and a publishable IMASM tape. Its receipt is `primepower_20261012_regression_895.receipt.json`.

Over the 90-second production bound, the current binary prepared in 0.019592 seconds and entered the source-carrier MPQS arm. Its last admission sample was threshold 438, maximum observed score 158, and 153,417 scored blocks. The sieve scanned 9,749,664,297 positions across 4,650 polynomials, with zero admitted candidates and zero relations. Readout and factor-verification clocks did not occur. The comparison 224-bit source-carrier run recorded threshold 105, maximum score 135, and emitted candidates and relations before producing its verified factor pair. The 895-bit result locates the immediate failure before candidate relation arithmetic: the current score support never reaches the admission threshold. It does not measure a full-width arithmetic cost in candidate processing.

The retained 895-bit executable's native traces span 524.472913 seconds for `profile90` and 459.447093 seconds for `profile300`; their wrappers failed to enforce their requested bounds. Both target processes and Vox tracers were terminated after the overrun. Those samples remain separate from the rebuilt 90-second N-only production run.

With the `B²`-aligned threshold, a fresh 90-second N-only rerun prepared in 0.019540 seconds and recorded threshold 433, maximum score 158, and 155,756 scored blocks. It scanned 9,898,365,551 positions across 4,720 polynomials, with zero candidates and zero relations. The run remained bound to source SHA-256 `94d5646e3ccb0b7ef083216649f48d1d74664c09c5ed07791bac0f5383d1dc24` and binary SHA-256 `5a40d45511a3cc1148745fdd2cedbb4e898eb0d2b9cc5f0e41708bf4d7e98567`. Vox and the binary wirer retained the source-carrier tower and closed whole-image wiring. Its receipt carries the signed SIC residual records and coherence control.

With the `B²`-aligned threshold, a fresh 90-second N-only rerun prepared in 0.019540 seconds and recorded threshold 433, maximum score 158, and 155,756 scored blocks. It scanned 9,898,365,551 positions across 4,720 polynomials, with zero candidates and zero relations. The run remained bound to the same source SHA-256 `94d5646e3ccb0b7ef083216649f48d1d74664c09c5ed07791bac0f5383d1dc24`; its binary SHA-256 is `5a40d45511a3cc1148745fdd2cedbb4e898eb0d2b9cc5f0e41708bf4d7e98567`. Vox and the binary wirer retained the source-carrier tower and closed whole-image wiring. The receipt carries all signed SIC residual records and the coherence control.

The carrier-profile stage counter now follows the executed tower order: `SIEVE_SOURCE` is stage 9 and `FIX` is stage 10. The previous counter map assigned those labels in reverse, so earlier stage-number readings need this correction; the operator execution order and 895-bit admission measurements are unchanged.

## 332-bit admission and residual boundary

All three supplied 332-bit N-only sources were rebuilt, disassembled, and bound to the executed source-carrier word. Their preparation times were 0.004694, 0.004761, and 0.006114 seconds. Under the original `B²` residual cutoff, the 90-second runs recorded score thresholds/maxima of 157/157, 156/150, and 156/154. Candidate counts were one, zero, and zero; none produced a relation or readout.

The admission threshold now uses the logarithm of the actual accepted residual bound, `B²`, rather than `B × factor-base width`. On `requested_1` this changed the threshold from 157 to 152 and admitted five candidates in 90.006901 seconds, all with residuals at most `B²`; no relation closed. Preparation was 0.004810 seconds. Its receipt binds source SHA-256 `0f8da5fd31894e97151db8a23f41a6600ac1a14b85fbb0b61200fa574625e50e` to binary SHA-256 `039485b813341c03c3b0c95a9e4dc745301d805cf14f24f2d0832a4c50c85333` and retains signed SIC residuals.

A bounded `B⁴` admission experiment accepted residual partials without a large-prime cycle closure. `requested_1`, `requested_2`, and `requested_3` admitted 4,594, 4,336, and 4,364 candidates; each run produced zero complete relations. The first run classified 30 residuals at most `B²` and 4,564 between `B²` and `B⁴`. The 224-bit regression reached its 90.090-second bound without a factor under this `B⁴` path. That admission/relation path is withdrawn; the retained implementation uses the `B²` cutoff. With the calibrated `B²` threshold, the 224-bit source again returned its verified factor pair in 69.466403 seconds after 0.003168 seconds of preparation. Its receipt retains signed SIC residuals and the Vox/wirer binding.

## Bounded native profile

Vox `profile-native --duration-ms` stops and reaps its traced target at the requested deadline, then closes sample, counter, sieve, and status reports. A 15.013-second profile of the verified 224-bit source-carrier executable collected 1,459 samples. The MPQS function held 77.519%; `fold` and `unfold` held 2.125% and 1.371%. The hottest sampled instruction addresses, `0xa0474` and `0xa0493`, are the two root-stride loops that add prime-log marks into the sieve block. The trace ended with `deadline_reached=true` and no target process remained.

A four-way unroll of those stride loops preserved sieve output but increased the verified 224-bit run from 69.280065 to 74.716679 seconds, with nearly identical work counters. That path was withdrawn. The profile locates the main runtime in sieve-block marking and scanning, while full-width `fold`/`unfold` conversion is a small share of this successful run. Profile artifacts use `residualslack_20261012_regression_224.profile15.*`; the unrolled comparison is `markunroll_20261012_regression_224.receipt.json`.

## Dispatch comparison and controls

| Case and route | Bits | N | Verified factors | Execution time | Outcome |
|---|---:|---|---|---:|---|
| `fresh_balanced_224`, old `dialectic-lean` dispatch | 224 | `17633830729170705443114731107413090459099559935655905618404853337953` | none emitted | 90.001 s | timeout in repeated Lehman descent |
| `control_256_close`, `ResidentDialectic` control | 256 | `70191810847962523428393383144335058145044794059705339572713006486785864659869` | `264937371557812740802878898768711833427 × 264937371557812740802878898768711834447` | 0.001922 s process; 0.000029 s trigger-to-verified | exact product verified; close-factor control |
| `control_256_multiplier`, `ResidentDialectic` control | 256 | `69524785875384045385100433959484997002735054119456659233154042805048009446457` | `372893512615556360754395794708509348761 × 186446756307778180377197897354254674337` | 0.016185 s process; 0.006892 s trigger-to-verified | exact product verified; multiplier-family control |
| `control_91`, source carrier | 7 | `91` | `7 × 13` | 0.000570 s trigger-to-verified | exact product verified |

The two 256-bit controls use the separate `ResidentDialectic` producer and have deliberately structured factors. They are controls, not hard-balanced-semiprime performance claims. The three 224-bit source-carrier measurements are repeated executions of the same N and verified factor pair.

## Receipts

- Source-carrier runs and exact product checks: `measurements/factor_source_carrier_20261011/active_base_fresh_208.receipt.json`, `shared_fresh_208.receipt.json`, `active_base_regression_224.receipt.json`, `dispatch_20261012_regression_224.receipt.json`, and `dispatchfixed_20261012_r1_regression_224.receipt.json`.
- Hard-target timeouts: `measurements/factor_source_carrier_20261011/active_base_fresh_256.receipt.json`, `active_base_regression_895.receipt.json`, and `requested_shared_requested_{1,2,3}.receipt.json`.
- 224-bit old dispatch and control receipts: `../Vox/measurements/dialectic_lean_20261012/byte_fresh_balanced_224.receipt.json`, `traced_control_256_close.receipt.json`, and `byte_control_256_multiplier.receipt.json`.
- 895-bit external observation: `measurements/factor_source_carrier_20261011/895_10min.observation.json`.
