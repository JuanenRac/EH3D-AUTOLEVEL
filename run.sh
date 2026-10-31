#!/usr/bin/env bash
# Runs eh3d-autolevel with its sample. Extra arguments replace the sample: ./run.sh [ARGUMENTS]
set -u
cd "$(dirname "$0")"
pause() { [ -n "${NOPAUSE:-}" ] || { read -r -p "Press Enter to close..." _ || true; }; }
trap pause EXIT
if [ "$#" -eq 0 ]; then set -- info examples/table.txt; fi
cargo run --release --quiet -- "$@"
