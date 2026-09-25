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
The pair frames fuse before the equality register reaches `IFIX`. An exact
product is emitted as `T [FIXED]`; a mismatch is emitted as `F` without fixing.

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

The dedicated transport tests cover every paired state,
empty input, unequal widths, and streams through 1,025 groups. The same run
includes the existing arithmetic and closure tests. The CLI reports exact
closure for 7 × 3 = 21 and mismatch for target 20.

This installs the paired-return stage in the runtime closure path. The
contained phase extractor below connects selection and complementary recovery
to that stage.

## Contained phase extractor

`./imasm_phase_factor_build.sh N [base] [output]` converts N and the base to
cell-binary IMASM words before compilation. The resulting binary takes no
operands. It repeatedly squares a phase residue through one resident IMASM
square-and-reduce circuit. A returned phase relation enters the IMASM
subtractor and GCD circuits to select a factor register. The odd-anchor
complement circuit reads one residual cell at a time, cancels the selected
contribution, and shifts the signed carry to recover the second register.
The two returned registers then enter the paired SIXTEEN_3 transport and
product closure. Output is buffered until the pair closes.

The host holds only encoded tapes, phase addresses, and the circuit's control
flow. It performs no factor arithmetic. Register widths follow the encoded
source and factor words. The binary bakes N and the base as full IMASM words.

Direct binary trials with `timeout --kill-after=2s 60s`:

| Baked N | Returned factors | In-process time | Result |
|---|---|---:|---|
| 21 | 7 × 3 | 20.8 ms with fused closure latch | closed |
| 35 | 7 × 5 | 21.7 ms | closed |
| 143 | 11 × 13 | 49.3 ms | closed |
| 8051 | 83 × 97 | 202.6 ms after resident square and cancellation | closed |
| 10007000070049 | 10007 × 1000000007 | 60 s | timed out |

The 14-digit case reached the same one-minute boundary before and after
resident square assembly. Its phase orbit, rather than repeated assembly or
complement division, is the remaining measured bottleneck. The executable
does not emit partial reports while the phase orbit is open.

`RUSTFLAGS='-D warnings' cargo test --test paired_frame_return --quiet`
passed all 41 tests. These include comparison of repeated resident phase
squares with the independent encoded arithmetic path, exact and rejected
complements, a 131-bit complement, the SIXTEEN_3 transport, and the existing
arithmetic closure cases. The latch test runs the IMASM stream with both exact
and mismatched targets and checks that only the exact closure is fixed.
