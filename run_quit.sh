#!/usr/bin/env bash
# run_quit.sh — non-interactive G-mOMonadOS: feed commands, quit, print only results.
#
# Boot banner, ⊙> prompt echoes, Halting/SHUTDOWN lines are stripped. What comes
# out is exactly what the verbs wrote.
#
# Usage:
#   ./run_quit.sh "tick" "weight ⊢∈⊤⊡⊣"
#   ./run_quit.sh -f cmds.txt          # one command per line; blank/# lines ignored
#   printf '%s\n' "tick" | ./run_quit.sh -
#
# Wrapped big integers inside one argv (or digit-only continuations under an
# incomplete bigint verb in a -f/- stream) are rejoined by
# join_digit_continuations.awk — same hold rule as the REPL. Each argv is
# joined on its own so a later command cannot glue onto a previous one.
#
# Env:
#   PROFILE=release|debug   (default: release)
#   KEEP_PROMPTS=1          keep the ⊙> command lines in the output
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

# Join digit wraps within one command stream; never across separate argv.
emit_cmds() {
  if [ "${1:-}" = "-f" ] || [ "${1:-}" = "--file" ]; then
    [ -n "${2:-}" ] || { echo "run_quit.sh: -f needs a path" >&2; exit 2; }
    # strip comments and blanks so a notes file can be fed straight in
    grep -vE '^\s*(#|$)' "$2" | awk -f "$JOIN_AWK"
  elif [ "${1:-}" = "-" ]; then
    grep -vE '^\s*(#|$)' | awk -f "$JOIN_AWK"
  elif [ "$#" -eq 0 ]; then
    echo "Usage: ./run_quit.sh \"cmd\" ... | -f file | -" >&2
    exit 2
  else
    local c
    for c in "$@"; do
      printf '%s\n' "$c" | awk -f "$JOIN_AWK"
    done
  fi
}

# Drop boot (everything before the first prompt), drop the quit exchange and
# shutdown, and by default drop the prompt echoes themselves so only verb
# output remains. KEEP_PROMPTS=1 keeps the ⊙> lines (still no boot/halt).
filter_results() {
  if [ "${KEEP_PROMPTS:-0}" = "1" ]; then
    awk '
      /^⊙> quit/ { exit }
      /^⊙> /     { active = 1 }
      active
    '
  else
    awk '
      /^⊙> quit/ { exit }
      /^⊙> /     { active = 1; next }
      active
    '
  fi
}

{ emit_cmds "$@"; printf 'quit\n'; } | "$BIN" | filter_results
