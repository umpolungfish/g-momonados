# Gödel RSA frame and sieve readout

The Godel analyzer now carries a bounded odd-prime sieve result into the same
SIXTEEN_3 kernel state as the codec and frame-support reads. `godel analyze N W`
tests odd primes through the aperture `2^W`; a found divisor is positive truth
support, while an exhausted aperture for an odd composite is a certified lower
bound on its prime factors. A frame pattern is retained as its own support and
is not treated as a divisor witness.

The baked input is only the decimal encoding of each challenge modulus. No
factor is supplied to the analyzer.

| Challenge | Input bits | Sieve aperture | Odd primes tested | Factor-bound read | Joint kernel |
|---|---:|---:|---:|---|---|
| RSA-100 | 330 | 65,536 | 6,541 | no odd prime divisor ≤65,536; lower bound >65,536 | `Tf` |
| RSA-110 | 364 | 65,536 | 6,541 | no odd prime divisor ≤65,536; lower bound >65,536 | `Tf` |
| RSA-120 | 397 | 65,536 | 6,541 | no odd prime divisor ≤65,536; lower bound >65,536 | `Tf` |

For all three inputs, the codec checks pass (`T`), the frame support and sieve
lower-bound reads contribute falsity support (`f`), and the joint register is
`Tf`: truth and falsity support coexist without collapsing either lane. Every frame at widths
2–8 preserves the entire LSB-first stream when its joint symbols are
concatenated. Adjacent frame states satisfy both kernel order reads
(`prev≤i=true`, `prev≤c=true`). These are structural and bounded-sieve
certificates, not factor recovery.

The positive control `21` at width 16 returns divisor 3 after one odd-prime
test and records a `t` sieve state. This distinguishes a witnessed divisor
from the RSA lower-bound result, whose sieve state is `f`.

Verification: `RUSTFLAGS='-D warnings' cargo test --bin godel`,
`RUSTFLAGS='-D warnings' cargo test -p vox --lib godel_analyzer::tests`, and
`RUSTFLAGS='-D warnings' cargo test -p vox --lib` all pass. The library suite
contains 74 tests. The four CLI invocations `godel analyze 21 16` and each RSA
input with aperture width 16 produced the readouts above.
