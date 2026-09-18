# Current factor-relation nesting audit

Circuit source:
`fully_nested_phase_based_prime_factorization_ob3ect.json`, canonical cut
`⊢∈≻⊤≺⊥⋈⊙⊞∋⊡⋈⊙⊣`.

The generated three-vessel implementation diagram contains 40 operators,
three split/fuse pairs, no open forks, and a closed terminal walk. The SVG and
ASCII renderings were produced with the same `IMSCRIBr` wiring and symbolic
diagram functions used by `ob3ect/auto.py`.

| Implementation boundary | Enclosing morphism | Status |
|---|---|---|
| Production phase state | live arbitrary-width `N, a, a²-N, b, p, q` state carried recursively through complete factor vessels | nested |
| Phase family | one root morphism retains the continuation across every candidate leaf until terminal emission | nested |
| Candidate advance | `AFWD` mutates the live phase state at the leaf of every requested nesting level | nested |
| Difference and square relation | `EVALT → AREV → EVALF` acts on the live state inside the recursive vessel | nested |
| Factor construction and validation | `CLINK → IMSCRIB` acts on the live state inside the recursive vessel | nested |
| Production terminal factor | fixed at `IFIX`, emitted only after `CLINK → IMSCRIB → TANCH` | terminal surface |
| BDD diagnostic circuit operator | operator token through `imasm_relation_tower` | nested |
| Construction epoch | full factorization vessel carrying the circuit cursor | nested |
| Node compaction | full factorization vessel carrying `IFIX` | nested |
| Unique-table rehash | full factorization vessel carrying `CLINK` | nested |
| Storage continuation | full factorization vessel carrying `AFWD` | nested |
| Decision-node choice | selector split/evaluate/fuse/fix word through the tower | nested |
| Witness limb | full factorization vessel carrying the incoming decision edge | nested |
| BDD diagnostic factor emission | final limb continuation followed by one buffered host write | terminal surface |

The hosted executable enters through the same enclosing IMASM phase word.
Its nested VINIT leaf admits raw command fields, its nested AFWD leaf invokes
the relation, and its TANCH leaf performs the sole terminal write. The terminal
record names both the executable vessel and the phase vessel it encloses. Circuit
compilation, CUDA module loading, and initial allocation remain preparations
inside the relation before phase execution. BigUint multiplication independently
checks the terminal emitted limbs after the vessel closes.

The production implementation has no mark-return boundary. A vessel does not
return an operator ordinal to a Rust arithmetic switch. It carries the live
relation state inward and applies that operator only at the nesting leaf.
The production entry invokes one executable vessel containing one phase-family
vessel. Candidate phases are internal leaves of that inner vessel and do not
return to the command boundary.

The process vessel is the parent of every REPL command vessel. Serial output,
guest-process stdout and stderr, and hosted diagnostics collect at the innermost
active terminal surface. An inner TANCH contributes that surface to its parent;
the outer process TANCH is the host write.
