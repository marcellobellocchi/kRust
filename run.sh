#!/usr/bin/env bash
# Build a bootable ISO from the most recent kernel binary cargo produced
# (debug or release, whichever is newer) and optionally run it in QEMU.
#
# Usage:
#   ./run.sh                build ISO from latest binary, then ask to launch
#   ./run.sh -l             launch the most recent ISO, without building
#   ./run.sh -y | -n        build, then launch (-y) or don't (-n) without asking
#   ./run.sh -u             boot with UEFI firmware (OVMF) instead of BIOS
#   ./run.sh -g             start QEMU paused, waiting for GDB on port 1234
#   ./run.sh -- ARGS...     pass extra arguments to QEMU
#
# The kernel crate is expected in <workspace root>/kernel; build it with
# `cargo kbuild` (debug) or `cargo kbuild --release` first.
# ISOs go to <cargo target dir>/iso/, keeping the newest $ISO_KEEP.
# Environment overrides: KERNEL_DIR, KERNEL_NAME, ISO_KEEP, QEMU_MEM, OVMF.

set -euo pipefail

ISO_KEEP="${ISO_KEEP:-5}"
QEMU_MEM="${QEMU_MEM:-512M}"

die()  { printf 'error: %s\n' "$*" >&2; exit 1; }
info() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }

usage() { sed -n '2,15s/^# \{0,1\}//p' "$0"; }

# ---------- options ----------
launch_latest=0 ask=1 launch=0 uefi=0 gdb=0
qemu_extra=()
while (($#)); do
    case "$1" in
        -l|--latest) launch_latest=1 ;;
        -y|--yes)    ask=0; launch=1 ;;
        -n|--no)     ask=0; launch=0 ;;
        -u|--uefi)   uefi=1 ;;
        -g|--gdb)    gdb=1 ;;
        -h|--help)   usage; exit 0 ;;
        --)          shift; qemu_extra=("$@"); break ;;
        *)           usage >&2; die "unknown option: $1" ;;
    esac
    shift
done

# ---------- project layout ----------
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"
command -v cargo >/dev/null || die "cargo not found"
# Workspace root, found from wherever this script lives
workspace_manifest="$(cargo locate-project --workspace --message-format plain 2>/dev/null)" \
    || die "no Cargo.toml found from $script_dir upward"
root="$(dirname "$workspace_manifest")"

kernel_dir="$root/${KERNEL_DIR:-kernel}"
kernel_manifest="$kernel_dir/Cargo.toml"
[[ -f "$kernel_manifest" ]] || die "kernel crate not found: ${kernel_manifest#"$root"/} (set KERNEL_DIR)"

# Respects CARGO_TARGET_DIR and build.target-dir in .cargo/config.toml
target_dir="$(cd "$root" && cargo metadata --no-deps --format-version 1 2>/dev/null \
    | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
[[ -n "$target_dir" ]] || target_dir="${CARGO_TARGET_DIR:-$root/target}"
iso_dir="$target_dir/iso"

