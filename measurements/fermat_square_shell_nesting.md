# Fermat square-shell nesting

The live admitted-destination continuation now consumes the carried excess as

```text
E = q(2B + q) + t'
```

and returns both the newly nested shell count `q` and the unnested remainder `t'`. The continuation derives `b'`, `c'`, and `g'` from those returned tapes, and exact closure is `t' == 0`.

The binary-bracket and reciprocal experiments remain preserved in commits `8bbff11` and `59eda42`; neither is on the release execution path.

One hundred fresh process runs after replacing general multiplication by two with tape doubling gave:

| product width | median | minimum | mean | shell folds |
|---:|---:|---:|---:|---:|
| 511 bits | 2.8884 ms | 2.6910 ms | 2.9667 ms | 5 |
| 1023 bits | 5.8346 ms | 5.6177 ms | 5.9192 ms | 25 |

All ten temporal phase-relation controls pass in release mode, including independent full-root agreement and the exact shell decomposition and frontier bound.

## Recursive block containment

The live frontier now closes `E` by descending through structural blocks. A
depth-`r` block contains `2^r` odd shells and has tape capacity:

```text
C(B, r) = 2^r(2B + 2^r) = (B << (r + 1)) + 2^(2r)
```

The release path tests this capacity against the remaining excess, accepts
only fully contained blocks, and updates the carried base and remainder. It
uses shifts, one-hot tape marks, addition, comparison, and subtraction. The
older correction routine remains a debug oracle for the exact contained span.

One hundred fresh process runs measured 2.9757 ms median at 511 bits and
5.4991 ms median at 1023 bits.
