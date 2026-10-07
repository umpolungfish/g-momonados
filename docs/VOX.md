# V⊙x — control-flow closure auditor

Lifts a program's control flow to a twelve-glyph IMASM word and runs the
SIXTEEN_3 verdict over it. `src/vox.rs` holds the classifier and the verdict,
`src/vox_decode.rs` the x86-64 decoder. Reached from the REPL as `vox`.

```
vox verdict <word>    SIXTEEN_3 verdict over a glyph word
vox classify <mn>     which glyph an instruction lifts to
vox lift <path>       decode an ELF and lift its executable sections
```

## What the verdict reads

A word closes at **T**, carries an open fork at **B**, and runs clean and linear
at **N**. Only three glyphs move the verdict: `∈` opens a fork, `∋` fuses one,
`⊣` anchors. Everything else is carried but does not decide.

`⊢` and `∋` are not instructions. `⊢` opens the word; `∋` marks an address with
two or more predecessors. x86 has flat control flow, so both are recovered by
analysing the instruction stream rather than read off any single instruction.
wasm, which has structured control flow, has real opcodes for both.

## The decoder refuses rather than guesses

`decode_one` returns the instruction's length and enough of its shape to name a
mnemonic the classifier already understands. It does not reconstruct registers
or operands, because the glyph does not depend on them.

An opcode it does not know returns `None`. The walk stops there and reports how
far it got, so a partial lift always reads as partial:

```
stopped at +0x20da9a on an opcode the decoder does not know: 49 92 4c 87 e5 …
```

This is the load-bearing decision in the file. A length decoder that guesses a
width goes out of phase with the instruction stream and keeps producing
instructions — wrong ones. The word still verdicts, and the verdict is fiction.
Refusing makes the failure visible; guessing makes it invisible.

Each refusal names the bytes, so extending the table is mechanical. Coverage on
`/bin/ls` went 15% → 41% → 44% → 100% in four rounds, the stops being string
operations, the `0F BA` bit-test group, the whole x87 `D8..DF` escape, and
`xchg` with the accumulator.

## Coverage

100% of bytes decoded, no stops, on `/bin/true`, `/bin/ls`, `/bin/bash`,
`/usr/bin/git`, `/usr/bin/python3`, and on the kernel's own binary — 690,031
instructions across 3.2MB of its `.text`.

Verdicts distribute by shape rather than by size. The `_init` thunk closes at
**T**: it opens a fork and fuses it before the anchor. PLT stubs run **N**,
linear with no fork. A real `.text` read whole sits at **B**, a fork still open
at a terminal, which is what an entire program's control flow looks like when
read as one word.

## Two defects this surfaced

`parse_elf` read `sh_type` at section-header offset `+0`, which is `sh_name`.
Nothing ever matched `SHT_PROGBITS`, so it always returned zero executable
sections and the ELF path had never run. Both the 64-bit and 32-bit field
offsets were shifted by one field.

`compute_merges` looked up predecessors and addresses by linear scan, making the
lift quadratic. It completed on small objects and hung on a real binary.
Predecessor counts now use a `BTreeMap` and address membership a binary search
over the ascending address list; the same 517k-instruction section that hung now
lifts in under two seconds.

## What is not reconstructed

Operand detail. `vox lift` synthesises an operand string only where the glyph
depends on it: a direct branch target, a memory destination, or the absence of
an immediate target that makes a branch indirect. Anything finer would be a
disassembler, and the word does not read it.

## Source-bound ququart extraction debugging

Only the current candidate and its immediate comparison membrane are kept.
Superseded shared-mask, branch-index, per-wire and modular-trace binaries and
generated disassemblies have been removed. Preparation manifests, source words,
validation records and native sample traces remain in their case directories.
Historical ELF addresses below refer to the recorded builds, not current files.

The baked source `229513619370652772473594096727489823787` passes native
composite screening and preparation validation. Vox native instruction samples
and the baked ELF's IMASM symbol map locate the stalled extraction in decision
node interning, support-mask destruction and slot reclamation. The samples are
in Vox's `measurements/ququart_source229513_profile.samples.tsv`; the symbol
map and disassembly are retained alongside the original membrane and in Vox's
measurement directory. Each profile is a bounded observation of a live extraction.

