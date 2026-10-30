# G-mOMonadOS user guide

## Operating programs

Run these commands from this directory. Python 3.11 or newer runs the launcher;
Rust and the selected target are needed when building Rust programs. `doctor`
prints available build tools and installed Rust targets. GPU execution requires
a compatible CUDA device and driver.

```bash
./programs --help
./programs doctor
./programs list
./programs list --all phase
./programs menu
./programs cli --help
```

`list` shows named Rust binaries. `--all` also includes examples, shell and
Python tools, and the live nested Cargo projects under `membranes/` and
`gpu_port/`. Search by part of a name or description. Use the full name printed
by the list when names collide, for example `bin:NAME`, `example:NAME`,
`script:tools/NAME.sh` or `membranes/bin:NAME`.

| Action | Command |
| --- | --- |
| Read usage without starting a computation | `./programs help NAME` |
| Build one Rust program | `./programs build NAME` |
| Build as needed and run with arguments | `./programs run NAME ARGUMENTS` |
| Use the main command interface | `./programs cli ARGUMENTS` |
| Run an already built program | `./programs --no-build run NAME ARGUMENTS` |
| Use a debug build | `./programs --debug run NAME ARGUMENTS` |
| Run a saved native membrane or serialization | `./programs execute PATH` |
| Choose a program and enter its arguments | `./programs menu` |

Global options such as `--no-build` go before the action. Arguments after a
program name are passed through as separate arguments. Quote an IMASM word or
an argument containing spaces. Program arguments use this repository as their
working directory. `execute` and `bake` resolve supplied paths relative to the
calling directory, so absolute launcher paths also work from elsewhere.

The launcher checks static ELF linkage before returning or running a native
image. A `.imasm` or `.glyphs` input to `execute` runs through Vox. Baking never
starts the resulting factor search. Press Ctrl+C to stop a running program;
the launcher imposes no time limit. Computations can retain their own algorithmic
bounds and return an open state. A user-selected profiling duration is separate
from a baked execution timeout.

### Bake from ordinary inputs

```bash
./programs bake --list
./programs bake --help
```

Choose a family explicitly to get the same behavior from either directory.
`--number` accepts decimal N. `--input` accepts a file containing decimal N
alone. `--word-file` accepts saved IMASM for the word and ququart families.
`--output` must name a fresh path. Without it, a fresh timestamped path is
created under the family repository's `membranes/` directory. The final
`Ready:` and `Run:` lines give the native image path and direct execution
command. Build logs and preparation records remain beside the image.

| Family | Example using an existing source file | Native output at `--output CASE` |
| --- | --- | --- |
| Source-only factor | `./programs bake factor --input SOURCE.decimal --output CASE` | `CASE/factor_one` |
| Prepared resident | `./programs bake resident --input SOURCE.decimal --output CASE` | `CASE/resident_factor_baked` |
| Rust phase | `./programs bake rust-phase --input SOURCE.decimal --output CASE` | `CASE` |
| C phase | `./programs bake c-phase --input SOURCE.decimal --output CASE` | `CASE`, plus `.imasm` and `.glyphs` sidecars |
| Ququart | `./programs bake ququart --word-file SOURCE.imasm --native-fourier --output CASE` | `CASE/membrane` |
| GPU idempotent | `./programs bake gpu --input SOURCE.decimal --output CASE` | `CASE`, with embedded device code |
| CPU idempotent | `./programs bake cpu --prepared PREPARED_DIRECTORY --output CASE` | `CASE`, plus preparation and serialization records |
| Saved IMASM word | `./programs bake word --word-file SOURCE.imasm --output CASE` | `CASE` |

The default family is `factor` in Vox and `resident` in G-mOMonadOS. Phase and
ququart bases default to 2; `--base` changes that compilation input. Ququart
`--retained-case DIRECTORY` reuses compatible preparation instead of
`--native-fourier`. Word baking selects the first nonempty source line by
default; `--line` selects another. CPU baking needs the source directory emitted
by the GPU preparer, including `membrane.cu` and `preparation.json`. Its default
compilation prepares through closure; `CPU_PREPARE_TO_CLOSURE=0` selects an open
diagnostic preparation. Neither mode has a wall-clock cutoff.

