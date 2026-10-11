# Source-bound phase pipeline wiring status

Target: the exact source in `source.decimal` (895 bits). The baked executable
uses its prepared IMASM operator, folded ququart work register, SIC readout,
phase reconstruction, and nested-radix closure. The native factor-producing
arm remains disabled for this run.

## Packed double SIMD

The resumed rewritten 128-bit control completes both shots and exits zero.
The independent readout verifier accepts its factor product and certified
residue receipt. Every factor word, residue word, phase sample, and SIC sample
matches native output byte for byte. The sole different field is the Fourier
computational diagnostic: native reports `2.0665914735611586e-16`, and the
earlier Vox runner reports zero.

A static musl probe isolates missing `xorpd` execution. The decoder retains
the instruction, but the VM omitted packed-double logical operations from
SIMD dispatch. The repaired runtime handles XOR, AND, AND-NOT, and OR across
both lanes while preserving integer flags. All twelve SIMD tests pass.
All fifteen floating-point probe cases now match native output byte for byte.
The target and full control tapes are replaying under `vox_runner_simd`.
The native target profile remains running through its original static carrier.

The bounded `runtime_reclaim/diagnostic_probe` imports the same Fourier
contract and includes the same prepared 128-bit control. Under the combined
repair, all four diagnostic fields match native execution exactly, including
computational residual bits `3cadc86076325b4c`. The static allocator probe
also matches native output under this runner. Verification receipts and
runner hashes are retained beside the probes.

## Guest page reclamation

The resumed target's native instruction samples repeatedly reach musl
`munmap`, allocation, and `DecisionArena::reclaim_pending`. Vox previously
reported successful `munmap` without removing the discarded guest pages.
The runtime now removes the page-rounded interval and rejects unaligned,
empty, or overflowing requests. Five dynamic-page tests pass, including
neighbor preservation, repeated unmapping, fresh writes after reclamation,
and the existing musl growth-remapping checks. The long tape executions
started before this repair retain their original runtime. Their progress
does not establish the repair's effect on target performance.

The repaired runner also completes the static musl allocation probe under
`../modular_residue_20261010/runtime_reclaim/`. Its complete stdout matches
native execution byte for byte. Independent checksum sums agree, and live
neighbor buffers survive repeated vector growth, release, and allocation.
This verifies runtime transport independently of the factor search.

The target retry validates all 899 prepared powers and enters phase execution.
Across two snapshots, modular calls advance from 35 to 96 while materialized
guest pages fall from 12,572 to 11,613; the folded arena retains 13,446 nodes
at both samples. The earlier runner has 126,587 materialized pages after 412
modular calls. These are different execution positions, so their ratio is
not a measured speedup.

The resumed native target reaches three measured digits. The minimum sampled
post-fold retained-node counts after digits one, two, and three are 10,736,
42,906, and 171,420. Subsequent gates before the next measurement increase
these counts again. `target895_final/phase_node_growth.json` preserves the
sample boundaries. `peak_nodes` also updates after folding, so neither counter
measures the temporary allocation peak inside an arithmetic operation. No
target factor pair has been emitted by these samples.

## Compiled fold cadence

The native target profile samples modular entry/exit boundaries with over
15 million unreclaimed nodes while post-fold retention is about 858,000.
`tools/rewire_fold_cadence.py` identifies the shared successful-operation gate
in `QuquartFoldedWorkDevice::apply_nested`: load the object counter, increment
and store it, compare with eight, and call that object's fold method on
equality. The tool checks the fold method's counter reset and the machine
call target. It copies the ELF and changes the single compare immediate from
eight to one at address `0x325b6`.

The patched 128-bit native carrier completes with all 51,583,828 stdout bytes
identical to the original native carrier. The independent readout verifier
accepts its phase, SIC, factor-product, and residue receipt. Both patched
ELFs pass whole-image rewiring: all 767,288 executable bytes are accounted
for, with 17,669 closed pairs, zero open pairs, and zero unresolved transfers.
This is a control-flow closure check plus a structured-control readout
witness. The patched target's value-level result and performance remain
under evaluation in `target895_final/fold_each/native_profile.*`.

## Lean boundary

`ququart_factor_baked` now calls the shared `p4rakernel/bridge/client.rs`
admission before preparing work. Admission requires the compiled kernel to
report SIXTEEN_3 active and checks the Lean `bridgeHold` re-entry, Y reduction,
and least/greatest fixed-point certificate. Once the phase executor returns a
candidate closure, the executable sends the decimal source and decoded factors
through the Lean `product` command. It reports completion only when Lean
certifies `p*q = source` and both factors exceed one. This adds an independent
kernel arithmetic boundary; it does not supply phase evidence or factors.

