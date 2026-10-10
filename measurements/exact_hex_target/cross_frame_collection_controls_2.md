# Retained folds and cross-frame collection

Command: `python3 measurements/exact_hex_target/cross_frame_collection.py --controls-only`.

Complete positioned words, original and collected folds, component paths, and every overlay are in `cross_frame_collection_controls_2.records.jsonl.gz`. Native checks are in `cross_frame_collection_controls_2.native.log.gz`.

```json
{
  "stats": {
    "root_folds": 36,
    "collected_nonzero_folds": 8,
    "component_words": 47,
    "component_fold_tests": 365,
    "component_products": 46,
    "fold_joins": 70,
    "overlay_pairs": 46,
    "overlay_fragments": 221,
    "overlay_joins": 72
  },
  "proper_source_products": 4,
  "elapsed_seconds": 0.245,
  "factor_extraction_evidence": "True"
}
```
