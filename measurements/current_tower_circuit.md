# Current fixed-point nested IMASM factor tower

Every live aggregate, phase-leaf, branch, fusion, fixation, and emission token is a `NestedMark { mark, depth }`. Before the mark acts, it traverses the same contained IMASM vessel:

```text
∈ FSPLIT
├── ⊤ EVALT ──┐
└── ⊥ EVALF ──┴── ∋ FFUSE ── ⊡ IFIX ── ⊙ IMSCRIB
```

The two arms preserve the same banked inner morphism. Therefore the vessel is its exact fixed point:

```text
B(A) = A
μ ∘ δ = id
emitted mark = input mark
```

The emitted mark then performs its scheduler, square-frontier, selection, or continuation action. Recursive zoom changes no semantic state and requires one fixed-point composition at every depth.

The rendered [SVG](current_tower_circuit.svg) and [PNG](current_tower_circuit.png) show the full containment circuit. The [Graphviz source](current_tower_circuit.dot) regenerates both artifacts.
