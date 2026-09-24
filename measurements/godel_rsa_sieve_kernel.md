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

| Challenge | Input bits | Sieve aperture | Odd primes tested | Factor-bound read | Pair closure | Semiprime closure |
|---|---:|---:|---:|---|---|---|
| RSA-100 | 330 | 2,097,152 | 155,610 | no odd prime divisor ≤2,097,152 | open, unresolved | `N` |
| RSA-110 | 364 | 2,097,152 | 155,610 | no odd prime divisor ≤2,097,152 | open, unresolved | `N` |
| RSA-120 | 397 | 2,097,152 | 155,610 | no odd prime divisor ≤2,097,152 | open, unresolved | `N` |

The encoded semiprime control `10007000070049` closes at width 16:

| Input bits | Sieve aperture | Derived factor pair | Product closure | Semiprime closure |
|---:|---:|---|---|---|
| 44 | 65,536 | `10007 × 1000000007` | closed | `T` |
| 50 | 1,048,576 | `1000003 × 1000000007` | closed | `T` |

The CLI was given each decimal modulus. It constructs the cell-binary word
internally; neither factor is supplied. The smaller factors are witnessed by
the sieve; exact arbitrary-precision division derives the ten-digit cofactors,
and their prime reads close the pairs.

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

The analyzer now derives a cofactor from each divisor witness using exact
arbitrary-precision division, verifies `factor × cofactor = N`, and reports
semiprime closure separately. `21` closes as `3 × 7` (`T`); `999999` closes as
the product pair `3 × 333333`, but its cofactor is composite (`F`). The RSA
inputs remain explicitly open because this sieve found no factor witness.

The arbitrary-precision path was also exercised with `godel analyze 21 128`:
it reports aperture `2^128=340282366920938463463374607431768211456` and finds
divisor 3 after one prime test. This confirms that the aperture is not restricted
to a machine-word value; it does not claim that exhausting such an aperture is
computationally practical.

Verification: `RUSTFLAGS='-D warnings' cargo test --bin godel`,
`RUSTFLAGS='-D warnings' cargo test -p vox --lib godel_analyzer::tests`, and
`RUSTFLAGS='-D warnings' cargo test -p vox --lib` all pass. The library suite
contains 79 tests. The CLI invocations `godel analyze 21 16`,
`godel analyze 21 128`, and each RSA input with aperture width 21 produced the
readouts above. Decimal inputs `10007000070049` and `1000003007000021` were
submitted directly at widths 16 and 20 and produced the closed pairs above.

## Cross-frame arithmetic

`godel frame-op <N> <left-width> <left-group> <operation> <right-width> <right-group>`
selects two little-endian group values from distinct frame widths of decimal
input `N`, then applies `add`, `mul`, `sub`, `mod`, or `divmod`. Group addresses
are zero-based. The report gives the source numeral, each selected value, and
the result as cell-binary IMASM words.

For `N=45`, width-2 group 2 has value 2 and width-3 group 0 has value 5.
`godel frame-op 45 2 2 mul 3 0` returns 10. The paired `divmod` read with
operands 5 and 2 returns quotient 2 and remainder 1. This verifies arithmetic
between values selected from different frame decompositions; it is a frame
operation, not a factor-pair witness.

The Godel commands `braid <p> <q>` and `unbraid <N>` apply Γ and Λ directly to
cell-binary numeral streams. Γ interleaves the two LSB-first streams with
zero-padding of the shorter one. Λ separates even and odd cells. Every report
includes the cell-binary words and checks `Λ(Γ(p,q))=(p,q)` or `Γ(Λ(N))=N`.
The unbraided lanes are only representation descendants: their product is
checked independently, and the RSA inputs still require a factor-producing
closure path.
