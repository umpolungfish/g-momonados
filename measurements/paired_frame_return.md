# Paired evaluation-frame return

The canonical computation is `.imasm/paired_frame_library.imasm`. It packs
two numeral cells from each operand into two FOUR registers, carrying the
FOUR × FOUR representation of SIXTEEN_3, then reverses that representation
into the original four cells. Numeral cells use T for zero and F for one.
The first cell of each pair occupies the truth-support coordinate and the
second occupies the falsity-support coordinate.

`product_closure_encoded_lsb_first` now wires this transport ahead of its
multiplier and equality closure. The compiler emits register addresses from
the input widths. The IMASM subroutine performs all pair packing and reversal.
The shorter operand receives high-order zero cells during transport; only
its original cells are written back. No numeric decode occurs in this path.

`.imasm/paired_frame_return.imasm` is the streaming entry used with the
library. It consumes groups `p0 p1 q0 q1` until the boundary N and emits the
returned cells. ParaVM buffers these emissions.

Verification:

```sh
RUSTFLAGS='-D warnings' cargo test --test paired_frame_return
RUSTFLAGS='-D warnings' cargo build --bin g-momonados
PROFILE=debug timeout --kill-after=2s 60s ./run_cmds.sh \
  'imasm_close ⊥⊥⊥ ⊥⊥ ⊥⊤⊥⊤⊥' \
  'imasm_close ⊥⊥⊥ ⊥⊥ ⊤⊤⊥⊤⊥'
```

All 37 tests passed. The dedicated transport tests cover every paired state,
empty input, unequal widths, and streams through 1,025 groups. The same run
includes the existing arithmetic and closure tests. The CLI reports exact
closure for 7 × 3 = 21 and mismatch for target 20.

This installs the paired-return stage in the runtime closure path. The Lean
phase-selection and complementary-recovery connection remains the next stage
to bring into that instruction stream.
