# Source-bound stack checks

Current factor inputs are the independently generated RSA-style semiprimes in
`Vox/measurements/anyon_wiring/rsa_cases.json`: 200 and 256 bits. Known primes
remain in the external verifier. No close-prime fixture replaces these inputs.

The required closure concerns transformed objects throughout the stack,
including interfaces between levels. A source-register equality, a bracket
pairing, or a generic involution is insufficient to establish that condition.

The braid presentation now uses generator indices relative to its carrier
frame. Both the canonical encoder and reader honor that origin. The checked
frame interface rejects invalid indices and depth overflow, retains parent
frames, requires work inside every split/fuse pair, and recovers each emitted
generator before returning to the incoming height.

`rsa_physical_frame_audit.log` records checks of the actual Fourier compiler's
2,095,197-generator words. The audited heights derive from the elementary
modular register layout plus the ququart's additional control lane: 605 for
the 200-bit source, 773 for the 256-bit source. This certifies presentation
recovery at those heights. Computational action, leakage, resident amplitude
maps, and interfaces between representations are separate obligations.

The Fourier compiler checks computational residual, leakage and unitarity
before its frame-recovery gate releases a word. The measured computational
residual for these words is approximately 1.22972011e-4. These local checks do
not establish completed factor extraction.

The morphism return path now continues through the enclosing fixation
operators after selection. Further search stops while the returned source is
checked on the way back through the carrier. The corresponding source-bound
regression isolates return closure without treating its injected payload as
a factor measurement.

Prepared work now retains its source and radix. Both word decoding and resident
entry check every reconstructed operation against the source/base/radix-bound
emitter, in order, throughout the complete controlled-power schedule. The
200-bit check covers 204 stages and 285,600 references; the 256-bit check covers
260 stages and 465,920 references. Changing the deepest operation order or
removing the deepest stage is detected before execution.

The resident entry uses this same verifier, including the executable operation
map. Matching source metadata without an operation stream cannot enter the
resident state. The coupled modular-power checks preserve the distinct complex
coefficients in all four tested control channels, retain their correlations
with the work register, and return clean arithmetic workspace in both
contiguous and interleaved layouts, using one-bit and four-bit work digits, on
the qualifying sources.

The contracted Fourier operator is now checked again at resident entry from
its exact serialized coordinates. Computational action, leakage, and both
adjoint return compositions are recomputed instead of trusting saved action
measurements. The physical-word inverse diagnostic remains a separate check.
`rsa_operator_entry_tests.log` records full contraction of both actual
2,095,197-generator braids, exact recovery of all 25 complex coordinates, and
rejection of changed action, leakage, and return maps even with zeroed saved
diagnostics. The accompanying manifest binds the evidence to the physical
words, serialized operators, probe, support code, and linked libraries.

The standalone resident driver accepts these source-bound contracted operators
as an explicit physical mode. Its original ideal Fourier construction is now
labeled as reference mode. Physical and reference observation records remain
separate.

Workspace checks now cover every allocated non-source wire in all five fusion
channels, including the borrow flag, carry workspace, and top control ancilla.
The qualifying regression injects dirty workspace at every allocated ancilla
height in each channel, in both layouts, and requires detection. Actual modular-power execution must also
pass this expanded gate.

`rsa_physical_ququart.json` and `rsa_physical_ququart_radix16.json` record
resident execution with the contracted physical operators. Both retain the
full 204/260-digit schedules and the qualifying sources. In the 30-second
observations both work radices completed three phase digits; four-bit work did
not move past that boundary. The observations do not verify a factor pair.
The next debugging target is growth of the shared decision branches inside
modular work, rather than reducing source size or phase resolution.

Vox native address sampling of the baked physical 200-bit run, with its
complete 204-digit schedule and the native producer disabled, identifies
`DecisionArena::reclaim_pending` in 37.54% of captured samples and `intern`
in 20.13%. The bounded observation reached four measured phase digits.
These are address-sample proportions, not hardware cycle measurements.
The profiled executable and input are bound by `rsa_native_profile_manifest.json`;
`summarize_native_profile.py` resolves complete sample rows against that ELF.

Support masks now represent value zero without an allocation or atomic
references. Nonempty masks retain immutable shared storage and copy on mutation.
Vacant slots use this empty value without changing direct live-node metadata
access. The qualifying checks also preserve distinct complex amplitudes across
positive and negative controls and a correlated spectator coordinate, including
invalid work residues. Applying the inverse modular translation returns the
entire correlated object exactly in both work layouts. Reused slots retain all
upper workspace constraints. Eight qualifying invariant checks pass with this
representation; this does not establish a completed phase shot or factor pair.

`rsa_support_mask_execution.json` records the physical runs after the final
mask change, with the same sources, base, entropy seed, operators, work radix,
and full schedules. Both observations reach three measured phase digits.
A speed improvement is not established by these observations. The earlier
optional whole-slot storage prototype is retained as historical measurement
in `rsa_reclamation_after_option_storage.json`; the current source instead
encodes only empty mask values without allocation.

The end-to-end stack remains under debugging. Completion requires a verified
factor pair for qualifying RSA-style inputs and closure of the actual maps
throughout execution, including their composites across stack levels.
