# Anyonic Gödel Ququart Factor Extractor Plan

## Objective

Produce a prepared, compiled G-mOMonadOS membrane that executes an arbitrary baked RSA-style semiprime above 200 bits and emits the extracted factors only after a Gödel multiplication closure verifies `p × q = N`.

## Current Finding

The terminal report emits the factor arms at the common fixed point of Vox’s product-over-prefix and prefix-over-product nestings. The nested radix is a baked canonical IMASM numeral word and must be a power of two greater than one. The source-bound ququart winding producer supplies the candidate arms; both nesting directions consume those arms before the terminal Gödel multiplication check. The saved-output verifier preserves those words and invokes the native closure verifier without decimal conversion. Gödel multiplication, division, and modular powering read adjacent binary cells as radix-four digits. The certified winding path uses these native numeral operations for modular certification, half-winding reduction, factor-arm division, and the final product check.

The standalone SIC representation has the intended sixteen outcomes. `FixedQuquartSic::rays[i]` uses `p = i / 4`, `q = i % 4`, and `SixteenOutcome::new(i)` preserves mask `i`.

The factor path does not use that representation. `QuquartPhaseDevice::measure_phase_digit` returns one of four `QuquartDigit` values. `QuquartFoldedWorkDevice::measure_phase_digit` samples the four computational channels plus the outside-carrier channel. Its measurement therefore bypasses the sixteen-outcome SIC POVM. The sixteen-state map is locally present and correctly indexed, but it is not integrated into factor extraction.

The factor path also assumes each inverse-Fourier measurement is an exact radix-four phase digit. A sixteen-outcome SIC result cannot be collapsed to one of four digits by taking a mask bit or a Weyl-Heisenberg index. That decode must follow from the SIC measurement probabilities and instrument.

## Current Implementation Delta

`QuquartFoldedWorkDevice::measure_sic_outcome` now evaluates the overlap against all sixteen fixed-point SIC rays, samples the corresponding `SixteenOutcome` mask, keeps the outside-carrier channel distinct, conditions the shared work state on the selected outcome, and prepares the next control in its initial computational channel. The focused 128-bit instrument test passes and verifies normalization of the retained work state. The factor executor still calls the four-outcome digit interface, so the SIC instrument is not yet the active phase readout.

`FixedQuquartSic::phase_outcome_masses` now calculates all sixteen SIC Born likelihoods for a rational eigenphase after controlled powering and rational feedback. Tests check each of the four ideal phase poles against the corresponding canonical SIC-ray probabilities at every source width listed in `anyon-extractor-width-controls.tsv`. This validates those four controls at those fixtures; it does not yet implement a posterior phase estimator, SIC-driven factor execution, or the complete 16-state map in the factor path.

`SicPhaseEvidence` now retains canonical modulo-one rational phase hypotheses and multiplies their exact fixed-point SIC likelihood weights for each observed `SixteenOutcome`; equivalent fractions are rejected as duplicates. Outside-carrier events are rejected distinctly. A focused test checks that scores equal the corresponding likelihood for all four ideal poles, and the existing source-width control test performs this calibration at every fixture width. The accumulator has no fixed hypothesis count or weight ceiling. It is not wired into `QuquartFactorExecutor`: period-hypothesis likelihoods, candidate generation, replacement of digit feedback, and end-to-end order closure remain outstanding.

`QuquartFactorExecutor::sic_shot` now calls the device SIC instrument at every controlled-power stage, records the complete mask sequence, updates caller-supplied rational phase evidence with each mask likelihood, and retains outside-carrier termination distinctly. The folded-work device exposes its SIC measurement through the shared device trait, including per-shot measurement accounting. A schedule test confirms the full mask sequence reaches the evidence accumulator without invoking the four-digit measurement method. This path has not yet replaced the baked executable's digit-based `shot` path, and candidate generation/order inference plus terminal extraction remain outstanding.

The SIC shot now validates that its evidence calculator uses the source-derived fixed precision and stages evidence updates transactionally. An outside-carrier result therefore leaves no partial shot evidence committed. Tests exercise both source-width rejection and rollback on a staged outside-carrier event.

`close_sic_phase_evidence` now ranks observed rational hypotheses by likelihood, treats their reduced denominators as order candidates, and accepts a candidate only after `base^r mod N = 1` and the existing half-winding plus native Gödel product closure succeed. A test closes the existing RSA100 certified winding through this SIC evidence seam, and verifies that evidence with no observations cannot close. Candidate generation is still not implemented, so this does not supply a complete estimator or justify runtime execution yet.

The certified RSA100 winding control checks the native nested meeting point at power-of-two radices with digit widths both below and above the source extent. Unit and composite radices, decimal radix text, and a unit factor arm are rejected. This control supplies its winding as a test fixture; source-dependent winding production runs in the baked membrane.

Prepared artifacts must continue to encode every baked source/base/operator value as native IMASM words. The phase hypotheses and candidate orders are runtime-derived from SIC evidence; they must never be inserted into the prepared artifact.

