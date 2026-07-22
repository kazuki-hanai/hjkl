#!/bin/sh
set -eu

# Disable/stop the macOS hjkl LaunchAgent and remove the installed binary by
# default. launchd handling is delegated to the binary's own `disable`
# subcommand.

APP_NAME="hjkl"
LABEL="com.kazuki-hanai.hjkl"

PREFIX=${PREFIX:-"$HOME/.local"}
BIN_DIR=${BIN_DIR:-"$PREFIX/bin"}
BINARY_PATH=${BINARY_PATH:-"$BIN_DIR/$APP_NAME"}

usage() {
	cat <<USAGE_EOF
Usage: scripts/uninstall.sh [--keep-binary]

Disable and stop the macOS hjkl LaunchAgent, then remove the installed binary.

Options:
  --keep-binary     Disable/stop only; leave the installed binary in place.
  --remove-binary   Deprecated no-op; binary removal is now the default.
  -h, --help        Show this help.

Environment overrides: PREFIX, BIN_DIR, BINARY_PATH.
USAGE_EOF
}

REMOVE_BINARY=1
for arg in "$@"; do
	case "$arg" in
		--keep-binary) REMOVE_BINARY=0 ;;
		--remove-binary) REMOVE_BINARY=1 ;;
		-h|--help) usage; exit 0 ;;
		*) echo "error: unknown option: $arg" >&2; exit 1 ;;
	esac
done

[ "$(uname -s)" = "Darwin" ] || { echo "error: scripts/uninstall.sh is macOS-only; on Windows run 'hjkl disable' and remove hjkl.exe" >&2; exit 1; }

if [ -x "$BINARY_PATH" ]; then
	echo "Disabling LaunchAgent..."
	"$BINARY_PATH" disable || true
else
	echo "Binary not found at $BINARY_PATH; cleaning up via launchctl..."
	launchctl bootout "gui/$(id -u)/$LABEL" >/dev/null 2>&1 || true
	rm -f "$HOME/Library/LaunchAgents/$LABEL.plist"
fi

if [ "$REMOVE_BINARY" -eq 1 ]; then
	echo "Removing binary $BINARY_PATH..."
	rm -f "$BINARY_PATH"
	remaining=$(command -v "$APP_NAME" 2>/dev/null || true)
	if [ -n "$remaining" ]; then
		echo "warning: another '$APP_NAME' is still on PATH: $remaining" >&2
		echo "         remove it manually if you did not intend to keep it." >&2
	fi
	echo "Done. If '$APP_NAME' still runs in this terminal, open a new one or run: hash -r"
else
	echo "Leaving binary in place: $BINARY_PATH (--keep-binary was used)."
fi
