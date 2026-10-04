# Ququart SIC and SIXTEEN_3

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
