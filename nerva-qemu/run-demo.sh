#!/usr/bin/env bash
# Compatibility entry point; the running neural model lives in Modell/.
set -euo pipefail
adapter_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec "$adapter_dir/../Modell/run-demo.sh" "$@"
