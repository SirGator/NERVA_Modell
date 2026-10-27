#!/usr/bin/env bash
set -euo pipefail
model_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$model_dir/../Dashboard/server.py" --live --root "$model_dir/../../Messungen" "$@"
