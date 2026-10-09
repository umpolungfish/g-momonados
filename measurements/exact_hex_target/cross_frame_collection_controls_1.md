# Retained folds and cross-frame collection

All arithmetic and classification controls completed. This first control run
then encountered a duplicate `elapsed_seconds` keyword while printing its final
status. The status formatter was corrected before the full target run, which
repeated the controls and completed successfully. These original control records
remain preserved.

Command: `python3 measurements/exact_hex_target/cross_frame_collection.py --controls-only`.

Complete positioned words, original and collected folds, component paths, and every overlay are in `cross_frame_collection_controls_1.records.jsonl.gz`. Native checks are in `cross_frame_collection_controls_1.native.log.gz`.

```json
{
  "stats": {
    "root_folds": 36,
    "collected_nonzero_folds": 8,
    "component_words": 46,
    "component_fold_tests": 356,
    "component_products": 44,
    "fold_joins": 65,
    "overlay_pairs": 46,
    "overlay_fragments": 221,
    "overlay_joins": 72
  },
  "proper_source_products": 4,
  "elapsed_seconds": 0.251,
  "factor_extraction_evidence": "True"
}
```