A baked native image carries its instance inputs and required computation.
Run the image directly with no source argument. Python, Cargo and the source
files are build tools and inputs; GPU device execution uses the host driver.
Use `python3 ../Vox/standalone.py PATH_TO_IMAGE` to inspect linkage independently.

### Check the operating interface

The retained operating checks use CPU commands and an N-only near-balanced
224-bit source. From either directory:

```bash
../Vox/programs run tools/check_program_operations.py
```

Add `--bake` to check word, factor, resident, Rust phase, C phase and ququart
baking. This retains native images and checks static linkage. It executes the
word image from an empty directory and environment. Factor searches and GPU
execution are outside this check. Logs and `receipt.json` remain under
`Vox/measurements/program_operations_20261015/`; `--output` selects a fresh
results directory. There is no imposed execution timeout.

### Extend the CPU isolation checks

From Vox, bake two fresh near-balanced 224-bit sources through the shell
perfect-membrane and Python sparse-readout bakers:

```bash
./programs run tools/check_baked_isolation.py
```

The final line prints the retained receipt. Set `RESULTS` to its containing
directory, then extend the same run:

```bash
./programs run tools/check_baked_isolation.py --glyphs --output "$RESULTS"
./programs run tools/check_baked_isolation.py --trace --output "$RESULTS"
```

From G-mOMonadOS, use `../Vox/programs` for these commands. The shell tests
exercise two nesting depths with inherited flags requesting dynamic linkage.
The images must still be static, retain their baked input, and ignore runtime
source overrides. The sparse queries are compared with independent arithmetic.
Glyph checks require exact module recovery, agreement with native output and
rejection of a corrupted image. The trace pass requires permission to trace CPU
processes; it rejects source-file opens and helper launches. It does not perform
GPU execution or factor searches. Reference factors remain separate from the
N-only build inputs.

### Tested entry points

```bash
./programs help g-momonados
./programs help membranes/bin:membrane_ecm_extract
./programs --no-build cli --help
./programs --no-build cli numeral encode 266175155279792434973506259190185380993589458872977057421959357
```

`numeral` and `native_numeral` name the same direct source-word command. The
decimal example is the retained balanced 208-bit source. To bake the retained
fresh 224-bit source with a new output path:

```bash
./programs bake resident --input ../Vox/measurements/static_bakers_20261015/source.decimal
```

For REPL commands, `./run_cmds.sh "COMMAND" ["COMMAND" ...]` builds the main
program and feeds each command followed by `quit`. `./run.sh` opens the prompt.
The launcher's `cli` action also accepts REPL command arguments through the
main program's existing dispatch.

## Build and run

Run commands from this directory. Hosted builds link the C runtime statically
by default through the local and parent Cargo configuration.

```bash
cargo build --release --bin g-momonados
./target/x86_64-unknown-linux-gnu/release/g-momonados
```

`./run.sh` builds as needed and opens the prompt. `./run_cmds.sh "COMMAND"`
dispatches a command without an interactive session. GPU commands require an
exposed compatible CUDA device and driver. CPU commands can run without one.
The [README](README.md) introduces the available mechanisms; the
[command index](../ig-docs/g_momonados_command_index_20261012.md) maps their entry points.

## Static membrane output

Every live membrane baker must emit a static, self-contained native executable by
default. The image contains its source word, baked instance constants, prepared
records and required computational code. Device code is embedded for GPU images;
device execution still uses the host's CUDA driver.

A successful bake requires an ELF executable without an external interpreter or
shared-library dependencies. Static PIE is accepted. The shared build-time checker
is [Vox/standalone.py](../Vox/standalone.py). Python and shell tooling are needed
to bake the image; the emitted image does not need those tools, the source tree
or the original input files to load its baked computation.

Baking and execution are separate steps. Bakers print their retained executable
path. Run that image directly when ready. `make_membrane.sh` also supports
`MEMBRANE_RUN=1` to execute immediately after baking. Static-image admission
checks linkage; a factor result requires a separate execution and exact product receipt.

## Bakers and retained outputs

Here `N` is a decimal source supplied at build time. Use a fresh output directory
or filename where a baker rejects an existing destination.