Prepared membranes retain a readout verifier whose digest is bound in the manifest. The preparation digest and the exact prepared bytes embedded in the executable bind the replay to its baked words. Saved-output replay uses that retained verifier and validates the baked numeric leaves before checking phase evidence and Gödel multiplication.

The terminal factor report now emits numeric values only as canonical IMASM words, including source, base, shot count, phase numerator/denominator, order, and both factors. The verifier rejects unapproved decimal-valued fields and checks the echoed input words against the prepared words. Preparation validates every numeric JSON leaf by decoding it as a canonical cell-binary word before compilation. The nested terminal closure calls Vox’s existing factor_2adic library through the kernel. The executable runs directly.

The source-dependent modular producer executes whole reversible additions and comparisons as shared dyadic split/fuse transducers. A subtraction-borrow coordinate selects each addition input; a low-to-high comparison state controls the high flag. Changed and unchanged arms fuse on the original control literals. The same modular-add shell, controlled swap and inverse rail supply both these nested boundaries and the elementary gate lowering. Arena reclamation occurs after each complete nested operation, with no node, support or retry ceiling.

The coordinate control checks every complex amplitude on a complete finite basis with mixed positive and negative controls and nonadjacent register wires. The modular control checks independent complex residue arms on all computational channels at the smaller control precision and at RSA100 precision. Exact amplitudes at clean-work output addresses account for the complete retained mass. The elementary multiplier and existing four-channel gate correlation regressions pass.

The prepared nested-work family is `membranes/ququart_nested_work_radix_rsa100_20261004`. Its digit radix selects the terminal product/prefix nesting; its phase acquisition remains the source-bound ququart circuit. The octal family member runs directly with a live handle recorded in `active_execution.json`.

## Implementation Sequence

1. **Fix the readout contract.** Derive the sixteen SIC effects and the conditional work-register state for each outcome after the inverse ququart Fourier and feedback operations. Specify the phase evidence carried by every `SixteenOutcome`, including the outside-carrier result. Derive the outcome likelihoods for each candidate phase and feedback state. Do not use a convenient mask-to-digit projection.

2. **Verify the SIC readout mathematics.** Add instrument-level checks for all sixteen masks: normalization of the seventeen outcome masses, correspondence between mask order and the canonical WH(4) rays, state reconstruction on a complete ququart SIC frame, and agreement of phase-outcome likelihoods with direct four-state controls. Run these checks at the supported source precisions. The readout contract must close before any factor run is treated as evidence.

3. **Wire the factor device to the full instrument.** Change the device API to return a `SixteenOutcome` or outside-carrier outcome with the conditional shared-work state. Use the existing fixed-point SIC rays in the coherent work backend to compute each outcome branch. Give each phase stage a fresh prepared ququart and release its measured carrier after the SIC outcome, unless an outcome-conditioned reset braid is derived and verified. Preserve the full measured mask sequence in resident state.

4. **Replace digit accumulation with SIC evidence accumulation.** Update the phase readout accumulator to consume the full SIC outcome likelihoods. It must retain competing phase/order candidates until evidence distinguishes them. Accept an order only when modular exponentiation certifies `base^r mod N = 1`; derive nontrivial factors from the certified half-winding and reject trivial splits. There is no fixed shot ceiling.

5. **Close factors in the terminal Gödel path.** Hold `N`, `p`, and `q` as canonical cell-binary words. Require `godel_calculus::check(p_word, Mul, q_word, source_word).valid` before constructing the terminal result. The completed result contains the factor words, the source word, and the explicit successful product-verification verdict. No factor, order, or phase readout is embedded in the prepared program.

6. **Prepare one qualifying membrane.** Bake a single balanced, unstructured RSA-style source above 200 bits and its coprime base into native G-mOMonadOS IMASM words. Prepare the Fibonacci Fourier operator and controlled-power schedule in G-mOMonadOS. Compile that prepared JSON into the standalone factor executable. Reuse the existing Vox nested product/prefix library for terminal closure. A retained Fourier preparation can be reused only when its executable checksum matches and its exact prepared bytes occur in that executable.

7. **Execute and verify the artifact.** Run the baked executable directly with terminal-only output. The success evidence is its factor-bearing terminal closure, plus an independent invocation of the native Gödel verifier on the emitted `p_word`, `q_word`, and `source_word`. Confirm `cargo build --release` for the complete project after the integration.

## Completion Gates

- The factor device produces all sixteen SIC outcomes with canonical `SIXTEEN_3` mask identity and retains the outside-carrier outcome separately.
- The phase estimator consumes SIC outcomes through the derived likelihood model, with no four-outcome substitute.
- The prepared executable has no runtime source input, no baked factor or order, and no pre-closure telemetry.
- A qualifying execution emits `p`, `q`, and a successful Gödel check of `p × q = N`.
- The release build succeeds and the direct execution result verifies against the native Gödel checker.
