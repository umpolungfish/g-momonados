#!/usr/bin/env bash
# Execute one relation without entering the boot or REPL presentation layers.
set -euo pipefail
cd "$(dirname "$0")"
PROFILE="${PROFILE:-release}"
if [ "${1:-}" = "--stdin" ]; then
  shift
  exec "target/${PROFILE}/g-momonados" --selector-relation-stdin "$@"
fi
exec "target/${PROFILE}/g-momonados" --selector-relation "$@"
