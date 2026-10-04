# Ququart SIC and SIXTEEN_3

Current factor-extraction acceptance uses only RSA-style unstructured
semiprimes over 200 bits. Structured controls, small-factor inputs and
sub-200-bit sources below are historical records and supply no qualifying
factor-extraction evidence. The current prepared input is RSA-100 (330 bits),
with base two and no known factors or order in its executable preparation.
The source audit is `measurements/ququart/rsa100_input_audit.json`.

`sic-tool ququart` executes a cyclic Weyl–Heisenberg SIC in dimension four.
Its sixteen weights are indexed by the SIXTEEN_3 carrier masks. The frozen
mask layout is bit zero t, bit one f, bit two T, bit three F. The displacement
index is `(mask / 4, mask % 4)`. This indexing is an explicit coordinate
convention. The information, truth, and constructivity orders are available on
each outcome through `SixteenOutcome`.

The frame uses Appleby's analytic dimension-four fiducial, equation 149 of
`quant-ph/0412001`, with the existing kernel's displacement convention.
Projectors, effects, and duals are immutable derived caches. Analysis gives
sixteen weights and synthesis reconstructs the operator as
`5 sum_i p_i Pi_i - I`. Checked weights require a positive reconstructed
density operator, as well as finite, normalized nonnegative weights.

The state basis is T,F,t,f. The three evaluator arms have ranks one, one, and
two. Conditioning on the informational arm retains its t/f coherence.
The signed Born reconstruction coefficients are `5 p_i - 1/4`.

The ququart tests measure completeness, equiangularity, positivity, duality,
and reconstruction on the complete complex operator basis. Additional pure
state checks cover reconstruction, purity, and direct Born probabilities.

This frame is the local coordinate structure for the anyonic factoring path.
Its numerical certificate does not by itself establish phase extraction or
end-to-end factorization at the requested semiprime widths.

`sic-tool anyon-ququart <N> [exchanges]` executes the existing six-anyon
Fibonacci exchange algebra on a four-channel carrier, retaining the fifth
fusion channel. It reports sixteen integer SIC masses indexed by carrier
mask and an additional outside-carrier mass. Readout draws from these masses
with integer rejection sampling. The fixed SIC fiducial uses integer square
roots and rational winding phases at source-dependent precision.

`QuquartPhaseReadout` appends four-pole measurements as base-four digits and
passes the resulting numerator and denominator to the existing phase closure
accumulator. A SIC mask is a sixteen-outcome reading and is not automatically
a base-four phase digit. The carrier exposes both measurement operations.
Tests exercise all sixteen SIC intervals and all four phase digits using
the semiprime source-width controls at 128, 256, 512, 1024, and 2048 bits.

The radix-four executor replaces binary H operations with the Z4 Fourier
transform `F4[k,l] = exp(2 pi i k l/4)/2`. Feedback is the complete diagonal
`exp(-2 pi i k n/4^m)` for k=0,1,2,3. At stage j the work register receives
`base^(k*4^j)`. The streamed reversible implementation applies
`base^(4^j)` controlled by the low lane and its square controlled by the high
lane. Both lanes act on the same work register, with clean arithmetic scratch.

`QuquartFactorExecutor` schedules these operations against a
`QuquartPhaseDevice`, accumulates its measured digits, and verifies any closed
factor arms using the Gödel multiplication engine. Its scheduling test uses
a recording backend, so it establishes circuit wiring rather than factor
extraction. A native fixed-point implementation of Fourier and feedback
targets exists on `QuquartCarrier`; physical braid synthesis for the complete
four-channel Fourier target and the shared-work device remain required.

The executable accepts native targets `f4`, `f4-inverse`, and
`feedback:<numerator>:<denominator_digits>` alongside exchange indices.
These target operations are reported explicitly as native numerical targets.

`fourier_braid_targets` supplies the entire F4 Clifford+T target sequence,
including controlled-S and the output swap. The inverse reverses the sequence
and conjugates its T phases. `feedback_braid_targets` supplies the two lane
phases with weights one and two. Their matrix tests compare every column with
the full Z4 Fourier operator and all four feedback phases. These targets use
the existing calibrated braid compiler vocabulary; the tests validate the
target decomposition, rather than a newly synthesized physical braid word.

`g-momonados anyon_ququart_word <N> [sk net capacity refinement accuracy inverse]`
compiles the complete target sequence into adjacent Fibonacci exchanges and
evaluates the resulting six-strand word. It reports projective computational
error, outside-carrier leakage amplitude, and unitarity error. The command
rejects a word whose complete measured residual exceeds the requested
accuracy. This operation is also available in the Quantum menu and REPL.

## Prepared Fourier membranes

`prepare_ququart.py SOURCE DESTINATION` compiles the complete physical Fourier
braid, bakes its source as a Gödel cell-binary word and its exchanges into a
standalone `membrane` executable, and lifts that executable through Vox. The
emitted IMASM module is recovered from its glyph stream byte for byte.
The destination retains the preparation, executable, module, and manifest.