The decision arena shares immutable empty support masks across leaves and
reclaimed slots. Equal shared child masks reuse storage; copy-on-write still
protects live masks when setting branch bits. Exact complex-coordinate, slot
reuse, shared control correlation, modular arithmetic and SIC controls pass.
The modular test generates its canonical numeral words through the codec
instead of depending on missing historical membrane files.

The rebuilt membrane is retained in
`measurements/ququart/source229513_shared_masks`. Its bounded extraction reaches
120 seconds with no completed readout, which the separate verifier rejects.
The new Vox profile still concentrates in node interning, equality lookup and
reclamation. Those operations remain the next measured bottleneck.

The per-wire branch index membrane is retained in
`measurements/ququart/source229513_wire_index`. Vox disassembly resolves
the source-digit modular translation at address `0x94b14` and reclamation
at `0x9ae92` in that ELF. Its native counter trace records three completed
phase digits out of 132 at 24.595771 seconds, with 114,068 retained nodes
and 4,096 nested operations. The separate unprofiled execution reaches its
30-second bound before emitting a completed readout. The retained trace is
`Vox/measurements/ququart_source229513_wire_index_profile.ququart.tsv`.
The next debugging target is the growth of the shared modular-work diagram
during the following phase digit.

The modular-stage trace for `source229513_modular_trace` separates completed
addition boundaries (stage 0), source interval partitioning (stage 1),
low-interval translation (stage 2), high-interval translation (stage 3),
interval fusion (stage 4) and invalid-arm fusion (stage 5). In the bounded
Vox execution, 1,978 samples land between additions, 161 in low translation,
163 in high translation and two in partitioning. No sampled stop lands in
either fusion stage. Temporary entry diagrams exceed 225,000 nodes while
the retained phase diagram remains near 114,000 nodes. The next target is
temporary-node creation and reclamation around interval translations.

Temporary-node cleanup drains the operation's existing allocation list as a
shared work stack and retains its capacity for the following operation.
Decision-coordinate, slot-reuse, control-correlation, modular-coherence and
SIC controls pass. The source-bound `source229513_batch_reclaim` membrane
still reaches the standalone 30-second bound without a completed factor
readout; its verifier reports `missing completed factor readout`. Vox locates
the hottest sampled instruction at `0x9b11d` within shared-stack reclamation
in this ELF. The phase trace remains at three completed digits near 25 seconds.

The current `source229513_coordinate_hash` candidate uses a mixed internal
child-coordinate hash with exact pair equality; amplitude-leaf indexing is
unchanged. All decision and shared-work controls pass. Its bounded 30-second
execution emits no completed readout, and the verifier reports the missing
readout. This execution overlapped the control-test build and is not a clean
timing comparison.

The current candidate is `source229513_fold_batch8`; its comparison baseline
is `source229513_coordinate_hash`. Nested arithmetic collects temporary nodes
every eight operations, and multiplication, feedback and measurement boundaries
still collect immediately. Shared-work correlation, modular coherence and SIC
controls pass. Vox measures the fourth phase digit at 22.931900 seconds,
versus three completed digits at the coordinate-hash baseline's 25-second
bound. At 24.751296 seconds it records four measured digits, 4,112 nested
operations and 454,995 retained nodes. The superseded batch-reclamation
membrane binary has been removed; its measurement records remain.
The standalone 30-second execution reaches its bound without a completed
factor readout; the separate verifier reports the missing readout.

Vox disassembly maps the shared-mask build's hottest equality sample to
`Node::equivalent` at `0x99e15`, where the hash-table key's enum tag is compared.
The branch index stores `(wire,low,high)` separately from exact amplitude leaves.
The rebuilt executable's sampled hot reclaim instruction is the compact-key
comparison at `0x9ad94`. Exact coordinate and shared-work controls pass; its
120-second source-bound extraction still ends without a completed readout.
The next inspection is branch-key lookup and deletion, following the new live PCs.
Vox keeps both full disassemblies and bounded native register traces under
`measurements/ququart_source229513_{shared_masks,branch_index}_*`.
