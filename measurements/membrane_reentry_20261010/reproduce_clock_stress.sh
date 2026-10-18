#!/usr/bin/env bash
set -euo pipefail
task_repo=$(cd "$(dirname "$0")/../.." && pwd)
task_root=$(dirname "$task_repo")
task_records="$task_repo/measurements/membrane_reentry_20261010"
exec 9>"$task_records/stress.lock"
flock -n 9 || { echo "A clock stress run is already active." >&2; exit 1; }
for task_process in lake lean cargo rustc; do
    if pgrep -x "$task_process" >/dev/null; then
        echo "Finish the existing $task_process job before this serial run." >&2
        exit 1
    fi
done
if pgrep -f '^(.*/)?ququart_(factor|work_probe)_baked( |$)' >/dev/null; then
    echo "Finish the existing ququart membrane before this serial run." >&2
    exit 1
fi
cd "$task_root/p4rakernel/p4ramill"
lake env lean -j1 Imscribing/Paraconsistent/QuquartMembraneReentry.lean \
    -o .lake/build/lib/lean/Imscribing/Paraconsistent/QuquartMembraneReentry.olean \
    > ../measurements/membrane_reentry_20261010/clock_binding_build.log 2>&1
lake env lean -j1 Imscribing/Paraconsistent/QuquartMembraneReentryAudit.lean \
    > ../measurements/membrane_reentry_20261010/clock_binding_audit.log 2>&1
sed -n 's/^QUQUART_CLOCKS=//p' ../measurements/membrane_reentry_20261010/clock_binding_audit.log \
    > "$task_records/certified_clocks.json"
cd "$task_repo"
cargo build -j1 --bin sic-tool --bin qpe_semiprime_cases --bin ququart_prepare_operator \
    --bin ququart_work_probe_baked > "$task_records/clock_runtime_build.log" 2>&1
mkdir -p "$task_records/stress"
rustc --edition=2021 --crate-type=lib --emit=obj -C opt-level=2 -C panic=abort \
    src/ququart_reentry_clock.rs -o "$task_records/stress/ququart_clock.o"
cc -shared -nostdlib -o "$task_records/stress/ququart_clock.so" "$task_records/stress/ququart_clock.o"
node "$task_records/stress_probe.mjs" > "$task_records/stress_probe.log" 2>&1
node "$task_records/stress_probe.mjs" --controls > "$task_records/clock_controls.log" 2>&1
taskset -c 0 target/debug/qpe_semiprime_cases --verify "$task_records/balanced_wide.tsv.proofs.jsonl" \
    > "$task_records/balanced_wide_verification.stdout" 2> "$task_records/balanced_wide_verification.stderr"
node "$task_records/stress_probe.mjs" --wide > "$task_records/stress_wide_probe.log" 2>&1
