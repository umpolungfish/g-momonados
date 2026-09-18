#!/usr/bin/env bash
# run_cmds.sh — feed REPL commands to the G-mOMonadOS binary, non-interactive.
#
# Usage: ./run_cmds.sh "sic d16" "weight ⊢∈⊤⊡⊣" ...
#
# Wrapped big integers inside one argv are rejoined by
# join_digit_continuations.awk — same hold rule as run_quit.sh and the REPL.
# Each argv is joined alone so holds never cross commands.
set -euo pipefail
cd "$(dirname "$0")"

PROFILE="${PROFILE:-release}"
BIN="target/${PROFILE}/g-momonados"
JOIN_AWK="$(dirname "$0")/join_digit_continuations.awk"

if [ ! -x "$BIN" ]; then
  PROFILE_FLAG=()
  [ "$PROFILE" = "release" ] && PROFILE_FLAG=(--release)
  cargo build "${PROFILE_FLAG[@]}" >&2
fi

{
  for c in "$@"; do
    printf '%s\n' "$c" | awk -f "$JOIN_AWK"
  done
  printf 'quit\n'
} | "$BIN"
