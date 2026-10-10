# Source-bound phase pipeline wiring status

Target: the exact source in `source.decimal` (895 bits). The baked executable
uses its prepared IMASM operator, folded ququart work register, SIC readout,
phase reconstruction, and nested-radix closure. The native factor-producing
arm remains disabled for this run.

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

The native Vox trace reads four `apply_nested` operation counters. In the live
exact-source run, Toggle and ModularAdd are active; Add and Compare remain at
zero. The indirect jump-table paths for all four variants are present in the
compiled symbol, but those last two variants have not been exercised by this
phase schedule. Therefore the Diagrammar gap concerns static branch coverage;
it does not block the phase work that is currently executing. The continuing
modular-add path has reached 3 of 899 scheduled phase digits. Its current
profile is in `vox_native_roundtrip_release.*`; this is partial progress, not a
factor result.

The phase-analysis stage is `PhaseReadoutAccumulator::close_fraction`: it
filters continued-fraction convergents by the measured-bin error bound, combines
only denominators supported by measured evidence, and checks the candidate
period with exact modular exponentiation before attempting factor closure. The
live run has reached 3 of 899 scheduled digits; no period or factors have been
certified yet.

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
bits. The 895-bit run remains active. Completion still requires a phase-derived
candidate, the Lean exact-product certificate, and an independently checked
integer multiplication.
