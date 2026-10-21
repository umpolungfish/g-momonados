# Native protocol execution during numerical preparation

`conventional prepare N [srfp|dnn|both]` now loads the saved native graph for
the selected construction. It validates the word, ordered wiring and native
closure, then executes the nodes with `imasm_core::flow::gate_out`. Incoming
and outgoing carrier values must match the native graph evaluator. Numerical
payloads follow those same edges inside the retained decision arena.

AFWD applies the square-minus-self operator through the existing resident Y.
AREV applies the native carrier involution and the numerical complement
`E -> N+1-E`, which preserves `E²-E mod N`. EVALT projects residual zero;
EVALF complements the point register and projects residual zero. FSPLIT and
FFUSE retain and reunite root families through the existing powerset carrier.
IFIX checks projection stability before TANCH returns. Source words, all live
protocol banks and the Y witness remain attached. These explicit numerical
lifts do not establish every mathematical predicate in `srfp.txt` or `dualnn.txt`.
CLINK currently checks source binding; it does not implement the full stated
Heisenberg commutation relation. `all_predicates_hold` therefore remains false.

[Executed preparation diagram](conventional_preparation_process_20261020.svg)

The prepared artifact is now `conventional-prepared-point/v2`. Its shared
protocol register serializes every retained bank in one node pool with an
explicit root order. Read validates the executed protocol selection and
completed traces before retrieving the evaluated factor leaves. Loading,
read-off and product verification keep separate clocks. Python Structure
preparation passes its actual `srfp` or `dnn` selection. `both` composes SRFP
and DNN on the same retained state; it does not launch independent producers.

The execution control uses the existing fresh random balanced 200-bit source
and derives `E=N-1` internally. It executes all twelve nodes of each graph,
checks the forward residual against exact arithmetic, verifies complement
involution, rejects the nonpoint at projection, recovers every retained bank,
and rejects a corrupted bank address. This is a negative numerical-path
control, not a successful factor production receipt. Reproduction and source
bindings are in
`G-mOMonadOS/measurements/conventional_translation_20261020/`.

AFWD still materializes the growing square-minus-self relation. Native graph
dispatch does not by itself flatten that arithmetic. The earlier 896-bit
execution stopped at row 2, column 209 after 492.305 seconds with 339,074 nodes;
it did not reach projection or emit a divisor. Full preparation and factor
return on that source remain open.

The pre-change diagram, source and renderer remain preserved as
`before_protocol_dispatch*` in the measurement directory. The earlier 200-bit
CPU profile remains in `flatten_profile/`. `render_process.py` reproduces the
current source and native graph diagram, with hashes in
`process_diagram_binding.json`.
