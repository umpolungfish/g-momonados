#!/usr/bin/env bash
# Select an explicit operation order before the wrapper's original argument parser.
for ig_order_arg in "$@"; do
    if [[ "$ig_order_arg" == --ordering || "$ig_order_arg" == --orderings ]]; then
        exec python3 "$(dirname -- "${BASH_SOURCE[0]}")/../../Vox/ordering.py" "$0" "$@"
    fi
done
set -euo pipefail
project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$project_root"
number=${1:?usage: bash tools/bake_recycled_phase.sh N [base=2] [shots=12]}
base=${2:-2}
shots=${3:-12}
[[ "$number" =~ ^[0-9]+$ && "$base" =~ ^[0-9]+$ && "$shots" =~ ^[0-9]+$ ]]
minimum=170141183460469231731687303715884105728
normalized_number="$number"
while [[ ${#normalized_number} -gt 1 && "$normalized_number" == 0* ]]; do
    normalized_number=${normalized_number#0}
done
if [[ ${#normalized_number} -lt ${#minimum} ]] || \
   { [[ ${#normalized_number} -eq ${#minimum} ]] && [[ "$normalized_number" < "$minimum" ]]; }; then
    printf 'Factoring requires a semiprime of at least 128 bits.\n' >&2
    exit 1
fi
destination=$(python3 ../Vox/ordering.py --variant-path "measurements/baked-qpe/$number")
mkdir -p "$destination"
if [[ -e "$destination/phase_unbraid" ]]; then
    printf 'Baked executable already exists: %s\n' "$destination/phase_unbraid" >&2
    exit 1
fi
source_word=$("$project_root/../Vox/target/x86_64-unknown-linux-gnu/release/vox" numeral "$number")
printf '%s\n' "$source_word" > "$destination/source.imasm"
PHASE_UNBRAID_BAKED_EXECUTE=1 FACTOR_PHASE_SOURCE_WORD="$source_word" \
PHASE_UNBRAID_BAKED_BASE="$base" PHASE_UNBRAID_BAKED_SHOTS="$shots" \
cargo build --offline --release --target-dir target --target x86_64-unknown-linux-gnu --bin g-momonados
cp target/x86_64-unknown-linux-gnu/release/g-momonados "$destination/phase_unbraid"
strip --strip-debug "$destination/phase_unbraid"
sha256sum "$destination/phase_unbraid" > "$destination/sha256.txt"
printf 'Baked executable: %s/%s/phase_unbraid\n' "$project_root" "$destination"
