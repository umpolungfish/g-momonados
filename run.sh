#!/usr/bin/env bash
# run.sh — build and run the G-mOMonadOS REPL.
#
# Hosted is the only mode this build has (see Cargo.toml), on by default, so
# unlike the parent kernel there is no bare-metal/hosted split to navigate
# here: plain `cargo build` already produces the right binary.
#
# Usage: ./run.sh [release|debug] [extra,cargo,features]
set -euo pipefail
cd "$(dirname "$0")"

PROFILE="${1:-release}"
FEATURES="${2:-}"

PROFILE_FLAG=()
[ "$PROFILE" = "release" ] && PROFILE_FLAG=(--release)
FEATURES_FLAG=()
[ -n "$FEATURES" ] && FEATURES_FLAG=(--features "$FEATURES")

cargo build "${PROFILE_FLAG[@]}" "${FEATURES_FLAG[@]}"
exec "target/${PROFILE}/g-momonados"
