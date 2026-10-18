#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

if [[ "${1:-}" == "--help" || "$#" -lt 1 || "$#" -gt 6 ]]; then
  echo 'usage: ./vox_phase_factor_build.sh <odd-semiprime-decimal> [phase-base-decimal] [output-membrane.glyphs] [sweep|single] [sparse|every] [nested-extract-depth]'
  exit 0
fi

N_DECIMAL="$1"
BASE_DECIMAL="${2:-2}"
OUTPUT_MODULE="${3:-membranes/vox_phase_factor_$(date +%s).glyphs}"
SEED_MODE="${4:-sweep}"
SUPPORT_MODE="${5:-every}"
NEST_DEPTH_DECIMAL="${6:-1}"
VOX_BIN="${VOX_BIN:-/home/mrnob0dy666/imsgct/Vox/target/release/vox}"

[[ "$N_DECIMAL" =~ ^[0-9]+$ && "$BASE_DECIMAL" =~ ^[0-9]+$ ]] || {
  echo 'N and phase base must be decimal integers' >&2
  exit 2
}
[[ "$SEED_MODE" == sweep || "$SEED_MODE" == single ]] || {
  echo 'seed mode must be sweep or single' >&2
  exit 2
}
[[ "$SUPPORT_MODE" == sparse || "$SUPPORT_MODE" == every ]] || {
  echo 'support mode must be sparse or every' >&2
  exit 2
}
[[ "$NEST_DEPTH_DECIMAL" =~ ^[0-9]+$ ]] || {
  echo 'nested extraction depth must be a positive decimal integer' >&2
  exit 2
}
[[ "$NEST_DEPTH_DECIMAL" =~ [1-9] ]] || {
  echo 'nested extraction depth must be positive' >&2
  exit 2
}

append_nested_marks() {
  local mark="$1"
  local remaining="$NEST_DEPTH_DECIMAL"
  local position
  while [[ "$remaining" =~ [1-9] ]]; do
    EXTRACT_WORD+="$mark"
    local result=''
    local borrow=1
    local digit
    for ((position = ${#remaining} - 1; position >= 0; --position)); do
      digit="${remaining:position:1}"
      if (( borrow )); then
        if [[ "$digit" == 0 ]]; then
          digit=9
        else
          digit=$((digit - 1))
          borrow=0
        fi
      fi
      result="${digit}${result}"
    done
    remaining="$result"
  done
}
[[ -x "$VOX_BIN" ]] || { echo "V⊙x executable not found: $VOX_BIN" >&2; exit 2; }

encode_word() {
  local decimal="$1"
  local binary
  binary="$(bc <<< "obase=2; $decimal" | tr -d '\\\n' | rev)"
  local encoded='⊢'
  local bit
  for ((index = 0; index < ${#binary}; ++index)); do
    bit="${binary:index:1}"
    if [[ "$bit" == 1 ]]; then
      encoded+='≻⋈∈⊥∋'
    else
      encoded+='≻⋈∈⊤∋'
    fi
  done
  encoded+='⊙⊡⊣'
  printf '%s' "$encoded"
}

N_BINARY="$(bc <<< "obase=2; $N_DECIMAL" | tr -d '\\\n')"
BASE_BINARY="$(bc <<< "obase=2; $BASE_DECIMAL" | tr -d '\\\n')"
[[ -n "$N_BINARY" && "${N_BINARY: -1}" == 1 ]] || {
  echo 'N must be positive and odd' >&2
  exit 2
}
MODULUS_BITS=${#N_BINARY}
N_LIMBS=$(( (MODULUS_BITS + 63) / 64 ))
BASE_LIMBS=$(( (${#BASE_BINARY} + 63) / 64 ))
LIMBS=$N_LIMBS
(( BASE_LIMBS > LIMBS )) && LIMBS=$BASE_LIMBS
N_WORD="$(encode_word "$N_DECIMAL")"
BASE_WORD="$(encode_word "$BASE_DECIMAL")"
EXTRACT_WORD='⊢'
append_nested_marks '∈'
EXTRACT_WORD+='≻⊤≺⊥⊞⋈'
append_nested_marks '∋'
EXTRACT_WORD+='⊙⊡⊣'
SINGLE_BASE=0
SUPPORT_EVERY_PHASE=0
[[ "$SEED_MODE" == single ]] && SINGLE_BASE=1
[[ "$SUPPORT_MODE" == every ]] && SUPPORT_EVERY_PHASE=1

if [[ -e "$OUTPUT_MODULE" ]]; then
  echo "output already exists: $OUTPUT_MODULE" >&2
  exit 2
fi
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT
printf 'static const char baked_n[] = "%s";\nstatic const char baked_base_word[] = "%s";\nstatic const char baked_extract_word[] = "%s";\n' \
  "$N_WORD" "$BASE_WORD" "$EXTRACT_WORD" > "$TEMP_DIR/baked_inputs.h"
cp vox_phase_factor.c "$TEMP_DIR/phase_factor.c"

cc -O2 -std=c11 -Wall -Wextra -Werror \
  -fno-pie -no-pie -nostdlib -static -fno-stack-protector -fno-builtin \
  -DLIMBS="$LIMBS" -DMODULUS_BITS="$MODULUS_BITS" \
  -DSINGLE_BASE="$SINGLE_BASE" \
  -DSUPPORT_EVERY_PHASE="$SUPPORT_EVERY_PHASE" \
  -Wl,-e,_start -Wl,--build-id=none \
  "$TEMP_DIR/phase_factor.c" -o "$TEMP_DIR/phase_factor.elf"

"$VOX_BIN" imasm "$TEMP_DIR/phase_factor.elf" >/dev/null 2>&1
mkdir -p "$(dirname "$OUTPUT_MODULE")"
"$VOX_BIN" glyphs "$TEMP_DIR/phase_factor.elf.imasm" "$OUTPUT_MODULE" >/dev/null 2>&1
printf '%s\n' "$OUTPUT_MODULE"
