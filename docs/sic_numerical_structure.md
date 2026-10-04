# SIC numerical structure

The SIC module implements operator analysis and synthesis. Its central invariant
is `fuse(split(X)) = X`. `SicCertificate::measure` reports the largest Frobenius
residual on every real and imaginary matrix unit, alongside independent
normalization, equiangularity, completeness, projector purity, duality,
positivity, and second-moment symmetry residuals.

## Qubit coordinates

`QubitState` stores a checked Bloch vector. `TetraSic` uses the ordered directions
`(+++)`, `(+--)`, `(-+-)`, `(--+)`, divided by sqrt(3). `SicVertex` labels them
N, T, F, B. These labels are distinct from `belnap_residual::V` and from the
continuous `SicDistribution`; a nonzero B coordinate does not classify a logical
value as B.

`truth_measure` reads the Boolean z axis; `truth_measure_axis` accepts another
unit axis. `sic_measure` produces the four informationally complete coordinates.
`Simplex4` checks finite, nonnegative, normalized weights. `QubitSicState` also
checks the quantum region and its reconstructed Bloch radius. The checks permit
numerical deviations within the selected tolerance of 1e-12. Values are not
silently clamped.

The optimized qubit split uses `(1+r dot n_i)/4`; fuse uses `3 sum p_i n_i`.
Dephasing reconstructs the Bloch vector, attenuates its transverse components,
then analyzes the resulting state. Urgleichung evaluates `3p_i-1/2` as signed
dual coefficients and preserves negative coefficients.

## Operator frame and backend separation

`Sic` supplies dimension-independent split and fuse operations on complex
operators. `SicCoordinates` retains complex coordinates for arbitrary operators;
it is separate from a physical real probability distribution. `SicFrame`
explicitly stores projectors, effects Pi/d, and duals `(d+1)Pi-I`.

The floating-point backend uses f64 complex scalars. `sic::fixed` supplies
deterministic fixed-point FFT and WH overlap evaluation. `sic::exact` supplies
the number field Q(sqrt(3), i), with reduced rational coefficients and a separate
positive-sqrt(3) numerical embedding. Its tetrahedral certificate checks frame
identities and operator reconstruction using exact arithmetic.

Numerical residuals pass to a separate evidence policy. A threshold gap or
missing reading yields N. `FourEvidence` indexes records by proposition and
independent source. Updating one source replaces its earlier reading; B requires
support and refutation from distinct retained sources for the same proposition.
The executable's FOUR assessment refers to numerical operator closure.

The earlier `anyon_fusion_kernel::SicProjection` retains visible-sector weights
on a carrier with unread mass. It is not a `QubitSicState`, whose four
probabilities must sum to one. There is no implicit conversion that drops the
unread arm.

## Weyl–Heisenberg convention

The global convention is `omega=exp(2pi i/d)`, `tau=-exp(pi i/d)`, and
`D[p,q]=tau^(pq) X^p Z^q`, with `X psi[n]=psi[n-1]`. Its overlap formula is
`chi[p,q]=tau^(-pq) sum_n conj(psi[n]) psi[n-p] omega^(qn)`.
The negative phase factor follows from the stated operator order. Using a
positive phase with this same tape would implement a different ordering.

`WhSic` stores one fiducial and produces individual displaced views/projectors
on demand. `OverlapField` is a separate first-class object. Overlap evaluation
uses d positive-sign FFTs, with radix-two FFTs and Bluestein convolution for
other lengths. CPU, fixed-point, and CUDA implementations use this same
convention and O(d squared log d) arithmetic scaling. They do not retain a
collection of d squared dense projectors. Full operator certificates cost more
than overlap verification and are explicit operations.
Integer winding indices are reduced before floating-point embedding, and the
CPU and CUDA paths share the same displacement and chirp phase helpers.

## Executable and tests

Run `cargo run --release --bin sic-tool -- qubit` for the complete qubit report,
or append three Bloch coordinates. `g-momonados sic` prints the same qubit frame
invariants. `sic-tool wh fiducial.json` evaluates a fiducial supplied as an array
of `[real, imaginary]` pairs. `wh-gpu` selects the CUDA overlap backend.
The WH command evaluates the complete operator certificate through dimension
eight; larger inputs explicitly report overlap-only verification. Exact status
remains absent for arbitrary supplied WH fiducials.

Run `cargo test --release --lib sic::tests` for geometry, pure/mixed/random-state
retraction, physicality, dephasing, Urgleichung, exact arithmetic, lazy WH frame
identities in dimensions two and three, FFT convention, and reproducible fixed
overlaps. The CUDA comparison requires an available device and is run separately
with `-- --ignored`. Readings are retained in `measurements/sic_checks.log`,
`measurements/sic_gpu_checks.log`, and `measurements/sic_build_checks.log`.
The executable and runnable test binary are retained in `measurements/sic/`
with their SHA-256 checksums. The rebuilt machine executable is retained in
`measurements/anyon-compilation/`.
WH coordinates use lexicographic `(p,q)` order; the tetrahedral qubit
distribution uses `(N,T,F,B)` order. Arrays must be interpreted in their frame.

SIC coordinate retraction does not by itself compute the conditional state of a
large entangled factoring register. The anyon backend still needs that global
state evolution and contraction. The SIC structure provides executable local
analysis/synthesis maps for that work.

### Immutable tetrahedral frames

`TetraSic::canonical()` and `TetraSic::with_vertices()` share one checked construction path. Finite unit vertices must have pairwise inner product −1/3 and sum to zero before projectors, effects, and duals are derived. Geometry and operator caches expose read access only. A replacement frame is constructed as a whole.

`QubitSicState` retains the geometry used to analyze it. Its reconstruction and dephasing therefore preserve the selected frame, including rotated tetrahedra. `QubitSicState::new` interprets raw probabilities in the canonical frame.
