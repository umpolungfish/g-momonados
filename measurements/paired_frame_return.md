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
cell-binary IMASM words before release compilation. The resulting binary takes
no operands. It enters an odd-modulus Montgomery evaluation frame and
repeatedly squares a phase residue through one resident IMASM circuit. Each
square uses conditional encoded additions of the residue and modulus, shifts
the joint register, and cancels one remaining modulus contribution. The entry
circuit normalizes a base of any width and transports it into that frame.

At phase index zero and the dyadic checkpoints, one width-eight frame evaluates
N's support polynomial at the phase residue. Its coefficient groups are the
same bit stream regrouped eight at a time. A proper IMASM gcd of this readout
with N selects the first factor register. If no support target closes, the
full phase-return route remains active. The odd-anchor complement circuit
reads one residual cell at a time. Since the selected factor is odd, each
complementary bit is forced by the residual low bit; a set bit cancels one
copy of the selected factor before the residual shifts. It emits the
complementary register only when the terminal residual is zero. That terminal
state is the extraction's own exact closure of `P × Q = N`; no separate
multiplication check follows. Output is buffered until the pair closes.

The resident IMASM VM stores phase cells in a dense register bank and links
branch labels to instruction addresses when the circuit loads. The builder
uses the optimized release profile; the earlier debug-profile build made the
same interpreter circuit substantially slower.

The host holds only encoded tapes, phase addresses, and the circuit's control
flow. It performs no factor arithmetic. Register widths follow the encoded
source and factor words. The binary bakes N and the base as full IMASM words.
The factor-return GCD circuit splits common powers of two into an encoded
scale register, shifts even arms, cancels the smaller odd arm from the larger,
then multiplies the odd result by the scale. It loops in IMASM until one arm
is zero. Its instruction stream has no quotient or restoring-division stage.

Direct release-binary trials with `timeout --kill-after=2s 60s`:

| Baked N | Returned factors | In-process time | Result |
|---|---|---:|---|
| 21 | 7 × 3 | 0.903 ms | closed |
| 35 | 7 × 5 | 1.171 ms | closed |
| 143 | 11 × 13 | 1.809 ms | closed |
| 8051 | 83 × 97 | 5.084 ms | closed |
| 10002200057 | 100003 × 100019 | 37.663 s | closed |
| 580284393595165992175009793 | 3221225473 × 180143985094819841 | 319.100 ms | closed |
| 10007000070049 | 10007 × 1000000007 | 60 s with restoring reduction | timed out |

The 11-digit case timed out in the earlier debug-profile binary and closed in
the release-profile binary. The 27-digit pair also closes in the release
binary, with both factors represented only as emitted IMASM numeral words.

`RUSTFLAGS='-D warnings' cargo test --test paired_frame_return --quiet`
passed all 44 tests. These include comparison of repeated resident phase
squares with the independent encoded arithmetic path, exact and rejected
complements, a 131-bit complement, the SIXTEEN_3 transport, and the existing
arithmetic closure cases. A support-polynomial frame test extracts 5 from the
phase register for modulus 15 and emits its complement only at zero residual.
Frame-eight support values also match encoded Horner evaluation across full and
partial frames.
The latch test runs the IMASM stream with both exact
and mismatched targets and checks that only the exact closure is fixed. The
GCD tests cover zero arms, all small pairs, and 65- and 129-bit operands. The
Montgomery tests compare shifted-frame squaring with the encoded arithmetic
control across odd moduli, a 129-bit modulus, and an oversized base.
