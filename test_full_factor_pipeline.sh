#!/usr/bin/env bash
# Feed only N to the production extractor; fixture factors are output oracles.
set -u
cd -- "$(dirname -- "$0")"
limit=${FACTOR_TEST_SECONDS:-60}
out=measurements/full-factor-pipeline
mkdir -p "$out"
while IFS=$'\t' read -r case_id width source expected_p expected_q; do
    [[ "$width" == bits ]] && continue
    (
        /usr/bin/time -f 'elapsed_seconds=%e' timeout --kill-after=5s "${limit}s" \
            target/release/semiprime-tool "$source" \
            > "$out/$width.log" 2> "$out/$width.timing.log"
        result=$?
        if [[ "$result" == 0 ]]; then
            pair=$(awk '$1 == "factor.pair" {print $2 " " $4}' "$out/$width.log")
            if [[ "$pair" != "$expected_p $expected_q" && "$pair" != "$expected_q $expected_p" ]]; then
                result=3
            fi
            if ! rg -q 'product-closure +closed' "$out/$width.log"; then
                result=3
            fi
        fi
        printf '%s\t%s\n' "$width" "$result" > "$out/$width.status.tsv"
    ) &
done < measurements/anyon-extractor-width-controls.tsv
wait
failed=0
for width in 128 256 512 1024 2048; do
    cat "$out/$width.status.tsv"
    result=$(cut -f2 "$out/$width.status.tsv")
    [[ "$result" == 0 ]] || failed=1
done
exit "$failed"