| Command | Retained native output |
| --- | --- |
| `./bake_resident.sh "$N" [OUTPUT_DIRECTORY]` | `OUTPUT_DIRECTORY/resident_factor_baked`; defaults to a fresh directory under `measurements/baked_resident/` |
| `./make_membrane.sh "$N" [base] [shots]` | `target/x86_64-unknown-linux-gnu/release/examples/membrane_<N>`; base defaults to 2, shots to 12 |
| `./imasm_phase_factor_build.sh "$N" [base] [OUTPUT] [sweep\|single] [sparse\|every]` | `OUTPUT`, defaulting to a source/base fingerprint under `membranes/` |
| `./vox_phase_factor_build.sh "$N" [base] [OUTPUT] [sweep\|single] [sparse\|every] [nested-depth]` | `OUTPUT`, with `.imasm`, `.glyphs` and retained `.source/` sidecars |
| `bash tools/bake_recycled_phase.sh "$N" [base] [shots]` | `measurements/baked-qpe/<N>/phase_unbraid` with its source word and hash |
| `python3 prepare_ququart.py @SOURCE_WORD_FILE OUTPUT_DIRECTORY [options]` | `OUTPUT_DIRECTORY/membrane`, preparation records, verifier and manifest |
| `bash tools/bake_idempotent_cpu.sh PREPARED_SOURCE_DIRECTORY OUTPUT` | `OUTPUT`, retained CPU preparation, linkage report, bindings and `.imasm` serialization |

The resident baker converts N to an IMASM word and generates its split/fuse
circuit receipt through `qc HTSX 3 1`. Cargo then constructs and embeds the
resident preparation in the static musl executable. The preparation inputs and
build diagnostics remain in its output directory.

The Rust phase baker defaults to base 2, seed mode `sweep` and support mode
`sparse`. The C phase baker defaults to base 2, `sweep`, support mode `every`
and nested extraction depth 1. Its primary output is the native executable.
Use its `.imasm` or `.glyphs` sidecar explicitly for `vox run`.

`prepare_ququart.py --help` lists word-valued precision and operator options.
`--native-fourier`, `--compiled-report` and `--retained-case` select preparation
inputs. Its `--dynamic-work` option controls when nested arithmetic operations
are generated; the emitted executable still passes the static linkage check.

The CPU idempotent baker prepares through closure by default.
`CPU_PREPARE_TO_CLOSURE=0` selects an open diagnostic image. The GPU idempotent
emitter in `src/idempotent_closure.rs` embeds prepared banks and device code and
checks its static native image. The raw ELF compiler and linker, `imasm_emit.py`
and `imasm_link.py`, also validate their emitted native images.

## Inspecting a result

```bash
python3 ../Vox/standalone.py PATH_TO_MEMBRANE
readelf -l -d PATH_TO_MEMBRANE
./PATH_TO_MEMBRANE
```

The checker exits unsuccessfully if the image requires an ELF interpreter or
shared library. Native execution uses the binary path. Serialized execution uses
the corresponding `.imasm` or `.glyphs` path with Vox.

### Native resident execution

The resident and ququart baked paths execute their admission, retained Y
continuation, clock operations and product evidence inside G-mOMonadOS. They do
not launch the external Lean bridge. `src/membrane_kernel.rs` implements the
SIXTEEN_3 carrier in Lean membership order `[N,T,F,B]`, keeping it distinct from
the constructive IMASM register coordinates `[T,F,t,f]`.

The resident uses the retained Curry Y reducer and the certified clock record
on the same carried state. Actual contractions, operator calls and clock ticks
remain available through open and closed returns. Work completion drives the
continuation; there is no tick-count stopping rule. The baked entry point loads
the prepared carrier emitted by Cargo.

Ququart admission executes the retained Y reduction and checks the recursive
carrier's fixed-point bounds locally. Product evidence checks both nontrivial
arms and exact word multiplication against the source, retaining contradictory
B membership. The interactive Lean commands remain available as separate formal
instruments.

The [static baker reference](../ig-docs/membrane_bakers_static_20261015.md)
and its [source inventory](../ig-docs/membrane_bakers_static_20261015.json)
bind the live recipes to validation receipts. Checks use a fresh N-only
near-balanced 224-bit source. The CPU diagnostic check also retains its original
300-bit preparation boundary. Historical proof snapshots keep their original
contents and build records.
