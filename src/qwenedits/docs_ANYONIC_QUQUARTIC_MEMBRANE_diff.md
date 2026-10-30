--- docs/ANYONIC_QUQUARTIC_MEMBRANE.md (原始)


+++ docs/ANYONIC_QUQUARTIC_MEMBRANE.md (修改后)
# Anyonic Ququartic Semiprime Factorization Membrane — build plan

Status: frozen spec, pre-implementation. Revised against THE CODEX FIBONACCI
(`https://pastebin.com/bzsJMxbq`, archived at `docs/CODEX_FIBONACCI.md`) and
the live SIC stack (`src/sic/{frame,certificate,wh,fixed,exact}.rs`,
`src/belnap_residual.rs`).
Carrier: ququart (d = 4). Anyon sector: Z₄ anyons (ℤ₂×ℤ₁₁ gauge data,
topological spin θ = i, twist exponent e = 2 mod 4). Register: the recycled
phase substrate of `phase_unbraid.rs`, widened from base-2 to base-4 digits.
Closure predicate: unchanged — μ∘δ = id via `native_numeral::syzygy_preserves`
plus word-multiplication re-check (`multiply_via_word(&p,&q) == N`).

## 0.A Codex alignment (normative)

The membrane is one more inscription of the Ur-Template
μ∘δ = id, so every layer must present as a split–fuse pair with its own
closure residual feeding the FOUR-valued verdict. The Codex's twelve
primitives map onto the ququart stack as follows (slot order
⟨⊢ ⊣ ≻ ≺ ⋈ ⊤ ∈ ∋ ⊙ ⊥ ⊞ ⊡⟩):

| Primitive | IMASM | Ququart realization |
|---|---|---|
| ⊢ VINIT | dimensionality | `QuquartRegisterLayout::new(m)` opens M = 4^m |
| ⊣ TANCH | topology | readout close / `device.finish()` |
| ≻ AFWD | forward morphism | forward braid exchanges R^{ab} = ξ·i^{ab} |
| ≺ AREV | reverse morphism | inverse exchanges (`exchange(..., inverse=true)`) |
| ⋈ CLINK | composition | Yang–Baxter-verified word composition |
| ⊤ IMSCRIB | self-reference | membrane bakes N into its own code (compile-time inscription) |
| ∈ FSPLIT | δ | SIC analysis coordinates p_i = tr(ρE_i), d² = 16 outcomes |
| ∋ FFUSE | μ | dual-frame synthesis ρ = Σᵢ pᵢ Dᵢ, Dᵢ = 5Πᵢ − I |
| ⊙ EVALT | positive witness | closure residual ≤ ε ⇒ T-evidence |
| ⊥ EVALF | refuting witness | residual ≥ ρ ⇒ F-evidence |
| ⊞ ENGAGR | Brass Vessel | contradictory closure evidence kept as B, no explosion |
| ⊡ IFIX | winding fix | topological spin θ = i residue + final seal glyph |

Three consequences adopted verbatim from the Codex:

1. **The SIC frame is the membrane's numerical ∈/∋.** M1 does not merely
   "verify equiangularity"; it implements `split(X) -> [Complex;16]` and
   `fuse(coords) -> Operator` exactly in the style of `src/sic/frame.rs`,
   and emits a six-part certificate (norm, equiangularity, completeness,
   closure ‖μ_SIC δ_SIC(X) − X‖, positivity, symmetry) per
   `SicCertificate::measure`. For d = 4 the fiducial-overlap field
   χ_{p,q} = ⟨ψ₀|D_{p,q}|ψ₀⟩ with |χ|² = 1/(d+1) = 1/5 for nonzero
   displacements is *exact* in fixed point (Zak phases over ℤ₄ are quartic),
   so `fixed.rs`/`exact.rs` paths apply — no float fallback needed.
2. **FFT evaluation is mandated, not optional.** Per Codex §XIII.9 the 16
   overlaps come from d = 4 length-d Fourier transforms, O(d² log d). This
   shares the radix-4 butterfly machinery with M3's QFT — one partner-tile
   implementation serves both the SIC overlap field and the phase register.
3. **Belnap discipline on the verdict.** The membrane exit code is a Boolean
   *projection*; the ambient structure is the FOUR residual
   `(R_comp, R_leak)` per `belnap_residual::CnotResidual`. Middle-band
   numerics classify as N, never B; B arises only from independently fused
   arms (T ⊔ₖ F). Tier vocabulary: Terminal / UnresolvedLeak / Crowley /
   Contaminated. Note: `src/belnap_residual.rs::tier()` currently folds
   (T,N) into Terminal; the Codex demands (T,N) = UnresolvedLeak distinct
   from (T,T) = Terminal. Fixing that divergence is folded into Stage A
   (small edit + test update); the ququart verdict table then matches
   Codex §XI exactly. The discrete μ∘δ = id on FOUR (idempotence of join,
   Codex §IX) is already witnessed by `fsplit`/`ffuse` in that module and
   gets a dedicated `special_closed()` exhaustively over all 16 residuals.
