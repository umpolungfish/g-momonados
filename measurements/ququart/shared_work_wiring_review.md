Vox wiring investigation

The executable IMASM module, per-function structure words, full native disassembly, and complete glyph encoding were produced from the actual shared-work membrane. Glyph decoding reproduced the executable module byte for byte. The full module and glyph payload remain local large artifacts; the pipeline manifest records their paths and sizes.

The default CFG words are incomplete for indirect dispatch. The arithmetic function has a 23-letter structure word and N verdict with no paired regions, while its complete symbol contains 1155 decoded instructions. The phase-shot structure word is absent from the selected default export, while the bounded decoder recovers 3037 instructions. The complete executable IMASM excerpt and bounded disassembly therefore supply the wiring evidence. N or B from these partial structure words is not an arithmetic verdict and cannot establish that the factor computation is correct or incorrect.

The source wiring uses COMPUTATIONAL_CHANNELS = [0,3,1,4] in src/anyon_pair.rs. Source inspection gives the following intended map:

| Logical digit | Physical pair channel | Enabled multiplier rails |
| --- | --- | --- |
| zero | first computational channel | neither |
| one | second computational channel | lane zero, multiplier a^power |
| two | third computational channel | lane one, squared multiplier |
| three | fourth computational channel | both rails |

Each Fourier row/column uses the physical pair-channel matrix. Controlled arithmetic and measurement instead enumerate the logical digits through the same channel table. Feedback also uses that logical digit as its phase weight. This is a source-level consistency check of the labeling only. It does not verify the operator's action on all amplitudes, the wire permutation, modular inverse cleanup, measured phase, or factors. The next trace must follow those numeric operations through the complete IMASM regions rather than using the omitted word arms as evidence.

The interrupted shared-work execution reached the enforced cutoff with no report. No factor-bearing result is present. No new factor execution, test suite or profiling was launched for this Vox investigation.
