#!/usr/bin/env bash
# Select an explicit operation order before the wrapper's original argument parser.
for ig_order_arg in "$@"; do
    if [[ "$ig_order_arg" == --ordering || "$ig_order_arg" == --orderings ]]; then
        exec python3 "$(dirname -- "${BASH_SOURCE[0]}")/../Vox/ordering.py" "$0" "$@"
    fi
done
# Bake an N-only IMASM source into a resident membrane executable.
# Execution is a separate step - the binary runs with no arguments.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../Vox/scripts/static_membrane.sh"

usage() {
    printf 'usage: %s DECIMAL_N [OUTPUT_DIRECTORY]\n' "$0"
}
if [[ ${1:-} == --help || ${1:-} == -h ]]; then usage; exit 0; fi
[[ $# -ge 1 && $# -le 2 ]] || { usage >&2; exit 2; }

bake_n=$1
bake_output=${2:-}

[[ $bake_n =~ ^[1-9][0-9]*$ && $bake_n != 1 ]] || {
    printf 'N must be a canonical decimal integer greater than one.\n' >&2; exit 2;
}

bake_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cd -- "$bake_root"

if [[ -n $bake_output && -n ${IG_ORDER_SITE:-} ]]; then
    bake_order_key=$(printf '%s\n' "$IG_ORDER_SITE" "${IG_ORDER_SPEC:-canonical}" | sha256sum | cut -c1-16)
    bake_output="${bake_output}.order-${bake_order_key}"
fi
if [[ -z $bake_output ]]; then
    mkdir -p measurements/baked_resident
    bake_output=$(mktemp -d "$bake_root/measurements/baked_resident/factor.XXXXXXXX")
else
    [[ $bake_output == /* ]] || bake_output="$bake_root/$bake_output"
    mkdir -p -- "$(dirname -- "$bake_output")"
    mkdir -- "$bake_output"
fi
bake_output=$(cd -- "$bake_output" && pwd)
printf 'site=%s\nspec=%s\n' "${IG_ORDER_SITE:-}" "${IG_ORDER_SPEC:-canonical}" >"$bake_output/ordering.txt"

bake_fail() {
    printf 'Bake failed; build diagnostics are in %s\n' "$bake_output" >&2
}
trap bake_fail ERR

# Build vox first (needed for numeral conversion)
cd /home/mrnob0dy666/imsgct/Vox
RUSTFLAGS="${RUSTFLAGS:-} -D warnings -C target-feature=+crt-static" cargo build --target-dir target --release --target x86_64-unknown-linux-gnu --bin vox \
    >"$bake_output/vox_build.stdout" 2>"$bake_output/vox_build.stderr"
cd -- "$bake_root"

# Convert N to IMASM
/home/mrnob0dy666/imsgct/Vox/target/x86_64-unknown-linux-gnu/release/vox numeral "$bake_n" >"$bake_output/input.imasm"
bake_word=$(<"$bake_output/input.imasm")

# Generate the split/fuse circuit receipt consumed by resident_build.rs.
cargo build --target-dir target --release --target x86_64-unknown-linux-gnu --bin g-momonados \
    >"$bake_output/operator_build.stdout" 2>"$bake_output/operator_build.stderr"
./target/x86_64-unknown-linux-gnu/release/g-momonados qc HTSX 3 1 \
    >"$bake_output/operator.stdout" 2>"$bake_output/operator.stderr"
awk '/^BRAID_OPERATOR_JSON=/{sub(/^BRAID_OPERATOR_JSON=/, ""); print}' \
    "$bake_output/operator.stdout" >"$bake_output/operator_receipt.json"
[[ -s "$bake_output/operator_receipt.json" ]] || { printf 'Missing circuit operator receipt.\n' >&2; exit 1; }

# Build the resident membrane binary with the IMASM word and operator receipt baked in
RESIDENT_SOURCE_FILE="$bake_output/input.imasm" \
RESIDENT_CIRCUIT_OPERATOR_FILE="$bake_output/operator_receipt.json" \
RUSTFLAGS="${RUSTFLAGS:-} -D warnings -C target-feature=+crt-static" \
cargo build --target-dir target --release --target x86_64-unknown-linux-musl --bin resident_factor_baked \
    >"$bake_output/build.stdout" 2>"$bake_output/build.stderr"

require_static_membrane target/x86_64-unknown-linux-musl/release/resident_factor_baked
cp -- target/x86_64-unknown-linux-musl/release/resident_factor_baked "$bake_output/resident_factor_baked"
printf '%s\n' "$bake_n" >"$bake_output/input.decimal"
sha256sum "$bake_output/resident_factor_baked" >"$bake_output/resident_factor_baked.sha256"
printf '%s\n' "$bake_output/resident_factor_baked"