4. **Data-type non-collapse** (Codex §XIII.5): the Belnap carrier {N,T,F,B},
   the tetrahedral/Clifford SIC frame {Π_N, Π_T, Π_F, Π_B} labeling, and the
   probability vector (p_N,p_T,p_F,p_B) remain three distinct types in code.
   In particular `p_B > 0` must never be wired to semantic `V::B`. Naming
   convention enforced: `BelnapLabel::{N,T,F,B}` (indices only),
   `SicRay(i)`, `SicProbs([f64;4])`.
5. **Protocol words.** The semiprime protocol
   ⊢⊙∈⊤⊥⊞∋≻⋈≺⊡⊣ and prime protocol ⊢⊙∈⊤≻⋈⊥≺∋⊞⊡⊣ (Codex §VI) become the
   two admissible glyph words for the ququart membrane family; M5 mints each
   membrane under exactly one of them, and `membrane_family::family()`
   (`Vec<(&'static str,&'static str)>`, src/membrane_family.rs:179) stores
   the word alongside the tuple line. Descent/return ladder
   (Saturn→Ogdoad, Codex §VII) maps onto the *existing* `arbitrary_factor::Route`
   lanes — verified present in src/arbitrary_factor.rs: `Trivial` (Saturn, ⊤),
   `SieveLane` (Jupiter, ⊡), `DifferenceOfSquares` (Mars, ≻),
   `WindingBridge` (Sun, ⊥), `CongruenceSieve` (Venus, ⋈),
   `OrderWinding` (Mercury, ≺), `Rho` (Moon, ∈); the Codex's eighth rung,
   Gödel `check` (Ogdoad, ∋), is the fusion step that already exists as the
   multiply-back verification. The ququart order-finding lane reports
   `route: Route::OrderWinding` on descent and closes at Ogdoad via
   `multiply_via_word(&p,&q) == N`. One extra lane observed in the codebase,
   `Route::AnyonPhase`, is the Fibonacci predecessor of the ququart lane and
   gets a sibling variant `Route::QuquartPhase` so both inscriptions remain
   individually reportable in the descent ladder.

## 0. Design decisions (frozen)

1. **Ququart charge group is Z₄.** Charges a ∈ {0,1,2,3}, fusion a⊞b = (a+b) mod 4.
   This is the rank-4 group algebra C[Z₄]; its simple objects are the four
   characters χ_j(a) = i^{ja}. The membrane's "anyonic" content is the exchange
   (braiding) phase R^{ab} = ξ · i^{ab} with ξ a fixed scalar twist; metric
   positivity on the S-matrix selects ξ. Chosen because d = 4 = |Z₄| exactly,
   so the carrier Hilbert space IS the charge space — no truncation, no embedding.
2. **Fixed-point arithmetic stays as it is.** `FixedPointFormat` / `FixedComplex`
   already carry arbitrary-width BigInt phases; quartic twiddles are just
   numerators over denominator_bits with an extra factor of 2 in the angle
   grid (i = e^{2πi/4}). No new number type.
3. **The register is digit-4, not qubit-pair.** A ququart tape cell holds one
   base-4 digit; two-qubit pair packing (`anyon_pair::PairMatrix`, 5×5 there,
   4×4 here) is only a *fallback layout* for hosts that lack native d=4.
4. **Membrane semantics unchanged.** Value N baked in at compile time, no
   input at run; exit 0 iff closed pair passes syzygy. `make_membrane.sh` is
   the template.

## 1. New modules and exact insertion points

### M1 `src/ququart4.rs` — the d=4 carrier substrate (~500 LoC)
- `pub struct Ququart(pub u8)` with charge ops ⊞, ⊟, and character table
  `chi(j,a) -> FixedComplex` (exact: i^{ja} as (±1,0) or (0,±1) scaled).
- Weyl–Heisenberg generators X|a⟩=|a⊞1⟩, Z|a⟩=i^{a}|a⟩ as 4×4
  `LocalMatrix`-style arrays (reuse `anyon_local::LocalMatrix` shape, widen
  from [FixedComplex;4] to a new `[FixedComplex;16]` `QuquartMatrix`).