# Kernel binary name = kernel package name, unless overridden
kernel_name="${KERNEL_NAME:-$(awk '
    /^\[/        { in_pkg = ($0 == "[package]") }
    in_pkg && /^[[:space:]]*name[[:space:]]*=/ {
        sub(/^[^=]*=[[:space:]]*"/, ""); sub(/".*/, ""); print; exit
    }' "$kernel_manifest")}"
[[ -n "$kernel_name" ]] || die "could not read package name from ${kernel_manifest#"$root"/} (set KERNEL_NAME)"

# ---------- helpers ----------
newest() {  # from "mtime path" lines, print the path with the newest mtime
    sort -n | tail -n 1 | cut -d' ' -f2-
}

find_latest_kernel() {
    [[ -d "$target_dir" ]] || return 0
    # target/{debug,release}/NAME or target/<triple>/{debug,release}/NAME
    find "$target_dir" -maxdepth 3 -type f \
        \( -path "*/debug/$kernel_name" -o -path "*/release/$kernel_name" \) \
        -printf '%T@ %p\n' | newest
}

find_latest_iso() {
    [[ -d "$iso_dir" ]] || return 0
    find "$iso_dir" -maxdepth 1 -type f -name '*.iso' -printf '%T@ %p\n' | newest
}

find_ovmf() {
    if [[ -n "${OVMF:-}" ]]; then
        [[ -f "$OVMF" ]] || die "OVMF=$OVMF does not exist"
        printf '%s' "$OVMF"; return
    fi
    local f
    for f in /usr/share/ovmf/OVMF.fd \
             /usr/share/OVMF/OVMF.fd \
             /usr/share/edk2/x64/OVMF.4m.fd \
             /usr/share/edk2/x64/OVMF.fd \
             /usr/share/edk2/ovmf/OVMF.fd \
             /usr/share/edk2-ovmf/x64/OVMF.fd \
             /usr/share/qemu/ovmf-x86_64.bin; do
        [[ -f "$f" ]] && { printf '%s' "$f"; return; }
    done
    die "UEFI firmware (OVMF) not found; install ovmf/edk2-ovmf or set OVMF=/path/to/OVMF.fd"
}

build_iso() {
    local kernel="$1" profile stage iso mkrescue log
    profile="$(basename "$(dirname "$kernel")")"

    mkrescue="$(command -v grub-mkrescue || command -v grub2-mkrescue || true)"
    [[ -n "$mkrescue" ]] || die "grub-mkrescue not found (install grub plus xorriso and mtools)"
    command -v xorriso >/dev/null || die "xorriso not found (needed by grub-mkrescue)"

    if command -v grub-file >/dev/null && ! grub-file --is-x86-multiboot2 "$kernel"; then
        die "$kernel is not a valid Multiboot2 kernel"
    fi

    # Warn if the kernel crate changed after the binary was built
    if [[ -n "$(find "$kernel_dir" -path "$kernel_dir/target" -prune -o \
                \( -name '*.rs' -o -name '*.s' -o -name '*.ld' -o -name Cargo.toml \) \
                -newer "$kernel" -print -quit)" ]]; then
        warn "kernel sources are newer than the $profile binary; did you forget 'cargo kbuild'?"
    fi

    stage="$(mktemp -d)"
    log="$(mktemp)"
    trap 'rm -rf "$stage" "$log"' EXIT

    mkdir -p "$stage/boot/grub" "$iso_dir"
    cp "$kernel" "$stage/boot/kernel.elf"
    cat > "$stage/boot/grub/grub.cfg" <<EOF
set timeout=0
set default=0
menuentry "$kernel_name ($profile)" {
    multiboot2 /boot/kernel.elf
    boot
}
EOF

    iso="$iso_dir/${kernel_name}-${profile}-$(date +%Y%m%d-%H%M%S).iso"
    info "building ISO from $profile binary: ${kernel#"$root"/}"
    if ! "$mkrescue" -o "$iso" "$stage" >"$log" 2>&1; then
        cat "$log" >&2
        die "grub-mkrescue failed"
    fi
    rm -rf "$stage" "$log"   # explicit: run_qemu uses exec, which skips EXIT traps
    trap - EXIT
    info "created ${iso#"$root"/}"

    # Keep only the newest $ISO_KEEP ISOs
    find "$iso_dir" -maxdepth 1 -type f -name '*.iso' -printf '%T@ %p\n' \
        | sort -rn | tail -n +"$((ISO_KEEP + 1))" | cut -d' ' -f2- \
        | while IFS= read -r old; do rm -f -- "$old"; done

    built_iso="$iso"
}

run_qemu() {
    local iso="$1" args ovmf
    command -v qemu-system-x86_64 >/dev/null || die "qemu-system-x86_64 not found"

    args=(-cdrom "$iso" -m "$QEMU_MEM" -serial stdio -no-reboot)
    if ((uefi)); then
        ovmf="$(find_ovmf)" || exit 1
        args+=(-bios "$ovmf")
    fi
    if ((gdb)); then
        args+=(-s -S)
        info "QEMU paused; connect with: gdb -ex 'target remote :1234'"
    fi
    args+=("${qemu_extra[@]}")

    info "launching ${iso#"$root"/}$( ((uefi)) && printf ' (UEFI)')"
    exec qemu-system-x86_64 "${args[@]}"
}

# ---------- main ----------
if ((launch_latest)); then
    iso="$(find_latest_iso)"
    [[ -n "$iso" ]] || die "no ISO in ${iso_dir#"$root"/}; run without -l to build one"
    run_qemu "$iso"
fi

kernel="$(find_latest_kernel)"
[[ -n "$kernel" ]] || die "no '$kernel_name' binary under ${target_dir#"$root"/}/*/{debug,release}; run 'cargo kbuild' first"

built_iso=""
build_iso "$kernel"
iso="$built_iso"

if ((ask)); then
    if [[ -t 0 ]]; then
        read -r -p "Launch in QEMU now? [y/N] " answer || answer=""
        [[ "$answer" =~ ^[Yy]([Ee][Ss])?$ ]] && launch=1
    else
        info "not a terminal, not launching (use -y to launch)"
    fi
fi

((launch)) && run_qemu "$iso"
exit 0