The bridge was exercised directly on `Y 6`, which returned the SIXTEEN_3,
re-entry, and certified fixed-point report. Its product path rejected the
non-factor control pair `(2, 3)` for the exact source. The baked release target
compiles with the bridge wired in. The exact-pair success path remains
unverified until the phase executor produces a candidate.

## Vox wiring coverage

The native Vox trace reads four `apply_nested` operation counters. In the archived
exact-source run, Toggle and ModularAdd are active; Add and Compare remain at
zero. The indirect jump-table paths for all four variants are present in the
compiled symbol, but those last two variants have not been exercised by this
phase schedule. Therefore the Diagrammar gap concerns static branch coverage;
the archived phase work executed the modular-add path. That
modular-add path has reached 4 of 899 scheduled phase digits. Its current
profile is in `vox_native_roundtrip_release.*`; this is partial progress, not a
factor result.

The phase-analysis stage is `PhaseReadoutAccumulator::close_fraction`: it
filters continued-fraction convergents by the measured-bin error bound and
combines only denominators supported by measured evidence. Candidate closure
uses the shared certified modular residue step. It first verifies
`base^period ≡ 1 (mod source)`, then evaluates each requested tick at its
residue modulo that period. The same step serves phase, SIC, trilattice, and
native-width winding factor paths. The archived trace reached 4 of 899 scheduled
digits.

Native instruction samples from the exact-source trace resolve most often to
`DecisionArena::modular_digit_add::visit`, followed by `reclaim_pending`,
`FixedComplex::mul`, and exact `BigInt::add`. This locates the cost in the
coherent decision-diagram arithmetic and reclamation beneath nested modular
addition, rather than in the omitted Toggle/Add/Compare dispatch paths. The
leaf interning index now uses a small byte hasher; complete `Node` equality
still checks both exact amplitude integers. Exact arithmetic tests pass and the
optimized release carrier builds with this change; the already-running trace
uses its earlier executable, so runtime speedup remains unmeasured.

The current release build also precomputes place values and modular sums for
digit widths up to eight bits, avoiding repeated BigUint modular reductions at
each recursive diagram node. Wider digits retain the prior general recursion.
A new 14-wire coherent-state test covers enabled and disabled controls, wrapped
translation, invalid work values, and non-real amplitudes. All nine exact
decision tests pass. The long exact-source trace predates both optimizations;
their effect on its run time remains to be measured in a subsequent run.

## Arithmetic checks and next evidence

The release path no longer repeats full source-cofactor and interval
partition traversals solely for round-trip assertions. Those checks remain
active in debug builds, and all nine tests in `tests/ququart_decision_exact.rs`
pass, including exact coherent-amplitude modular translations at 200 and 256
bits. Completion requires a phase-derived
candidate, the Lean exact-product certificate, and an independently checked
integer multiplication.

## Prepared-base startup validation

`scaled_modular_base` previously built and discarded a complete ququart power
schedule just to establish that the scaled base was a unit modulo the source.
The baked executor then checked the supplied schedule again at
`QuquartPowerSchedule::from_prepared`. The current implementation performs the
unit check with an exact gcd and retains the full per-entry schedule check at
the execution boundary. The source-bound release target builds with this
change.

The previous cleanup commit had removed
`measurements/anyon-extractor-width-controls.tsv`, which several library tests
include at compile time. I restored that test fixture from its pre-cleanup
revision. The exact batched-twiddle parity test and the SIC phase-pole test
both pass.

Vox now records a pipeline stage and the count of validated prepared powers.
The codes are 0 for startup, 1 for Lean admission, 2 for parsed input, 3 for
Fourier validation, 4 for scaled-base validation, 5 for prepared device, 6 for
decoded powers, 7 for validated schedule, and 8 for phase execution. The run
`vox_native_pipeline_stage_release`, built before the SIC table change, was
still at stage 4 after 263 seconds. The exact-source run
`vox_native_sic_table_release` reached stage 5 by its 32-second sample and
was there while decoding the prepared powers. This directly confirms that
the batched SIC phases pass the previous device-construction bottleneck. The
recorded sample precedes phase-counter initialization.

## Certified residue execution

`p4rakernel/p4ramill/Imscribing/Paraconsistent/CertifiedModularResidue.lean`
proves the shared identity for arbitrary natural ticks, bases, moduli, and
positive certified periods: the reduced modular power equals the unreduced
power. It also proves the residue bound, equality for matching residues, and
period-multiple jumps. A certificate binds the base, modulus, and period and
requires the period power to equal one modulo the source. The proof accepts
nonminimal periods. The extended re-entry audit passes; its axiom report lists
the standard `propext` dependency for the power identity and period jumps.
The bound and matching-residue theorems have no axiom dependencies. This is a
conditional arithmetic proof; the factoring executor supplies the period.

