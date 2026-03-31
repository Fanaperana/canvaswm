#!/usr/bin/env bash
# install.sh — install canvaswm without make/cargo
#
# Usage:
#   ./install.sh [--prefix /usr/local] [--destdir ""]
#
# Options:
#   --prefix PATH    Installation prefix (default: /usr/local)
#   --destdir PATH   Staging directory (default: empty, install directly)
#   --uninstall      Remove previously installed files

set -euo pipefail

PREFIX="/usr/local"
DESTDIR=""
UNINSTALL=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix)  PREFIX="$2";  shift 2 ;;
        --destdir) DESTDIR="$2"; shift 2 ;;
        --uninstall) UNINSTALL=1; shift ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

BINDIR="${DESTDIR}${PREFIX}/bin"
DATADIR="${DESTDIR}${PREFIX}/share/canvaswm"

if [[ $UNINSTALL -eq 1 ]]; then
    echo "Uninstalling canvaswm..."
    rm -f "${BINDIR}/canvaswm" "${BINDIR}/canvaswm-msg"
    rm -rf "${DATADIR}"
    echo "Done."
    exit 0
fi

# Build release binary
echo "Building canvaswm (release)..."
cargo build --release

# Install
echo "Installing to ${PREFIX}..."
install -Dm755 target/release/canvaswm      "${BINDIR}/canvaswm"
install -Dm755 extras/canvaswm-msg           "${BINDIR}/canvaswm-msg"

if [[ -f "example/canvaswm.toml" ]]; then
    install -Dm644 example/canvaswm.toml     "${DATADIR}/canvaswm.toml"
fi

echo ""
echo "canvaswm installed to ${BINDIR}/canvaswm"
echo "Add '${BINDIR}' to your PATH if needed, then run:"
echo "  canvaswm"
