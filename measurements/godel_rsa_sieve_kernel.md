# Gödel RSA frame and sieve readout

The Godel analyzer carries an odd-prime sieve result into the same SIXTEEN_3
kernel state as the codec and frame-support reads. `godel analyze N W` tests odd
primes through aperture `2^W`; a found divisor is positive truth support, while
an exhausted aperture for an odd composite is a certified lower bound on its
prime factors. A frame pattern is retained as its own support and is not treated
as a divisor witness.

There is no fixed aperture ceiling. The aperture and prime counter are dynamic
`Nat` values. When the aperture fits the machine's addressable index range, a
segmented sieve uses a bounded-size working segment instead of allocating the
whole aperture. For larger apertures the exact arbitrary-precision candidate
path is used. Runtime still grows with the number of primes in the requested
aperture; dynamic representation removes an arbitrary cutoff, not the work
required to certify every prime below the bound.

The baked input is only the decimal encoding of each challenge modulus. No
factor is supplied to the analyzer.

| Challenge | Input bits | Sieve aperture | Odd primes tested | Factor-bound read | Joint kernel |
|---|---:|---:|---:|---|---|
| RSA-100 | 330 | 2,097,152 | 155,610 | no odd prime divisor ≤2,097,152; lower bound >2,097,152 | `Tf` |
| RSA-110 | 364 | 2,097,152 | 155,610 | no odd prime divisor ≤2,097,152; lower bound >2,097,152 | `Tf` |
| RSA-120 | 397 | 2,097,152 | 155,610 | no odd prime divisor ≤2,097,152; lower bound >2,097,152 | `Tf` |

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

The arbitrary-precision path was also exercised with `godel analyze 21 128`:
it reports aperture `2^128=340282366920938463463374607431768211456` and finds
divisor 3 after one prime test. This confirms that the aperture is not restricted
to a machine-word value; it does not claim that exhausting such an aperture is
computationally practical.

Verification: `RUSTFLAGS='-D warnings' cargo test --bin godel`,
`RUSTFLAGS='-D warnings' cargo test -p vox --lib godel_analyzer::tests`, and
`RUSTFLAGS='-D warnings' cargo test -p vox --lib` all pass. The library suite
contains 75 tests. The CLI invocations `godel analyze 21 16`,
`godel analyze 21 128`, and each RSA input with aperture width 21 produced the
readouts above.