`Vox/src/modular_residue.rs` supplies the shared operation for BigUint, native
integers, and binary numeral tapes. The certificate privately binds the base,
source, and period. It verifies `base^period mod source = 1` once and evaluates
each tick at `tick mod period`. Phase, SIC, winding, QFT, GPU Shor, Belnap,
coupled, and Hadamard period closures use this operation. Candidate-period
discovery precedes certification.

The measured base stays attached to ququart extraction and its residue receipt.
The baked executable evaluates tick `1000000000000000001` after phase closure
and Lean product certification. `ququart_verify_readout` checks every receipt
field against the measured base and period and evaluates the unreduced exponent
independently. Corrupting each receipt field separately is rejected.

The static 128-bit control under `../modular_residue_20261010/control128/`
produces `18446744073709551615` and `18446744073709551617` from two phase
measurements, with measured period 2. The source is
`340282366920938463463374607431768211455`. The huge tick reduces to index 1 and
value `18446744073709551616`. The independent readout verifier accepts the
phase, SIC, nested closure, product, and residue receipt. This source is a
structured composite execution control.

The entire static control and 895-bit image pass the rewirer's interprocedural
Frobenius control-flow check with full instruction coverage. Their rewritten
IMASM tapes retain the indirect expressions and call-return frames. The baked
readout emits canonical IMASM words for both factors and every residue receipt
value.

The VM pointer-write trace identifies musl arena growth as the source of the
first-shot stall. `mremap` fell through the syscall handler and returned zero.
Musl added the allocation's internal offset to that address and replaced the
arena node pointer with `0xa80`, so Born-mass evaluation multiplied ELF bytes as
amplitude integers. The trace records the write at guest PC `0x5e4f6`, step
18918086, after the preceding node pointer was `0x38015aa80`. The actual guest
call chain reaches `DecisionArena::mass::visit` and `measure_phase_digit`.

Vox now resizes anonymous guest storage through `mremap`, preserving its sparse
pages across moves, growth, shrinkage, and fixed destination replacement. Error
paths preserve the source allocation and return errno. All 14 VM instruction
and memory tests pass, including the musl-sized allocation growth regression.
The same rewritten control tape is the execution check for this repair.

Factoring inputs use isolated source documents. `../unstructured_semiprimes.decimal`
contains only the extracted decimal n column, one value per line. The CUDA
prefix comparison reads this file directly. The 895-bit prepared operator is
bound to its isolated source word and includes no factor or period fields.

## Word readout transport

The Vox process runner preserves guest stdout byte for byte and propagates the
guest exit status. Runner halts use stderr and exit status 2. Successful and
nonzero-exit controls emit the same canonical word without a decimal status
suffix. Concurrent executions reserve separate Lean capture files through
exclusive creation, including when separate PID namespaces reuse a PID.

The final static control produces the same word readout as its preceding build.
`ququart_verify_readout` accepts that output, including the measured-base
residue receipt and exact factor product. The execution records are
`../modular_residue_20261010/control128/continued_native_checks.json` and
`continued_native_verification.log`. The rewritten control and target tapes
carry the final static bridge and execute through `vox_runner_final`.

## Native sample resumption

Vox advances one instruction across each native sample stop before continuing
the process. The control completes under this sampler in 6.591 seconds, emits
a byte-identical copy of its unsampled IMASM readout, and passes the independent
factor and residue verifier. Continuing directly from the sample stop keeps
this control at the completed `wait4` return during Lean admission. Changing
the sampling signal alone preserves that behavior. The retained comparison is
`../modular_residue_20261010/control128/profiler_wait_controls.json`.

## Sequential bounded execution

On the user’s instruction, all six overlapping factor runs were interrupted. New experiments run sequentially with at most 60 seconds per execution. The 895-bit target has no completed factor readout.

A checked one-byte binary rewire changed the shared fold cadence from eight to four completed nested operations. The structured 128-bit control completed in 7.44 seconds with its full output byte-identical to the original. A separate sequential original run completed in 6.13 seconds. This single comparison does not support a speed improvement, so the four-operation cadence is retained as an experimental artifact rather than promoted as an acceleration.

## Allocation-free runtime evaluation

Vox now parses effective-address fields without a temporary vector and stores flag interpretations without allocating a string on each arithmetic instruction. The compiled factor carrier is unchanged. All twelve arithmetic/SIMD checks passed. Three interleaved sequential static allocator-probe trials produced exact native output; median execution changed from 3.339 seconds to 3.225 seconds, approximately 3.4 percent lower. The complete source-bound Fourier-contract diagnostic also remained byte-identical to native and finished in 5.93 seconds. These bounded runtime probes do not establish a speedup or completed factorization for the 895-bit target.

The new preserved runner and receipts are under `measurements/modular_residue_20261010/runtime_address/`. The static flag-mode representation changes the internal Machine layout; existing debugger offset specifications remain tied to their original preserved runners.
