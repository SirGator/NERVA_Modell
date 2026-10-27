#!/usr/bin/env bash
set -euo pipefail

model_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
runtime_dir="$model_dir/.tools/qemu"

if [[ "${1:-}" == "gui" || "${1:-}" == "dashboard" ]]; then
    shift
    exec "$model_dir/gui.sh" "$@"
fi

if [[ "${1:-}" == "m4" || "${1:-}" == "m5" || "${1:-}" == "m6" ]]; then
    experiment_bin="nerva-$1"
    shift
    exec cargo run --offline --release --manifest-path "$model_dir/Cargo.toml" --bin "$experiment_bin" -- "$@"
fi

# Preserve earlier invocations while keeping experimental code in its own crate.
case "${1:-}" in
    study|study-replay|record|retention|agency|sensorimotor|m3|serial|--weights)
        echo "Weiterleitung an NERVA_Modell/Experimente/run.sh." >&2
        exec "$model_dir/../Experimente/run.sh" "$@"
        ;;
esac

needs_qemu=true
case "${1:-}" in layout|help|-h|--help) needs_qemu=false ;; esac
if "$needs_qemu" && [[ -z "${QEMU_SYSTEM_X86_64:-}" ]] && ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    if [[ ! -x "$runtime_dir/usr/bin/qemu-system-x86_64" ]]; then
        echo "QEMU fehlt. Unter Ubuntu/Debian: sudo apt install qemu-system-x86 binutils" >&2
        exit 1
    fi
    export QEMU_SYSTEM_X86_64="$runtime_dir/usr/bin/qemu-system-x86_64"
    export QEMU_FIRMWARE_DIR="$runtime_dir/usr/share/qemu"
    export QEMU_VGA_ROM="$runtime_dir/usr/share/seabios/vgabios-stdvga.bin"
    export QEMU_MODULE_DIR="$runtime_dir/usr/lib/x86_64-linux-gnu/qemu"
    export LD_LIBRARY_PATH="$runtime_dir/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi
if [[ $# -eq 0 ]] || [[ "${1:-}" == --* && "${1:-}" != --help ]]; then
    set -- demo "$@"
fi
exec cargo run --offline --release --manifest-path "$model_dir/Cargo.toml" --bin nerva-vm -- "$@"
