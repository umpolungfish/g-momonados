# Fixed-point containment of the live factor tower

The previous `depth` path discarded its argument and invoked the Rust leaf directly. The live aggregate, phase-leaf, branch, fusion, fixation, and emission marks now enter through `NestedMark { mark, depth }`.

The enclosing action is the complete fixed-point vessel:

```text
∈ FSPLIT
├─ ⊤ EVALT
├─ ⊥ EVALF
∋ FFUSE
⊡ IFIX
⊙ IMSCRIB
```

Both arms carry the same banked inner morphism. Consequently the vessel acts as

```text
μ ∘ δ = id
B(A) = A
```

and arbitrary recursive zoom closes in one composition. No loop proportional to nesting depth is executed.

The release control checks all twelve marks at depths 0, 1, 2, 64, and `u32::MAX`. Every vessel emits the same morphism and records one fixed-point composition.

One hundred fresh process runs produced:

| product width | median | minimum | mean |
|---:|---:|---:|---:|
| 511 bits | 2.9279 ms | 2.6046 ms | 3.0331 ms |
| 1023 bits | 5.5137 ms | 5.1440 ms | 5.6559 ms |

All eleven temporal phase-relation controls pass in release mode.
