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
# An inherited Cargo build.target writes beneath target/<triple>/, not target/.
# Bind the build and invocation to the same explicit host target.
GM_TARGET="${GM_TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
BIN="target/${GM_TARGET}/${PROFILE}/g-momonados"
JOIN_AWK="$(dirname "$0")/join_digit_continuations.awk"

PROFILE_FLAG=()
case "$PROFILE" in
  release) PROFILE_FLAG=(--release) ;;
  debug) ;;
  *) echo "PROFILE must be release or debug" >&2; exit 2 ;;
esac
# Incremental Cargo freshness checks prevent an existing stale executable from
# silently surviving source repairs.
cargo build --offline --quiet --target-dir target --target "$GM_TARGET" --bin g-momonados "${PROFILE_FLAG[@]}" >&2


{
  for c in "$@"; do
    printf '%s\n' "$c" | awk -f "$JOIN_AWK"
  done
  printf 'quit\n'
} | "$BIN"
