# Prime-Factor EXTRACTION Methodology — the godel instrument

Instrument: `/home/mrnob0dy666/imsgct/G-mOMonadOS/target/release/godel`
Bridge:    `/home/mrnob0dy666/imsgct/G-mOMonadOS/run_cmds.sh`

Extraction = separate a witness from a carrier, then verify it by product closure.
A factor is not *found* when it is *printed*; it is extracted when it leaves the
word and the re-braided product returns the source exactly.

The number IS the word. `encode(n) = ⊢ (≻⋈∈b∋)* ⊙⊡⊣`, bits LSB-first,
⊥=1 (odd), ⊤=0 (even), leftmost cell = 2^0. Every extraction below runs on
the word's own bits; the decimal is only the boundary where a human reads off.

## Stages

### S0 — Commit N to the word
`godel encode <N>`  →  cell-binary word, five codec assertions PASS.
All extraction operates on this word, never on a decimal kept on the side.

### S1 — 2-adic peel (exact, unbounded)
`godel analyze <N>` reports `primitive.v2`, `odd-part`, `composite.decomp-k`,
`decomp-prefix`, `decomp-remainder`, `residue-2^k`.
The 2-part 2^v2(N) is read straight off the word's low bits — exact, no aperture.
The residual carrier is the odd-part; odd-prime extraction (S2) runs on it.

### S2 — Bounded odd-prime sieve (the extraction proper)
`godel analyze <N> [sieve-window]` (default window 8 → aperture 2^8=256)
runs prime_sieve_read over odd primes in ascending order.

  witness found:
      negative.factor-witness   p          (p = least prime factor)
      kernel.factor-pair        p × q      (q = N/p)
      kernel.product-closure    closed
      kernel.register.semiprime-closure  T  iff p and q both prime
  no witness:
      negative.factor-bound     >A         (least prime factor exceeds A:
                                            a certified LOWER BOUND)
      kernel.product-closure    open
      kernel.register.semiprime-closure  N

Semiprime-closure T ⇒ extraction complete in one sieve pass (both lanes prime).
Co-factor composite ⇒ RECURSE: `godel analyze <q>` to peel the next least
factor. This is factor-peeling on the word, one witness per pass.

Verified live:
  91      → 7 × 13,        semiprime-closure T
  1000001 → 101 × 9901,    semiprime-closure T
  97      → 97 × 1,        product-closure open, semiprime-closure F (prime)
  999999937 → factor-bound >256, witness none, closure N (sieve exhausted)

### S3 — Product-closure verification (the gate)
Any candidate pair must pass the word-level equation:
`godel check mul <Wp> <Wq> <WN>`  →  PASS / FAIL.
Extraction is committed only when the re-braided product returns N exactly.
  7 * 13 = 91 → PASS      7 * 13 = 92 → FAIL
This is the same gate the unbounded `separate` lane uses
(unbraid_semiprime → Miller-Rabin on both lanes → verify p*q==N → check mul
→ convolution normalizes).

### S4 — Unbounded engines (when the sieve is exhausted)
When `factor-bound >A` holds and N is still composite, hand the carrier to the
run_cmds.sh REPL factor engines (repl.rs `factor` family):
  gpu_rho factor, nested_oneshot factor, nested_prime_factorization factor,
  doubly_nested_oneshot factor, prime_winding factor, trilattice_factor factor,
  winding factor
each returns candidate lanes that MUST re-enter S3 (check mul) before commit.
The unbounded morphism lane (Vox `godel separate` / `unbraid_semiprime`) is the
same protocol at scale: enumerate width pairs (p_bits+q_bits ∈ {bits(N),
bits(N)+1}, balanced outward), Hensel-lift each, commit iff trim(mul(p,q))==N.

### S5 — Prime-power readouts (LTE)
`godel lte2 <odd-a> <even-m>`  →  v2(a^m−1) = v2(a−1)+v2(a+1)+v2(m)−1.
Exact 2-power of a^m−1, a single formula, no search.
  lte2 5 144 → 6   (64 | 5^144−1)
  lte2 7 100 → 5   (32 | 7^100−1)

### S6 — Structural splits (not factorization — use to navigate, never to commit)
`godel braid <l> <r>`   Γ interlace of two values (structural, ≠ product).
`godel unbraid <n>`     Λ deinterlace into even/odd bit lanes; reports
  ΓΛ-closure (μ∘δ=id) and factor-closure (p·q==N?).
  unbraid 91 → left-lane 13, right-lane 3, factor-closure OPEN (13×3≠91).
These are diagnostic: they expose the word's bit layout and confirm the
interlace round-trips, but the `factor-closure` field is the only one that
clears a pair as a genuine factor pair.

## The protocol, in one line
encode → peel 2^v2 → sieve least odd prime (witness or certified bound) →
recurse on the co-factor → verify every pair with `check mul` → when the
bounded sieve is exhausted, the REPL/GPU unbounded engines produce candidates
that re-enter the same `check mul` gate. A factor is committed only by the
product closure returning the source word exactly.

## Kernel registers (the readout discipline)
analyze lands every result in the SIXTEEN_3 register and reports the joint:
  codec T, sieve t|f|N, pattern t|f, semiprime-closure T|F|N, joint Ttf|…
B is information, not a command to collapse: an `N` closure (sieve exhausted)
is a legitimate extraction state — it is a certified lower bound, not a failure.

## Live verification log (all readings above taken from the running instrument)
  godel analyze 91        → factor-pair 7 × 13,      product-closure closed, semiprime-closure T
  godel analyze 97        → 97 × 1,                  product-closure open,   semiprime-closure F
  godel analyze 1000001   → factor-pair 101 × 9901,  product-closure closed, semiprime-closure T
  godel analyze 274507    → factor-bound >256, witness none, closure N  (sieve exhausted — S4 hand-off)
  godel analyze 999999937 → factor-bound >256, witness none, closure N
  godel lte2 5 144        → v2(5^144−1) = 6
  godel lte2 7 100        → v2(7^100−1) = 5
  godel check mul W7 W13 W91  → PASS     |  godel check mul W7 W13 W92 → FAIL
  godel unbraid 91        → left-lane 13, right-lane 3, ΓΛ-closure closed, factor-closure open
  godel braid 7 13        → braid-value 183, unbraid recovers 7 | 13, ΓΛ-closure closed
  run_cmds.sh: prime_winding factor 274507 → 274507 = 277 × 991   (S4: unbounded engine
    extracts the factor the 2^8=256 sieve could not see; 277×991 verified = 274507)
