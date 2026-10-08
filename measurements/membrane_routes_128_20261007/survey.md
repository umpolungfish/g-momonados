**Input-boundary correction:** this report records the initial decimal-argument survey. The IMASM-only rerun is in imasm_only/survey.md; use that table for comparisons under the IMASM input requirement.

# 128-bit membrane route survey

Test value: 216083327738922615468318657912861339493 (128 bits), generated as the product of two independently selected 64-bit primes.
Reference pair: 13274714710409744131 × 16277813305432074103.

All runtime-input routes received the same N. The 15 fixed-word membrane_* executables share the G main_membrane engine and were given depth 64 and 25,000 frontier steps. The G two-arm factor membrane, Vox factor membrane, legacy order and ECM routes had 20-second wall limits. Vox's 2-adic builder baked this N, then its executable ran with a 60-second limit. Vox operator routes used 100,000 moat nodes.

## Factor-producing routes

| Route | Execution time | Producer |
|---|---:|---|
| Vox factor_one, N baked into its IMASM word | 0.1075 s | Vox morphism_factor::smart_factor |
| Vox factor N | 0.1140 s | Vox shape-routed factorizer |
| Vox factor-membrane N | 0.2451 s | Vox smart factorizer, named in its report |

Each produced the reference pair and reconstructed N. factor_one.sh took 29.79 s for its first bake/build plus execution; the 0.1075 s timing is the already-built membrane execution. The fastest measured execution is therefore the baked factor_one route, closely followed by the Vox factor command. These entry points converge on Vox's smart factorization machinery, so they are route comparisons rather than three independent factor algorithms.

## Other routes

| Route group | Result |
|---|---|
| 15 fixed-word G membrane executables | Four emitted the same equal candidate twice with verified=false. One, instant_read, timed out at 20 s. The other ten returned no pair within 25,000 frontier steps. |
| G factor_membrane factor N | Timed out after 20 s in native-word Hensel/range-prune propagation. |
| Vox baked 2-adic membrane | Build 38.662 s; execution reached the 60 s timeout without a report. |
| G shor_order and quantum_phase | 6.441 s and 6.327 s; each reached its 500,000-squaring limit without a cycle. |
| G ecm_extract N 100 1000 | Timed out after 20 s. |
| G inclusive ququart/native radix-four route | Fresh-source preparation reached 180 s without producing an executable. The earlier 0.591 s result used a different N and an archived build. Current source's native arm is a controlled error stub. |
| Vox factor-operator resolve N 100000 | 0.1077 s; node budget exhausted with a composite core. |
| Vox factor-operator full N 1000 100000 | 0.1071 s; budget capped with a composite core. |

The four fixed-word emissions are candidate outputs, not factors: their own output says verified=false, and the equal pair is not a divisor pair for N.

Already-baked scripts with other source values cannot consume this fresh N. I included the source-specific 2-adic route and factor_one route because their existing builders accept N and bake it into a new executable.

Vox disassembled the baked winner with the --wiring option. The wiring output names morphism_factor::smart_factor and semiprime_descent symbols in the executable path; it is saved as factor_one.wiring.tsv. The prior inclusive result is recorded in measurements/ququart/increasing_20261004/128/execution.json. Full command records, factors, and logs for this test are beside this report in routes.json and the adjacent .log files.
