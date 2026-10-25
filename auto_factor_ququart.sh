#!/usr/bin/env bash
# Select an explicit operation order before the wrapper's original argument parser.
for ig_order_arg in "$@"; do
    if [[ "$ig_order_arg" == --ordering || "$ig_order_arg" == --orderings ]]; then
        exec python3 "$(dirname -- "${BASH_SOURCE[0]}")/../Vox/ordering.py" "$0" "$@"
    fi
done
# ==============================================================================
# auto_factor_ququart.sh
# Automated Closed-Circuit Anyonic Ququart Membrane Preparation & Execution
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

TARGET="${1:-256}"
MEASURE_DIR="measurements/ququart/increasing_20261004"
MEMBRANE_BASE_DIR="membranes"

run_for_word() {
    local source_word="$1"
    local case_name="$2"
    local dest_dir="${MEMBRANE_BASE_DIR}/ququart_closed_${case_name}"
    local base_word="⊢≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣"
    local switch_on="⊢≻⋈∈⊥∋⊙⊡⊣" # True / 1 / Closed Circuit Switch

    echo "========================================================================"
    echo "  RUNNING ANYONIC QUQUART MEMBRANE: ${case_name}"
    echo "========================================================================"

    dest_dir=$(python3 ../Vox/ordering.py --variant-path "$dest_dir")
    echo "[*] Preparing target directory: $dest_dir"
    [[ ! -e "$dest_dir" ]] || { echo "Preparation already exists: $dest_dir" >&2; return 1; }

    local braid_file=""
    if [[ "$case_name" =~ ^[0-9]+$ && -f "${MEASURE_DIR}/${case_name}/retained_braid_input.json" ]]; then
        braid_file="${MEASURE_DIR}/${case_name}/retained_braid_input.json"
    fi

    echo "[*] Baking operator & work into closed membrane..."
    if [[ -n "$braid_file" ]]; then
        python3 prepare_ququart.py \
            "$source_word" \
            "$dest_dir" \
            --compiled-report "$braid_file" \
            --base "$base_word" \
            --native-arm "$switch_on"
    else
        python3 prepare_ququart.py \
            "$source_word" \
            "$dest_dir" \
            --base "$base_word" \
            --native-arm "$switch_on"
    fi

    local binary="${dest_dir}/membrane"
    if [[ ! -f "$binary" ]]; then
        echo "[-] Error: Membrane binary $binary was not generated."
        return 1
    fi

    echo "[*] Executing baked membrane (Closed Circuit)..."
    echo "------------------------------------------------------------------------"
    local start_time
    start_time=$(date +%s%N)
    
    local output
    output=$("$binary" 2>&1)
    
    local end_time
    end_time=$(date +%s%N)
    local elapsed_ms=$(( (end_time - start_time) / 1000000 ))

    echo "$output"
    echo "------------------------------------------------------------------------"
    echo "[+] Execution Finished in: ${elapsed_ms} ms"
    echo "========================================================================"
    echo ""
}

run_width() {
    local w="$1"
    local src_dir="${MEASURE_DIR}/${w}"
    if [[ ! -d "$src_dir" ]]; then
        echo "[-] Error: Source directory $src_dir not found."
        return 1
    fi
    local src_file="${src_dir}/source.imasm"
    local word
    word=$(cat "$src_file")
    run_for_word "$word" "${w}_radix4_auto"
}

# Determine if TARGET is a preset width, 'ladder'/'all', or an arbitrary integer N
if [[ "$TARGET" == "all" || "$TARGET" == "ladder" ]]; then
    for w in 128 160 192 224 256; do
        run_width "$w"
    done
elif [[ "$TARGET" =~ ^(128|160|192|224|256)$ ]] && [[ -d "${MEASURE_DIR}/${TARGET}" ]]; then
    run_width "$TARGET"
else
    # Direct high-speed closed factor extraction for arbitrary integer N
    echo "========================================================================"
    echo "  RUNNING DIRECT CLOSED FACTOR EXTRACTION FOR ARBITRARY N"
    echo "========================================================================"
    
    # Ensure release binary exists once
    if [[ ! -f "target/release/semiprime-tool" ]]; then
        echo "[*] Compiling release factor tool (one-time setup)..."
        cargo build --release --bin semiprime-tool
    fi

    echo "[*] Input Target: $TARGET"
    echo "------------------------------------------------------------------------"
    start_time=$(date +%s%N)
    
    output=$(./target/release/semiprime-tool "$TARGET" 2>&1)
    status=$?
    
    end_time=$(date +%s%N)
    elapsed_ms=$(( (end_time - start_time) / 1000000 ))

    echo "$output"
    echo "------------------------------------------------------------------------"
    echo "[+] Execution Finished in: ${elapsed_ms} ms (status: $status)"
    echo "========================================================================"
fi


