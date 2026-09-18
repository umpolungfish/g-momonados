# G-mOMonadOS

![Rust](https://img.shields.io/badge/language-Rust-CE422B?style=for-the-badge&logo=rust&logoColor=white)
![GPU](https://img.shields.io/badge/CUDA-GPU%20path-76B900?style=for-the-badge&logo=nvidia)
![License](https://img.shields.io/badge/license-Unlicense-1A1A1A?style=for-the-badge)

G-mOMonadOS is the hosted, GPU-native build of the mOMonadOS architecture.
It runs the Imscribing Grammar, IMASM words, trilattice state, quantum and
Vox tooling, and ABC/IUTT measurement readers from one Rust executable.

The central runtime loop is:

```text
THINK → ACT → OBSERVE → UPDATE
```

The hosted runtime enters through one recursively enclosed IMASM process vessel.
Each REPL command is admitted into that vessel, executes at its inner action
leaf, and emits through its terminal surface. The Crystal is the Grammar's
finite address space; the trilattice retains truth, falsity, information, and
held states. The GPU path batches the same gate operations as the CPU reference.

## Start here

```bash
cd /home/mrnob0dy666/imsgct/G-mOMonadOS
make hosted
./run_cmds.sh 'help'
./run.sh   # interactive session
```

GPU commands need an exposed CUDA device; CPU and Vox commands work without one.

## Useful commands

```text
help
fibqc help
vox help
gpu16_3 verify [n] [device]
gpu_sixteen3_tensor_kernel [n]
gpu_crystal_full_space
abc stream [eps] <cutoff...>
abc closure [eps] <cutoff...>
abc champions [eps] <max_c>
```

`--json` forms feed the Lean certificate scripts; champion traces enumerate
displacement events. See `docs/abc_champions_trilattice.md` for the format.

## Repository map

| Path | Contents |
|---|---|
| `src/` | Rust runtime, REPL, trilattice, ABC, GPU, quantum modules |
| `scripts/` | JSON-to-Lean certificate tooling and audits |
| `docs/` | command and certificate documentation |
| `measurements/` | GPU benchmark records |

Verify: `cargo check --features hosted`, `cargo test --features hosted`.
Lean artifacts: `cd ../p4rakernel/p4ramill && lake build`.
Full notes: [`README_FULL.md`](README_FULL.md). Unlicense.

$\mu\circ\delta = \mathrm{id}$
