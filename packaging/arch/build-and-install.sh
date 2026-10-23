#!/usr/bin/env bash
# Builds the Flemozi Arch package from this checkout and installs it.
#
# Usage: packaging/arch/build-and-install.sh [--no-install] [--clean]
#   --no-install  only build the package
#   --clean       discard cached build artifacts (full rebuild)
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$script_dir/../.." && pwd)"

install=1
clean=0
for arg in "$@"; do
    case "$arg" in
        --no-install) install=0 ;;
        --clean) clean=1 ;;
        -h|--help) sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "Unknown option: $arg" >&2; exit 2 ;;
    esac
done

if [[ $EUID -eq 0 ]]; then
    echo "Run as a normal user; makepkg refuses to run as root (sudo is used for install)." >&2
    exit 1
fi

# ── dependencies ──
missing=()
for p in base-devel gtk3 libayatana-appindicator libxkbcommon wayland fontconfig; do
    pacman -Qq "$p" &>/dev/null || pacman -Qqg "$p" &>/dev/null || missing+=("$p")
done
command -v cargo &>/dev/null || missing+=(rustup)
if ((${#missing[@]})); then
    echo "==> Installing missing dependencies: ${missing[*]}"
    sudo pacman -S --needed "${missing[@]}"
fi

if command -v rustup &>/dev/null && ! rustup default &>/dev/null; then
    echo "==> No default Rust toolchain; installing stable"
    rustup default stable
fi

# ── GIF API key ──
if ! grep -qs '^KLIPY_API_KEY=.\+' "$root/.env" && [[ -z "${KLIPY_API_KEY:-}" ]]; then
    echo "==> WARNING: KLIPY_API_KEY not found in $root/.env — the GIF tab will not work." >&2
    echo "    Get a free key at https://partner.klipy.com and add: KLIPY_API_KEY=<key>" >&2
elif [[ -n "${KLIPY_API_KEY:-}" ]] && ! grep -qs '^KLIPY_API_KEY=.\+' "$root/.env"; then
    # build.rs only embeds keys from .env.
    echo "==> Writing KLIPY_API_KEY from environment to .env"
    printf 'KLIPY_API_KEY=%s\n' "$KLIPY_API_KEY" >> "$root/.env"
    chmod 600 "$root/.env"
fi

# ── build ──
cd "$script_dir"
rm -f ./*.pkg.tar.zst
echo "==> Building package"
if ((clean)); then
    makepkg -f --cleanbuild
else
    makepkg -f
fi

pkg=$(ls -t ./*.pkg.tar.zst | grep -v -- '-debug-' | head -n1)
pkg="$script_dir/${pkg#./}"
echo "==> Built: $pkg"

# ── install ──
if ((install)); then
    running=$(pgrep -x flemozi || true)
    echo "==> Installing"
    sudo pacman -U "$pkg"
    if [[ -n "$running" ]]; then
        echo "==> Restarting running Flemozi"
        for pid in $running; do kill "$pid" 2>/dev/null || true; done
        sleep 1
        setsid -f flemozi --hidden >/dev/null 2>&1 < /dev/null
    fi
    echo "==> Done. Bind 'flemozi --toggle' to a shortcut in System Settings → Shortcuts."
fi
