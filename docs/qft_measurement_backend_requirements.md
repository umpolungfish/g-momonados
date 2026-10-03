# Compact QFT measurement backend

The requested factorization path must accept baked semiprime source words of
arbitrary width, including unstructured inputs, without enumerating a growing
modular residue state. Factoring checks must use semiprimes of at least 128 bits.
Completion has not been demonstrated.

## Current execution

The baked entry in `src/factor_phase.rs` follows a different execution path:
`FixedPointQuantumMembrane::prepare_structural_execution` feeds
`Program::measure_factor_pair`, which runs `FactorPhaseOracle` over a
`FoldedRegister`. That register stores shared decision branches in a
`BTreeMap`-interned arena and applies Hadamard, phase, and measurement
operations to those branches. Its work and storage therefore depend on the
decision graph produced by the factor-pair oracle. This path does not consume
the measured phase program or emit anyon braid operations.

`RecycledPhaseState` retains one complex amplitude per occupied residue in a
`BTreeMap`. `prepare_branches` constructs and sorts a modularly permuted copy of
the occupied keys. `measure_next_batched` computes both Born masses and then
reconstructs the selected state. Batching limits temporary output buffers; it
does not eliminate work proportional to the occupied residue count.

Vox's `structural_quantum_executor::Program` describes controlled modular powers,
inverse Fourier transformation and measurement. G-mOMonadOS implements Vox's
`Executor` trait with `RecycledCarrierExecutor<C>`, which streams the reversible
gates and requests one measured control bit at each phase step. It still needs a
concrete production `Carrier` that compiles those requests into multi-register
anyon braids and returns fusion measurements. Vox's test-only `InjectedControl`
supplies test bits and does not implement that carrier.

