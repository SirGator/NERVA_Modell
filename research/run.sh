#!/usr/bin/env bash
set -euo pipefail
research_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ "${1:-}" == history ]]; then
    shift
    exec python3 "$research_dir/learning_history.py" "$@"
fi
if [[ "${1:-}" == control ]]; then
    shift
    exec python3 "$research_dir/control.py" "$@"
fi
if [[ "${1:-}" == localize-homeostasis ]]; then
    shift
    exec python3 "$research_dir/localize_homeostasis.py" "$@"
fi
if [[ "${1:-}" == confirm-homeostasis ]]; then
    shift
    exec python3 "$research_dir/confirm_homeostasis.py" "$@"
fi
exec python3 "$research_dir/run.py" "$@"