The membrane takes its input from its compiled preparation. It evaluates the
complete five-channel physical operator, measures the Fourier residual, and
executes the inverse braid to check the return. All intermediate state remains
inside the process. It writes its terminal report after those computations
complete. The positive execution syscall trace contains one stdout write and
no socket connections or send calls. A one-exchange rejection control also
reports only after its computation completes.

The first triple's pair-channel frame exchanges the order of its local
synthesis generators: the local diagonal generator becomes physical sigma_2,
and the recoupled generator becomes sigma_1. The right triple uses sigma_4
and sigma_5 in synthesis order. The fusion-algebra regression test checks
every computational entry of all four generators. Single-gate emission and
the Hadamards used to reverse CNOT direction use this frame.

The retained 128-bit and 447-bit preparations contain the complete Fourier
braid with computational residual 1.22972011e-4. Their requested four-bit
residual threshold is 1/16. The 128-bit executable returns with residual
3.79160691e-77. Its syscall trace records a single final stdout write.

These are the Fourier components used by the radix-four factoring circuit.
`QuquartFactorExecutor` supplies the shared-work modular stages, adaptive
feedback, measured phase digits, and Gödel factor-product closure. The next
preparation joins those stages to the retained Fourier component in the
sealed executable.

## Prepared factor extraction

`prepare_ququart.py SOURCE DESTINATION --base BASE` bakes the source, base,
physical Fourier word and measurement seed into
`ququart_factor_baked`. It retains the joint ququart/work amplitudes across
adaptive phase measurements. The executable accepts no runtime inputs and
writes only its completed factor result or completed error.

The shared-work backend applies modular powers as exact numerical
permutations and feedback as fixed-point winding phases. The Fourier operator
comes from the physical Fibonacci exchanges. Compiling the modular and
feedback operators into physical braids remains required for the complete
anyonic implementation. The manifest records these distinctions.

The measured-phase unit test extracts an order-two control's factors and
verifies their Gödel product. The work device receives neither factors nor a
period. This control establishes the measurement-to-factor path; arbitrary
large inputs still require execution evidence. The work state uses dynamically growing storage without a support cap or
coherent-state truncation. Shots continue until extraction succeeds, with a
BigUint shot counter. No classical factor finder replaces the measured path.

The retained `membranes/ququart_factor_control_128_20261004/membrane`
extracts 3 and 85070591730234615865843651857942052861 after two measured
shots. The independent audit decodes both factor words and checks their
product against the prepared source. Its syscall trace contains one final
stdout write and no network calls. The retained 447-bit base-two membrane
completed with a support-budget error at 4096 entries; it extracted no
factors. Both are runnable, input-free executables with Vox modules and
verified glyph round trips. That 447-bit input is not a semiprime and its
execution was stopped when the qualifying-input requirement was clarified.
Qualifying RSA factor extraction remains unverified.

The 4096-entry and finite-shot preparations above are superseded historical
artifacts. The historical `ququart_factor_447_unlimited_20261004` and
`ququart_factor_control_128_unlimited_20261004` directories contain rebuilt
executables without imposed support or shot limits. Local synthesis nets also
grow dynamically; legacy capacity arguments no longer truncate them, and
the heap-headroom stop has been removed. Net depth specifies the requested
synthesis net, rather than a memory budget.

Prepared membranes serialize every source-dependent numerical value as an
IMASM cell-binary word. Signed complex coordinates prefix the magnitude word
with ≺ for negative or ≻ for nonnegative. Precision, accuracy, seed and exchange
count use unsigned words. Diagnostic words preserve the exact IEEE bit pattern.
The raw decimal compiler report and contraction input are preparation records;
neither is embedded in the executable. `prepare_ququart.py` checks the baked
state for non-word scalars before compilation. Coordinate round trips include
negative values and 674-bit values without decimal conversion.

Preparation also runs the Vox control-flow auditor directly on the final ELF
and retains `vox_audit.stdout`, `vox_audit.stderr`, and `vox_words.tsv` next to
that binary. These reports describe lifted machine code. Static T/B/N/F
verdicts do not certify measured phases, terminal factors, or execution time.
The native-word RSA-100 artifact covers 450,978 executable bytes, with no F
verdicts; its factor-producing execution remains unverified.

The factor preparation additionally bakes the entire controlled-power schedule
as `controlled_power_words`, with its native `phase_digits_word`. Powers are
base^(4^j) modulo the prepared source and contain no order or factor. Loading
verifies the source/base recurrence once; shots reuse the retained schedule.
This moves source-dependent schedule construction into preparation without
changing the measured phase circuit or imposing a shot limit.

Completed measured fractions remain resident in the executor until its sole
terminal report. The report includes `phase_samples` as native numerator and
denominator words. `ququart_verify_readout` replays the existing phase closure
against the baked source/base and checks the resulting factor arms, period,
resolution and shot count. This audits a completed measurement ledger; it does
not prove measurement production from static code or from a supplied period.
