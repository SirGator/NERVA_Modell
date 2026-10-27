#!/usr/bin/env bash
set -euo pipefail

experiment_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
runtime_dir="$experiment_dir/../Modell/.tools/qemu"

needs_qemu=false
case "${1:-help}" in study|record|serial|--weights) needs_qemu=true ;; esac
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

experiment_binary="nerva-experiments"
case "${1:-help}" in
    m3) experiment_binary="nerva-m3"; shift ;;
    m7) experiment_binary="nerva-m7"; shift ;;
    m8) experiment_binary="nerva-m8"; shift ;;
    m9) experiment_binary="nerva-m9"; shift ;;
    sensorimotor) experiment_binary="nerva-sensorimotor"; shift ;;
    serial) experiment_binary="nerva-serial"; shift ;;
    --weights) experiment_binary="nerva-serial" ;;
esac
if [[ "$experiment_binary" == "nerva-serial" ]] && [[ $# -eq 0 || "${1:-}" == --weights ]]; then
    set -- demo "$@"
fi
exec cargo run --offline --release --manifest-path "$experiment_dir/Cargo.toml" --bin "$experiment_binary" -- "$@"
