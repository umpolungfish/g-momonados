#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

if [[ "${1:-}" == "--help" || "$#" -lt 1 || "$#" -gt 5 ]]; then
  echo 'usage: ./vox_phase_factor_build.sh <odd-semiprime-decimal> [phase-base-decimal] [output-membrane.glyphs] [sweep|single] [sparse|every]'
  exit 0
fi

N_DECIMAL="$1"
BASE_DECIMAL="${2:-2}"
OUTPUT_MODULE="${3:-membranes/vox_phase_factor_$(date +%s).glyphs}"
SEED_MODE="${4:-sweep}"
SUPPORT_MODE="${5:-every}"
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
sed -e "s|@@N_WORD@@|$N_WORD|g" \
    -e "s|@@BASE_WORD@@|$BASE_WORD|g" \
    vox_phase_factor.c > "$TEMP_DIR/phase_factor.c"

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
