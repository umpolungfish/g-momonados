# Retained folds and cross-frame collection

The source is the complete 895-cell word in `source.txt`. At every width 1–894,
the instrument retains the original equation with placement `2^w-epsilon` and
its signed frame sum, then folds that correction until the remainder is in the
placement's canonical range. Both original and collected equations pass native
Gödel multiplication and addition. The source's canonical frames retain their
start, actual end and payload; high padding does not replace actual length.

The 6,108 distinct operand and correction words include zero and one. The other
6,106 receive a complete pass over their own proper widths for shared occupied
frames and both fold identities, including nonzero imbalance collection. Every
registered component product passes native exact-product, codec and independent
support-carry checks. This is one structural layer over the root components.

Every pair of root widths is overlaid by the union of its boundaries. The
5,918,276 fragments reconstruct their positioned source exactly. Identical
fragment payloads collect under their original starts; returned components of
either operand and of the whole positioned term remain available for collection
across the complete sum. All 893 nonempty whole-source common-component matches
are the original source paired with the unit, from overlays involving width one.
The proper-source product list is empty.

The only zero collected fold correction uses width one and placement one.
Four other folds have single-cell canonical remainders: placements 3 and 9
leave 1, and placement 5 leaves 2. Their odd placements therefore share only
the unit with the source. Other nonzero corrections retain their full words.
The original alternating folds include 202 negative correction words.

Three signed-correction classes connect different widths. They feed the separate
`cross_frame_product_transport.py` continuation, which retains the alternative
products and tests the observed cross-width component/quotient relationships.
This run uses source-origin frames; arbitrary starting-offset sweeps and deeper
closure under every notebook operation are outside this pass.

The native log contains one deliberate wrong-source product control. Its
exact-product and support-carry fields fail, while the corresponding proper and
unit controls exercise their separate classifications. Every target fold and
registered component certificate passes.

Command: `python3 measurements/exact_hex_target/cross_frame_collection.py`.

Complete positioned words, original and collected folds, component paths, and every overlay are in `cross_frame_collection_1.records.jsonl.gz`. Native checks are in `cross_frame_collection_1.native.log.gz`.

```json
{
  "stats": {
    "root_folds": 1788,
    "collected_nonzero_folds": 1,
    "component_words": 6106,
    "component_fold_tests": 5657640,
    "component_products": 14516,
    "fold_joins": 0,
    "overlay_pairs": 399171,
    "overlay_fragments": 5918276,
    "overlay_joins": 893
  },
  "proper_source_products": 0,
  "elapsed_seconds": 667.799,
  "factor_extraction_evidence": "None"
}
```
