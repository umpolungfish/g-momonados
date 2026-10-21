# Native paired numerals and read-off

A numeral carries its name and its two operand legs together. Multiplication
constructs that carrier. Reading projects the retained legs. Construction,
source admission, read-off, and predicate execution call no factor producer.

The native implementation is `G-mOMonadOS/src/conventional_translation.rs`.
G-mOMonadOS direct argv and its REPL dispatch to the same module. The root
`conventional_translation.py` uses that module through the compiled kernel and
provides SelfReadingFixedPoint, DualNamedNumerals, and Membrane interfaces.
The earlier Python source is preserved under its dated historical filename.

## Construct and read

Set LEFT and RIGHT to the coprime operand numerals used to construct the name.
The canonical source carries the decimal values and their complete IMASM words.
A copied source can be reopened without a separate operand file.

```bash
KERNEL=G-mOMonadOS/target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/g-momonados
"$KERNEL" conventional multiply "$LEFT" "$RIGHT" --write-source paired.json
"$KERNEL" conventional read --source paired.json
python3 conventional_translation.py both --source paired.json --receipt predicates.json
```

The Python interface also accepts explicit name-and-legs admission. `--rank`
selects the natural-ordinal instance of the retained rank family. `--value`
selects a value to pass through CRT split and fuse. `--require-all` returns a
nonzero status while any full predicate lacks an affirmative receipt.

```text
operand legs -> multiplication with retained shape -> saved paired numeral
saved paired numeral -> source admission -> projection -> the same two legs
                                      -> shared resident Y / CRT return
                                      -> separate predicate instruments
```

Source admission checks the exact product, the independent IMASM support
convolution, coprimality, and every canonical word. Corrupted words, mismatched
names, and noncoprime legs are rejected. The reader returns references to the
stored shape. Process timings include kernel admission, input validation, JSON
construction, and printing; they are not timings of projection alone.

Coprime moduli establish two CRT components. Exactly two prime spectrum points
additionally require prime legs. The integration corpus constructs its paired
inputs from randomly generated balanced prime pairs, with their input boundary
attached. These are paired-source read-off measurements.

## The carried constructions

Both structures retain one source object. They share exact CRT idempotents,
component multiplication and its naturality return, the source-bound polynomial
Heisenberg module, the stationary two-component dephasing state, and the
source-bound diagonal-copy correlation. The retained Y reducer executes on the
same source, ordinal representation, and residue pair. Its calls and contractions
are returned with the split/fuse evidence.

The commutator is computed with Gaussian-integer polynomial coefficients on an
unbounded-degree invariant domain. There is no finite-difference surrogate. The
steady-state generator is evaluated with exact matrix numerators. Both spectral
components remain present in that state. The dephasing rate is positive and
source-bound; the finite relaxation-to-observation ratio is returned explicitly.

Diagonal copying retains the same spectral label at separated sites. Its exact
connected-correlation numerator is the squared difference of the two labels,
with denominator four. Transfer preserves both idempotents. The resulting
nondecaying correlation has zero inverse correlation length in this construction.

The self-reading topology uses a disjoint Hopf-link disk-intersection witness;
its scope is that carried link model. Its infinite family is indexed lazily.
The dual-named topology embeds the two named points in an interval and enumerates
the proper subsets of the finite two-point carrier. Their final instruments
respectively return source-bound integer winding and a two-valued holonomy sign.

## Ordinal family and full reflection

`G-mOMonadOS/src/conventional_translation.lean` proves a payload-preserving
fixed-point family for every ordinal. Its carrier is an actual ZFC set obtained
from the successor ordinal. The theorem uses von Neumann rank, proves the rank
exceeds the requested ordinal, and places the carrier in a von Neumann level.
The native code retains the symbolic natural-ordinal instance and the same
payload through its executing return. The mathematical family is not inferred
from a finite sequence of runtime counter values.

The local p4rakernel elaborates the ordinal and correlation theorems. The saved
formal receipt contains the exact source, source hash, kernel identity, and
printed axiom dependencies. The ordinal theorem uses the standard foundations;
the constant-correlation theorem has no axiom dependency. The native report
rejects a formal receipt whose source text differs from the compiled Lean source.

The native self-model prepares the existing ParaVM disassembler and recompiler
by executing both tries over the complete opcode space. It applies their
computed actions to the complete paired source and to both programs themselves.
The kernel proves the return for every finite code word and transport of every
predicate on that native code domain. The same construction inhabits the live
SHIAB holographic predicate. The report includes the two program texts, executed
maps, interpreter step count, source extent, and actual self-return. A changed
compiled inverse changes the returned payload in the rejection control.

Slot one also retains the stronger constructible-universe reading from the
supplied text. The code-domain reflection theorem does not supply a set-sized
model satisfying that full set-universe schema. This full predicate remains
unestablished. The report separately returns native_operations_hold for the
executed native construction and all_predicates_hold for the full conjunction.

## Reproduction and evidence

Build the hosted native kernel with Cargo, then execute the integrated runner:

```bash
cd G-mOMonadOS
cargo build --offline --release --bin g-momonados --target-dir target/conventional_translation_20261020
cd ..
python3 G-mOMonadOS/measurements/conventional_translation_20261020/run.py
```

The measurement directory preserves the generated paired inputs, canonical
sources, both structure returns, Python/native equality checks, actual REPL
output, rejection controls, formal source binding, and build output. The final
`result.json` binds the executable and active sources. Earlier protein work is
paused and has no completion claim from this implementation.

## Distinct arithmetic trees and their common return

The v2 paired source retains a binary sum of distinct positions and a product
of its supplied operands. Each evaluates independently to the source numeral.
The closure receipt compares evaluated values and explicitly preserves distinct
structures. Retained operand leaves are not certified prime leaves, so their
count is not reported as Ω. No factor producer supplies either tree. Valid v1
paired sources remain readable and are upgraded to the retained representation.

`conventional boundary <natural>` exposes zero as an empty binary sum and
multiplicative absorber, and one as the empty multiplicative product. Numerical
zero is not identified with semantic bottom. Multiplication by zero absorbs a
positive input in one step; multiplication by one preserves it; multiplication
by a larger natural grows without bound. These boundary checks are algebraic
cases, separate from the paired-source execution battery.