- Exact d=4 SIC-POVM as the membrane's numerical ∈/∋ pair (Codex §XIII):
  - `sic_fiducial() -> [FixedComplex;4]` = (|0⟩+|1⟩+|2⟩+|3⟩)/2;
  - displacement orbit D_{p,q} = τ^{pq} X^p Z^q (τ = −i on ℤ₄), 16 rays;
  - **∈** `delta_sic(&Operator4) -> [FixedComplex;16]`, p_i = tr(XE_i);
  - **∋** `mu_sic(&[FixedComplex;16]) -> Operator4`, duals D_i = 5Π_i − I;
    closure ‖μ_SIC δ_SIC(X) − X‖ must be *identically zero* in fixed point.
  - Overlap field χ_{p,q} evaluated by the FFT route (4 length-4 DFTs over
    a_p(n) = ψ̄_n ψ_{n−p}), per Codex §XIII.9 — reuses M3's radix-4
    butterflies; never materializes all 16 displaced vectors for checking.
  - Certificate struct mirrors `sic::certificate::SicCertificate`:
    (norm, equiangularity, completeness, closure, positivity, symmetry) +
    FOUR classification via `EvidencePolicy`-style thresholds → V.
  - Physical-region guard (Codex §XIII.6): admissibility check Σp² ≤ 1/5·(d
    normalization form) for the d=4 frame; pure states saturate the bound.
- Frame-native Born rule (Codex §XIII.7): `born_in_frame(p, r_j|i)` with
  dual coordinates (d+1)p_i − 1/d = 5p_i − ¼; negativity of intermediates is
  expected and documented, not an error condition.
