# Nested IMASM graph flow on CUDA

Run from the native REPL:

```
gpu_kernel flow 3 4 ⊢∈≻∋⊣
```

The operands are fork arity (2 or 3), additional enclosure depth (0 through
8), and a glyph word. Each enclosure places another split and fuse inside
the word's source and anchor. The command uses the shared IMASM protocol
wiring and grammar validation, then executes carrier propagation on CUDA.
Graphs currently fit within 64 nodes and 192 edges.

All sixteen SIXTEEN_3 seeds run on device. The report includes the structural
check verdict, each ancestry-paired dyad's work status, recovery at each dyad,
and agreement with the shared core for every node and edge. Gate tables come
from `imasm_core::flow::gate_out`; propagation executes on device. Grammar
validation and ancestry pairing execute on the host.

Controls:

```
gpu_kernel flow 2 0 ⊢∈≻∋⊣
gpu_kernel flow 3 0 ⊢∈≻∋⊣
gpu_kernel flow 3 4 ⊢∈≻∋⊣
gpu_kernel flow 3 0 ⊢∈⊙∋⊣
gpu_kernel flow 3 0 ⊢∈≺∋⊣
```

The first three exercise transforming closure and recovery. The fourth is
identity closure. The fifth has transforming work but changes some carrier
seeds, making structural closure and flow recovery independently observable.

This command executes nested protocol graphs. The existing `gpu_kernel bench`
nested interpreter is a separate execution path. No factor relation or Lean
proof is supplied by these controls. The earlier `phase_prefix.py` timings
measure the host search implementation and carry no IMASM verdict.

Validation: the three graph tests pass, including every enclosure depth and
all sixteen seeds at both arities. The shared core's fourteen library tests
pass, and `ask` passes `cargo check --offline`. The broader native library
suite has 31 passes and two failures in `btc_key_deriver`: its public-key word
length and canonical key derivation assertions. See
`measurements/native_library_tests.log` for those failures.

The release build and all five CUDA controls above completed successfully.
Each control matched every core node and edge for all sixteen seeds, with zero
mismatches. Four additional enclosures produced five transforming dyads and
recovery at every dyad. The identity control returned N; the reversal control
returned T while changing Tt to Ft at its fuse. The complete device output is
in `measurements/native_graph_cuda_controls.log`.