`Carrier::begin` prepares the phase control in `|0⟩` and the modular value
register in the uniform mixture `1/N Σₓ₌₀ᴺ⁻¹ |x⟩⟨x|`. `execute_shot` applies no
fixed `X` seed to that register, so the phase is read from its modular orbit.
This follows the one-control-qubit construction of Parker and Plenio, where the
mixed register selects an orbit and most values have the full order. The current reversible
multiplier still requires `2n+3` clean scratch qubits beside the `n` mixed data
qubits and control. The arithmetic layout therefore needs an in-place or dirty
scratch construction to meet the single-pure-qubit resource condition.
[Parker and Plenio, quant-ph/0001066](https://arxiv.org/abs/quant-ph/0001066).

The reversible arithmetic emitter has a separate width-scaling check. It
streams controlled modular multiplication through an elementary-gate callback,
retains O(n) qubit workspace, and satisfies a 96n² gate bound on unstructured
semiprimes from 128 through 1024 bits for multipliers 2 and N−1. Both dense and
sparse constants emit about 91.6 million gates at 1024 bits, versus about 1.43
million at 128 bits. The test retains no gate list and applies no state-vector
update. Repeating the multiplication for the QFT control powers raises the
circuit count by another factor proportional to n; this check establishes the
arithmetic-emission scaling, not phase measurement or factor extraction.

When phase closure does return a factor pair, `FactorShot::result_pair` now
retains both factors with the source, base, and recovered order. Construction
checks that both arms exceed one and multiply back to the source. This holds the
rejoined result across later reversal steps; it does not supply a measurement
backend or a primality certificate.

## Generated CNOT braid

`anyon_braid_cnot::compile_single_qubit` compiles H, X, and T targets through
the local Fibonacci braid net and attempts dyadic feedback targets with an
exact fixed-point accuracy gate. It offsets the three-strand logical block by
the requested qubit index. A 128-bit semiprime test generated offset H, X, and
both T words. At SK depth one and net depth five, a feedback
phase with a 32-bit denominator selected the identity net word; the compiler
now returns an error for that nontrivial target instead of emitting an empty
braid. At depth three with a depth-seven net, synthesis emitted a nonempty word,
but exact fixed-point projective comparison found it outside the requested
dyadic error. Increasing SK depth to five with the same net still missed that
32-bit tolerance. Feedback precision still needs a synthesis policy that scales
with the QFT register width.

The required compiler path is algebraic. Kliuchnikov, Bocharov, and Svore
approximate the target in the Fibonacci cyclotomic ring `Z[ω]`, complete the
candidate through a relative norm equation, then exactly synthesize the resulting
Fibonacci unitary. Their approximation depth is asymptotically logarithmic in
inverse precision, with probabilistically polynomial runtime under their stated
number-theoretic conjecture. The emitted braid still needs the fixed-point
projective check above before entering the phase circuit.
Primary source: [Asymptotically Optimal Topological Quantum Compiling](https://arxiv.org/abs/1310.4150).

`anyon_cnot_word N [sk_depth] [net_depth] [net_capacity] [exchange_refinement] [minimum_accuracy_bits]`
emits the local target corrections around two refined controlled exchanges as
one six-strand braid word. The optional accuracy floor rejects a word when its
source-width CNOT residual does not reach the requested number of bits. The
local net is built from the generators
recoupled into the pair basis used by the CNOT target. The complete emitted word
is reevaluated in source-width fixed point after recoupling before its residual
is reported. `anyon_cnot_verify N <report> [minimum_accuracy_bits]`
reevaluates a saved word and enforces the requested floor independently.

For the 128-bit semiprime `296650821743515430283258444261036507151`, SK depth
seven with a depth-seven net and a 20-bit accuracy floor produced a
1,425,830-generator word. Both the compile pass and the independent saved-word
verifier measure 24 residual accuracy bits.
The same settings at SK depth two fail the 20-bit floor. Synthesis depth eight
did not return within the run window; the accepted measurements use depth seven.
This gives a width-independent generated braid with measured finite precision,
while the feedback-controlled arithmetic carrier and full factor extraction
remain the downstream execution path.

Depth diagnostics on the same 128-bit semiprime show the word-growth cost of
the current compiler. With a depth-seven net and exchange refinement two, SK
depths two, three, and four produced 1,894, 3,708, and 12,850 braid generators
at 4, 5, and 7 residual accuracy bits. Depth four with a depth-eight net
produced 15,238 generators at 8 bits. A depth-nine net capped at 100,000
entries did not improve the depth-three result: it produced 5,248 generators
at 5 bits. The accepted 24-bit setting remains 1,425,830 generators. The
current synthesis path therefore has no demonstrated short CNOT word at the
precision needed by the repeated modular arithmetic circuit.

The compiler now tries ordinary Solovay–Kitaev synthesis first and invokes
split/fuse only when the requested residual-accuracy floor rejects that word.
At depth four, split/fuse reaches 9 bits with 24,672 generators on both the
128-bit and 192-bit sources, while ordinary synthesis reaches only 7 bits on
the 128-bit source. At depth seven, ordinary synthesis meets the 20-bit floor
with 24 bits and 1,425,830 generators. Split/fuse at that depth reaches 27 bits
but expands to 4,357,176 generators, so the floor-gated fallback preserves the
shorter word whenever it already satisfies the caller's accuracy requirement.

The bounded direct search `anyon_cnot_compile` also ran on this 128-bit source
with depth 64, beam width 16, and an 8-bit target. It exhausted its budget; its
best word retained large computational and leakage residuals. The controlled
exchange construction remains the usable CNOT synthesis route.

The spectral carrier is also insufficient by itself:
`FixedPointQuantumMembrane::modular_phase` calls
`FixedPointSpectralConstruction::modular_branch`, which calls the tape-native
`hadamard_gate::modular_phase_power`. That function evaluates one addressed
residue by repeated squaring. It does not calculate the conditional inner
product below. The historical `measure_and_descend_arithmetic_reference` is
test-only and calls `resident_order` before creating its sample; it cannot
serve as the requested production measurement backend.

## Required calculation

For modular multiplication U and an unnormalized selected target state psi,
the next ideal branches are `(I + c U^s) psi` and `(I - c U^s) psi`, where c is
the feedback phase and s is the current power of two. Their squared norms are

    M+ = 2 <psi|psi> + 2 Re(c <psi|U^s|psi>)
    M- = 2 <psi|psi> - 2 Re(c <psi|U^s|psi>)

A replacement must evaluate these correlations and update the conditional
state without expanding the operator product into its residue amplitudes.
These formulas describe ideal arithmetic; matching the existing rounded
fixed-point simulator requires a separate precision/error argument.

Storing the operator product compactly leaves this norm calculation unresolved.
Computing the order first and using it to generate phase samples would also
leave the requested order-measurement task unresolved. Fixture factors and
known periods must never supply the measurement result.

## Matrix product state assessment

Dang, Hill and Hollenberg's optimized simulation dynamically rearranges the
matrix product state and measures qubits as early as possible. Section 5.2
still requires lower-register occupancy r before its measurement; afterward
the largest bond rank is the odd component beta of r. Its storage therefore
remains dependent on the modular order. Implementing this method alone does
not establish elimination of state-growth work for arbitrary semiprimes.

Primary source: [Optimising matrix product state simulations of Shor's
algorithm](https://arxiv.org/html/1712.07311v4), Sections 4 and 5.2.

## Evidence needed for completion

- An executable compact correlation/measurement method, with its computational
  cost stated rather than inferred from the size of the program description.
- Correct conditional measurement probabilities and an explicit treatment of
  numerical precision across arbitrary input widths.
- Complete factor extraction on independently generated unstructured semiprimes
  of at least 128 bits; short circuit prefixes are insufficient evidence.
- Verified multiplication of returned factors to the baked input, without
  fixture factors entering the execution path.

No factoring test or quantum job was run for this assessment.
