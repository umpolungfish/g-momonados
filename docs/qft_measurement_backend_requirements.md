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
gates and requests one measured control bit at each phase step.
`anyon_braid_cnot::factor_semiprime_with_anyons` binds a chosen coprime base to
the resident source, compiles each target to Fibonacci exchanges, executes
shots through a caller-supplied `FibonacciAnyonDevice`, and returns only a pair
whose product closes on the source. Vox's
`FixedPointQuantumMembrane::from_n_with_base` preserves a selected base through
the phase program, so an unproductive orbit can be retried with another base.
The compiler retains source-bound gate templates across completed shots.
`anyon_device::FibonacciGenerator` is the concrete streamed controller adapter.
It sends little-endian source/base words and adjacent Fibonacci exchanges over
the `g-momonados/fibonacci-anyons-v1` newline-delimited JSON protocol. It sends
no phase or factor result to the controller. Each control-fusion request blocks
for the controller's measured `fusion_bit`, which is the bit consumed by the
QFT phase accumulator. `g-momonados anyon_factor N base max_shots socket` runs
the complete membrane through a Unix-domain controller connection and returns
only a factor pair whose product is N.

The controller protocol is an execution contract, not a local source of
measurement values. The transport test checks the 128-bit semiprime-bound
request, exchange streaming, and readout decoding; a live controller run is
required to ground measured phase and factor closure.

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
semiprimes at 128, 192, 256, 512, 1024, and 2048 bits for multipliers 2 and
N−1. The 2048-bit case emits about 367 million gates for each multiplier while
allocating 6148 qubits. The test retains no gate list and applies no state-vector
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

Feedback denominator validation now happens before constructing its power-of-
two denominator and is bounded by the source-derived phase precision
`2 * bits(N) + 8`. Integral-turn feedback is recognized as the identity and
emits no braid generators. A 128-bit semiprime check confirms that the full
QFT denominator width is accepted by the fixed-point phase target and that an
unbounded `usize::MAX` denominator is rejected before allocation. This closes
the width-validation path; it does not yet synthesize nontrivial feedback at
that full precision.

`FibonacciBraidCompiler` caches source-bound CNOT and one-qubit templates and
streams target generators to a caller-provided sink. It retains its local gate
net across distinct feedback targets in one shot, so changing the measured
phase does not rebuild the same exponentially growing search net. Dyadic
feedback fractions are reduced by their common powers of two before target
construction; this preserves the phase and avoids requesting denominator bits
the reduced fraction does not contain. A source-width check confirms that
`12/2^8` and `3/2^6` produce identical fixed-point targets. Its CNOT lowering routes
nonadjacent logical wires with generated adjacent CNOT and SWAP braid words,
then reverses the route. The 128-bit semiprime check confirms repeatable stream
output within a compiler instance, reverse-wire output, generator bounds, and
valid CNOT output through target lowering. This verifies anyon-word generation
and the compiler interface. The streamed controller adapter supplies the
fusion-readout boundary; a live physical controller session has not been run.
The local matrices used for algebra and accuracy checks do not serve as an
execution substrate.

`CompiledFibonacciCarrier<D>` now implements the recycled executor's `Carrier`
boundary by streaming each `BraidTarget` through that compiler to a
`FibonacciAnyonDevice`. It forwards the source-bound register layout and
uniform-residue preparation request, then delegates control fusion readout,
finish, and abort to the device. The 128-bit semiprime adapter test checks that
the requested register sizing reaches the device and that an X on logical wire
7 generates only that wire's adjacent strand exchanges. This is the executor
integration point; the controller wire adapter exists, but live controller
execution remains unverified. Feedback rotations retain
their full source-width dyadic angles, while braid approximation now uses an
approximate-QFT error budget. For an `m`-bit phase tape, each feedback unitary
is compiled to error at most `1/(64m)`, bounding the accumulated operator
error across at most `m` corrections by `1/64`. This reduces the required
synthesis tolerance from O(m) bits to O(log m) bits without shortening the
measured phase tape. A nontrivial feedback rotation with a 264-bit denominator
compiled for a 128-bit semiprime at the derived 15-bit gate tolerance. That
single-correction result does not establish the full phase circuit or factor
closure.

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
is reported. `anyon_cnot_verify N <report> [minimum_accuracy_bits=0] [eps=1e-6] [reject=1e-2]`
reevaluates a saved word and enforces the requested floor independently.
It also reports the computational and leakage Belnap values and their tier.
Computational classification includes the maximum of the computational and
unitarity residuals; leakage classification uses the maximum leakage amplitude
component. Both comparisons operate directly on the BigUint residuals and the
fixed-point scale. The finite floating-point thresholds are interpreted as
exact dyadic rationals, so comparison requires no residual conversion or
truncation. Thresholds must satisfy `0 <= eps < reject`. Zero clean tolerance
therefore affirms only an exactly zero residual, including at 2048-bit source
width. The caller's tolerance-based tier is reported alongside the raw residuals
and the independently enforced accuracy floor.

