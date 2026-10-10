#!/usr/bin/env bash
set -euo pipefail
task_repo=$(cd "$(dirname "$0")/../.." && pwd)
cd "$task_repo"
task_records=measurements/membrane_reentry_20261010
for task_process in lake lean cargo rustc ququart_baked ququart_factor_baked; do
    if pgrep -x "$task_process" >/dev/null; then
        echo "An existing $task_process job is active; finish it before this execution." >&2
        exit 1
    fi
done
cargo build -j1 --bin sic-tool --bin ququart_verify_readout --bin ququart_factor_baked --bin vox_disasm_region \
    > "$task_records/reconstruction_build.log" 2>&1
target/debug/sic-tool anyon-program < "$task_records/coherent_fourier_program.json" \
    > "$task_records/coherent_fourier_program.out.json" 2> "$task_records/coherent_fourier_program.stderr"
jq '{source:.source,gram:(.events[]|select(.kind=="rejoin")|.gram),masses:(.events[]|select(.kind=="rejoin")|.masses)}' \
    "$task_records/coherent_fourier_program.out.json" > "$task_records/coherent_fourier_certificate_input.json"
target/debug/sic-tool gram-reconstruct < "$task_records/coherent_fourier_certificate_input.json" \
    > "$task_records/coherent_fourier_certificate.out.json" 2> "$task_records/coherent_fourier_certificate.stderr"
jq '.gram |= map([.[0], (.[1]|if startswith("-") then .[1:] elif .=="0" then . else "-"+. end)])' \
    "$task_records/coherent_fourier_certificate_input.json" > "$task_records/conjugated_coherence_control.json"
task_control_status=0
target/debug/sic-tool gram-reconstruct < "$task_records/conjugated_coherence_control.json" \
    > "$task_records/conjugated_coherence_control.stdout" 2> "$task_records/conjugated_coherence_control.stderr" \
    || task_control_status=$?
if [[ "$task_control_status" != 2 ]] || ! rg -qx 'sic-tool: SIC masses differ from the supplied control Gram' \
    "$task_records/conjugated_coherence_control.stderr"; then
    echo "Unexpected coherence-control outcome: $task_control_status" >&2
    exit 1
fi
nm -S -C --defined-only target/debug/sic-tool | rg 'FixedQuquartSic>::certify_gram_frame$' \
    > "$task_records/reconstruction_symbols.txt"
read -r task_start task_size _ < "$task_records/reconstruction_symbols.txt"
task_end=$(printf '%x' "$((16#$task_start + 16#$task_size))")
target/debug/vox_disasm_region target/debug/sic-tool "$task_start" "$task_end" \
    > "$task_records/reconstruction_vox_disassembly.tsv" 2> "$task_records/reconstruction_vox_disassembly.stderr"
jq -n --argjson control_exit "$task_control_status" \
    '{build_exit:0,carrier_exit:0,certificate_exit:0,conjugated_control_exit:$control_exit,vox_decode_exit:0}' \
    > "$task_records/execution_status.json"
sha256sum target/debug/sic-tool target/debug/ququart_verify_readout target/debug/ququart_factor_baked \
    > "$task_records/executable_hashes.sha256"
jq '{outcome_kernel_masks,source_return,rejoin:[.events[]|select(.kind=="rejoin")|{gram_dual_verified,reconstruction_residual,reconstruction_tolerance}]}' \
    "$task_records/coherent_fourier_program.out.json"
