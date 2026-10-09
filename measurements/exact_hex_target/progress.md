# Exact source factor goal

The complete source is in `source.txt`. Its native Gödel encoding has 895 cells; its ordered hexadecimal spelling has 224 complete motifs. Completion requires proper factors whose exact product equals this source, produced through its Gödel and `tfactor read` relationships. No proper factors have been produced in this record.

Verified on this source:

- Ordered hex composition reconstructs the exact native source word.
- Full motif nonrepetition selects the unequal 448-cell block relationship. Its output/remainder square correction fails the exact square tests.
- The translated lane payload is 3 with remainder 1; the shared component is the unit.
- The full upper-frame square anchor needs two successive prefix transports, at target cells 5 and 7. The second repair enables the complete finite root test. Neither root is an exact square root. The returned signed source residuals have initial empty runs of 454 and 455 cells.
- Each of the two original operand pairs was compared with its own signed residual using instrument-produced division/remainder operations. All four shared components are the unit.
- All six exchanges of truth, falsity, information and fork slots were checked. All six source/placement shared components and all six output/remainder shared components are the unit. These exact exchanges are exhausted for this source.

- Carry-preserving cyclic folds at complete-motif boundaries of 448, 224 and 112 cells agree exactly with the full source remainders for both boundary polarities. All six shared components are the unit.
- The source-selected sums `2^w + upper_frame` at these three boundaries all fail the discriminant square test at native cell 4, truth position in its hex motif. Each sum is 4 modulo 8; the odd factor pair of this source, which is 7 modulo 8, requires sum 0 modulo 8. The full sum word can therefore retain its upper cells and clear its low three cells. All three full tests completed and retained two roots each. The six signed source-product equations are independently certified in `frame_sum_returns.md`; their nonzero corrections begin at cells 449, 453, 677, 672, 784 and 786 respectively. Each correction is certified as a positioned power-of-two word times its remaining payload; `tfactor read` records the payload hex compositions in `frame_sum_returns.log.gz`.

- The six full-frame correction payloads have been tested against each exposed operand. Only the first return contains complete operand copies: two copies of its left operand. Collecting those copies changes the cofactor and reduces the correction; all twelve collected source equations pass independent Gödel addition/multiplication checks. Every retained remainder is nonzero. The full equations and hex readings are in `frame_residual_collection.md` and `frame_residual_collection.log.gz`.

- The complete-word decoder control returns the exact source from its native word and rejects its complete hex word as an unregistered numeral family. The hex/native round trip in the ordered-word probe reconstructs cells from motif presence before decoding. The source constructor and this control are recorded in the working notebook and `hex_decoder_control.log`.

Implementation corrections: bounded source labels prevent filesystem filename overflow; repeated prefix transport follows each newly exposed failure; division/remainder transport avoids the slower subtraction certificate. Python spells and indexes words and orchestrates calls; source arithmetic remains in `godel`. Full logs are stored losslessly as gzip archives and can be loaded through `read_record`.

The source goal remains active. These checks are research results, not a factor certificate.