Exact threshold checks cover source widths 128, 256, 512, 1024, and 2048,
including a positive one-unit residual at zero clean tolerance and comparisons
at both band boundaries. Subnormal thresholds and invalid configurations have
separate checks. Saved-word verification of the 24,672-generator CNOT for the
128-bit source `296650821743515430283258444261036507151` reaches nine residual
accuracy bits. Its default Belnap verdict is computational `B`, leakage `T`,
tier `Inconsistent`. With caller thresholds `eps=0.002`, `reject=0.01`, its
verdict is `T,T`, tier `Terminal`. These are tolerance-dependent classifications
of the same raw residual; the braid's residual remains positive.

The split/fuse CNOT compiler was run on unstructured semiprimes at 128, 256,
512, 1024, and 2048 bits. At every width it emitted 24,672 Fibonacci generators
and measured nine residual accuracy bits. The debug sweep took 270.09 seconds;
the width-dependent fixed-point compilation is the measured cost. These runs
verify CNOT word generation at each source width and do not execute phase
feedback or factor extraction.

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

## Recursive arbitrary extraction

`arbitrary_anyon_factor <natural|canonical-cell-binary-word> <base> <max_shots> <unix_socket>`
connects the same controller adapter to the recursive extractor. For each
composite descendant of at least 128 bits, it first executes the source-bound
anyon phase program. A returned candidate must satisfy exact division and
Gödel multiplication closure before both descendants are processed. Its route
trace records the phase base, recovered order, and number of shots. Descendants
below the controller's minimum width use the native route ladder and its
continuation paths.

A completed shot budget without a factor is represented as `None` by
`try_factor_with_anyons`; the extractor records that outcome and continues the
ladder. Compilation, transport, and readout failures remain errors. A candidate
that fails Gödel closure is discarded and the ladder continues. The direct
`anyon_factor` command retains its error on an unclosed shot budget. The new
command is available from the CLI, REPL, and menu.

The recursive interface checks use explicit candidate fixtures on a balanced
128-bit source and on a 256-bit fourth power. They verify candidate closure,
continued descent through composite candidates, invalid-candidate fallback,
unclosed-budget fallback, and preservation of device errors. These checks
exercise integration and verification; they supply no device phase evidence.

`arbitrary_factor` recursively processes both descendants of a verified split
and preserves stripped prime-power multiplicities when combining its result.
Every route candidate must pass exact division and Gödel multiplication closure
before it is returned by the ladder. An invalid candidate falls through to the
next available route. Final closure includes the multiplicities of every prime
factor.

The 128-bit regression cases cover a prime square, an unbalanced semiprime with
a 21-bit prime and a 107-bit Mersenne prime, and a composite with 39 powers of
two beside an 89-bit Mersenne prime. The unbalanced case is labelled explicitly
as such. The balanced, unstructured 128-bit source
`296650821743515430283258444261036507151` closes through the native PARI
candidate route as 16925480323643806501 × 17526877587580975651. The standalone
tool reports protocol PASS, conjunctive reconstruction PASS, and closed Gödel
product reconstruction. The measured release command completed in 0.10 seconds.

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

The width sweeps verify reversible arithmetic emission and Fibonacci CNOT word
generation. The controller adapter now connects measured fusion bits to the
phase accumulator. The local transport check uses a protocol response fixture;
it is not a device measurement or factor closure.
# Semiprime tool extraction

`semiprime-tool` uses the shared `arbitrary_factor` route ladder after acquiring
the structural sieve reading. The ladder tries the sieve, an immediate square
bridge, the local PARI factor engine, bounded difference of squares, winding
bridge, congruence sieve, order winding, and Brent rho. The PARI route invokes
`gp` with N alone and reads one factor candidate. Every
accepted split closes through Gödel multiplication. A two-prime result includes
prime squares by counting multiplicity. Exhausting the initial ladder advances
Brent seeds and work budgets alongside an exact divisor lane. The retry record
occupies one trace slot, and the walks retain no state history. A successful
report requires full source reconstruction through Gödel multiplication. The
original balanced 128-bit source closes as
16925480323643806501 × 17526877587580975651 through the PARI candidate route.

`factor_routes::congruence_split` retains one modular relation per factor-base
pivot. Each row carries X, Y, and parity with
X² ≡ Y² ∏(odd base primes) modulo N. Combining rows moves their shared odd
primes into Y. The basis therefore retains at most k rows with k parity bits
and two modular integers per row, independent of the number of trials. Dependent
rows are processed immediately. A 130-bit semiprime test closes a nontrivial
split from two square-root relations; repeated square relations on a 128-bit
semiprime leave the pivot storage empty.

`measurements/anyon-extractor-width-controls.tsv` carries independently drawn,
balanced semiprimes at 128, 256, 512, 1024, and 2048 bits. Its companion
`.proofs.jsonl` carries prime certificates verified by `qpe_semiprime_cases
--verify`. The extractor test supplies their certified p candidates through the
anyon callback and checks recursive factor output and Gödel product closure at
each width. This is a candidate-interface test. Device phase measurements use
the controller execution path described below.

The release executables are retained in `measurements/anyon-compilation/` as
`g-momonados.elf`, `semiprime-tool.elf`, and `prime-tool.elf`, with checksums in
`tool-elf-sha256.txt`. The standalone prime-square and original balanced 128-bit
readings are retained in `measurements/semiprime_tool_prime_square_128.log` and
`measurements/semiprime_tool_unstructured_128.log`.
