#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

if [[ "${1:-}" == "--help" || "$#" -lt 1 || "$#" -gt 5 ]]; then
  echo 'usage: ./imasm_phase_factor_build.sh <odd-semiprime-decimal> [phase-base-decimal] [output-binary] [sweep|single] [sparse|every]'
  exit 0
fi

IMASM_N_DECIMAL="$1"
IMASM_BASE_DECIMAL="${2:-2}"
IMASM_SEED_MODE="${4:-sweep}"
IMASM_SUPPORT_MODE="${5:-sparse}"
[[ "$IMASM_N_DECIMAL" =~ ^[0-9]+$ && "$IMASM_BASE_DECIMAL" =~ ^[0-9]+$ ]] || {
  echo 'N and phase base must be decimal integers' >&2
  exit 2
}
[[ "$IMASM_SEED_MODE" == sweep || "$IMASM_SEED_MODE" == single ]] || {
  echo 'seed mode must be sweep or single' >&2
  exit 2
}
[[ "$IMASM_SUPPORT_MODE" == sparse || "$IMASM_SUPPORT_MODE" == every ]] || {
  echo 'support mode must be sparse or every' >&2
  exit 2
}

RUSTFLAGS='-D warnings' cargo build --release --bin godel >/dev/null
IMASM_N_WORD="$(./target/release/godel encode "$IMASM_N_DECIMAL" | awk '$1 == "word" {print $2}')"
IMASM_BASE_WORD="$(./target/release/godel encode "$IMASM_BASE_DECIMAL" | awk '$1 == "word" {print $2}')"
[[ -n "$IMASM_N_WORD" && -n "$IMASM_BASE_WORD" ]] || {
  echo 'Gödel codec returned no encoded numeral word' >&2
  exit 2
}

IMASM_FINGERPRINT="$(printf '%s\n%s\n' "$IMASM_N_WORD" "$IMASM_BASE_WORD" | sha256sum | cut -c1-20)"
IMASM_OUTPUT="${3:-membranes/imasm_phase_factor_${IMASM_FINGERPRINT}}"
if [[ -e "$IMASM_OUTPUT" ]]; then
  echo "output already exists: $IMASM_OUTPUT" >&2
  exit 2
fi
IMASM_PHASE_N_WORD="$IMASM_N_WORD" IMASM_PHASE_BASE_WORD="$IMASM_BASE_WORD" \
  IMASM_PHASE_SINGLE_BASE="$([[ "$IMASM_SEED_MODE" == single ]] && echo 1 || echo 0)" \
  IMASM_PHASE_SUPPORT_EVERY="$([[ "$IMASM_SUPPORT_MODE" == every ]] && echo 1 || echo 0)" \
  RUSTFLAGS='-D warnings' cargo build --release --bin imasm_phase_factor >/dev/null
mkdir -p "$(dirname "$IMASM_OUTPUT")"
cp target/release/imasm_phase_factor "$IMASM_OUTPUT"
strip --strip-debug "$IMASM_OUTPUT"
chmod +x "$IMASM_OUTPUT"
printf '%s\n' "$IMASM_OUTPUT"
