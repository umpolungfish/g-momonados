# Open terminal readout experiment

The internal fork/fuse reconnections remain closed. The terminal `⊣` output
remains a living end, available to its caller or output instrument. The input
Diagrammar files are explicit experimental reference graphs, not captured
machine-code graphs; the rewirer independently derives the complete executable
graph from each ELF.

## Compiled control

`examples/open_phase_readout.rs` binds source 15, base 2, a four-bit phase
register and Born rank 5/8. It prepares and validates a nested operation pool,
executes those operations over the shared register, applies the full inverse
Fourier transform, and returns the complete measured phase word. It supplies
no phase outcome to preparation and performs no factor extraction.

Both `control.elf` and its rewritten executable IMASM produced:

```
whole_register_phase_word=4
```

`control_report.json` records matching native/rewired output, ELF and tape
hashes, the terminal `⊣` with zero outgoing CFG edges, and complete instruction
coverage. The rewirer reports 11,355 closed reconnections, zero open pairs,
zero opaque bytes and zero unresolved transfers. A decoder gap for the
runtime's `movmskps` was repaired in Vox and verified with decode/execution
tests. The original incomplete output remains in the `decoder_gap` files.

## Baked 895-bit target

`target895_open.elf` was compiled from the current `ququart_factor_baked`
source using the existing 895-bit prepared input. `--open-readout` exposes the
whole phase sample as a flushed JSON event before factor closure runs. The
source, base, width, numerator, denominator and shot are canonical numeral
words. This flag does not alter the phase operator or supply factors.

The supplied target input has a controlled-power schedule but no serialized
`prepared_work` pool. The baked path consumes and validates a pool when one is
present; otherwise it uses the existing source-bound elementary emitter.

The target rewire passed: 174,428 instructions, 1,602 functions, 16,874 closed
reconnections, complete coverage, zero open pairs, zero opaque bytes and zero
unresolved transfers. These are structural checks; they do not certify a
measured phase or factors.

Separate native and rewired observations reached their 45-second observation
limits without phase output. At the user's request, the rewired target was
then launched without a wall-clock timeout, with the VM step budget set to
`18446744073709551615` and no added address-space limit. Its session handle
and output paths are recorded in `target895_report.json`.

Live output: `target895_unbounded.stdout` and `target895_unbounded.stderr`.
The run record describes its launch; inspect the live process and output to
determine its later state.

## Verification

All five `whole_register_phase::tests` passed, including terminal readout
consumption before extraction. The baked binary check passed. Vox's sign-mask
decoder and execution checks passed. The compiled control ran successfully
through the rewritten IMASM in Vox.

The first unlimited execution blocks in `StdHost::read(0)` while its Lake child has already exited. GDB resolves the host stack through `Machine::run_loop` to a real stdin read. The VM previously returned success for `pipe2` without assigning descriptors. Hosted pipe creation now supplies real descriptors, emulated forks own separate descriptor tables, and redirected stdio retains its own descriptor when the original is closed. The focused host tests pass for error-pipe EOF, redirection lifetime, and exclusive capture creation.

The repaired release VM runs the same target tape with `--open-readout`, unlimited `VOX_STEPS`, and no timeout in session 96710. Output is captured in `target895_pipefix.stdout` and `target895_pipefix.stderr`; the earlier `target895_unbounded` captures are preserved. Its real Lake evaluator is active after restart.
GDB on the restarted target resolves the current stack to `parse_imm`, `Machine::read`, and `Machine::run_loop`, confirming execution beyond the former host-stdin block.