- Unit tests: X⁴=Z⁴=I, ZX=iXZ, Gram off-diagonal modulus 1/5 (note: tr(ΠᵢΠⱼ)
  = (dδ+1)/(d+1) ⇒ 1/5 at d=4, correcting the earlier draft's 1/4),
  μ_SIC∘δ_SIC = id on a seeded operator basis, |χ_{p,q}|² = 1/5 ∀ nonzero.

### M2 `src/anyon_ququart.rs` — the Z₄ braid representation (~350 LoC)
- Local exchange operator on adjacent charges (a,b):
  R|a,b⟩ = ξ·i^{ab}|b,a⟩, implemented as a 16×16 permutation-with-phase.
  API mirrors `anyon_fusion_kernel::FusionKernel::exchange`:
  `pub fn exchange(&self, left,right,in,out, inverse) -> Result<FixedComplex,String>`
  but over Z₄ charges (validate ≤ 3 instead of ≤ 1).
- Yang–Baxter check: R₁₂R₂₃R₁₂ = R₂₃R₁₂R₂₃ on 3-ququart space (64-dim),
  evaluated with `QuquartMatrix::multiply` / `adjoint`. Test asserts zero
  residual in fixed point.
- Braid word evaluator `evaluate(word:&[i32])` mirroring `anyon_local::evaluate`
  so existing glyph-word plumbing (`braid_protocol::braid_to_imasm`) feeds it.
- Twist exponent bookkeeping: e = 2 (mod 4) ⇒ θ=i; expose
  `pub fn topological_spin(charge) -> FixedComplex`.

### M3 `src/ququart_order_find.rs` — base-4 phase register (~500 LoC)
- Port of the comb→QFT→Born pipeline in `phase_unbraid.rs` with M = 4^m
  instead of 2^q:
  - `QuquartRegisterLayout` replaces `DenseRegisterLayout` (tile index in
    base-4, butterfly partner per stage uses radix-4 butterflies).
  - `ququart_qft_twiddle(...)` wraps `FixedComplex::qft_twiddle` with
    denominator_bits doubled per digit.
  - Modular multiplication x↦a·x mod N as a **permutation of base-4 digit
    states**: reuse `reversible_modular::ModularMultiply` gate emission but
    retarget `CarrierGate`→`QuquartGate { Shift(digit, k), Phase(digit, j),
    SwapCarry(pair) }`. Provide `emit_ququart<F>(&self, multiplier, emit)`.
  - Born shot sampling reuses `anyon_fusion_kernel::sample_born_masses`
    generalized from `&[BigUint;2]` to `&[BigUint;4]` (add the slice-generic
    version there; keep the 2-slot API as a thin wrapper).
- Readout: measured m-digit base-4 value k → continued fraction on k/4^m →
  convergent denominators → candidate orders r → gcd(a^r−1, N). Same closing
  logic as `close_wide_phase_readout`, renamed `close_ququart_phase_readout`.
- Verdict object (Codex §XII): the run returns a `QuquartResidual`
  (comp = distance of braid action from target, leak = non-computational
  charge mass) plus the classical factor pair. Exit code is the Boolean
  projection of the residual's tier, never the other way round; the residual
  is archived alongside the membrane artifact so Crowley-class outcomes
  ((T,B)) are inspectable rather than flattened to success/failure.

### M4 `src/recycled_carrier.rs` — device face extension (edit, ~40 LoC diff)
- Add `pub enum QuquartCarrierGate` variant or feature-gate
  `"carrier":"ququart4"` in `WorkPreparation::lower_for_braid`; braid lowering
  emits M2 exchanges for Feedback gates whose numerator ≡ 0 mod 2 (quartic
  angles lower exactly; odd dyadics stay on the qubit path).

### M5 `make_ququart_membrane.sh` — mint script (copy of make_membrane.sh)
- Usage: `./make_ququart_membrane.sh <N> [a_base] [max_shots]`.
- Generates `examples/ququart_membrane_<N>.rs` calling
  `ququart_order_find::run_ququart_unbraid(n, a, max_shots)`; same exit-code
  contract (0 closed, 1 rupture, 2 unclosed-in-shots, 3 Frobenius failure);
  same syzygy + multiply_via_word verification block.
- Registers the new family word in `membrane_family::family()`:
  `("ququart_anyonic", "<glyph word TBD from ob3ects/digital>")` so
  `shor_b4 membrane run` sees it too.

### M6 validation harness `tests/ququart_membrane.rs` + `closure_check.py` sibling
- Rust integration tests: N = 15, 21, 33, 143, 10002200057 (existing corpus
  under `membranes/imasm_phase_factor_*`), assert closure in ≤ MAX_SHOTS where
  the qubit membrane closes, and assert Yang–Baxter / SIC residuals = 0.
- Python cross-check `ququart_closure_check.py` mirroring `closure_check.py`:
  independently recompute the Z₄ S/T matrices and the readout CF chain.

## 2. Build order and gates

Stage A: M1 alone, `cargo test ququart4` green → carrier substrate trusted.
Gate A′ (Codex conformance, rides along with Stage A): fix
`belnap_residual::tier()` so (T,N) = UnresolvedLeak is distinct from
(T,T) = Terminal per Codex §XI; add the exhaustive `special_closed()` test
over all 16 residuals (μ∘δ = id₄ by join-idempotence, Codex §IX); land the
M1 SIC certificate's FOUR classification through the same thresholding path
as `sic::certificate::EvidencePolicy::classify`. Existing tests that assert
the folded behavior get updated in the same commit — this is a semantics
change, flagged so it cannot sneak in silently.
Stage B: M2 against M1; Yang–Baxter test green → anyon sector trusted.
Stage C: M3 against `phase_unbraid` reference: for N ≤ 2^20, the base-4
register must produce the same winding k (in base-4) as the base-2 register
produces in base-2 for the same shot seed. Property test over seeded rng.
Stage D: M4 wiring; `run_cmds.sh "ququart list"` REPL face (mirror
`shor_b4_membrane::repl_shor_b4_membrane` style output incl. tuple line
`QUQUART_TUPLE = "<qc xp sh ph sw rb cf wd et ht Z>"`) and the protocol-word
line (`SEMPIPRIME_PROTOCOL` / `PRIME_PROTOCOL` glyph string, Codex §VI).
Stage E: M5 mint + M6 validation; archive artifacts into
`membranes/ququart_<N>_...` following the existing naming convention, each
artifact carrying its `QuquartResidual` verdict alongside the exit code.

## 3. Risks / open items (technical only)

- Radix-4 FFT blocking: `phase_unbraid` streams radix-2 butterflies in
  `mem_cap` blocks; radix-4 needs partner-tile logic for two stages at once
  (`butterfly_partner_tile` generalizes to a 3-partner set). Budget: this is
  the largest single edit.
- Odd-N guarantee: current membrane peels 2s first and winds odd N; base-4
  comb preparation requires the same coprimality precondition on a. Keep the
  shell guard (`N % 2 == 0 → exit 11`).
- Word encoding: `native_numeral` is parity/Z₂-graded. A ququart numeral needs
  the 2-bit grade (mod-4 residue) as its topological boundary invariant. Plan:
  extend `encode` with a `encode_mod4` sister that chains
  `∈_4 (base-4 branch) × m ∋` cells using new tokens, keeping decode exact.
- Fusion-space dimension growth: n ququarts give 4^n; stencil evaluation
  (M2) avoids materializing it, same trick as `anyon_fusion_kernel::stencil`.
