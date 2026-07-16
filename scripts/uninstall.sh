#!/bin/sh
set -eu

# Disable/stop the hjkl-for-mac LaunchAgent and optionally remove the binary.
# launchd handling is delegated to the binary's own `disable` subcommand.

APP_NAME="hjkl-for-mac"
LABEL="com.kazuki-hanai.hjkl-for-mac"

PREFIX=${PREFIX:-"$HOME/.local"}
BIN_DIR=${BIN_DIR:-"$PREFIX/bin"}
BINARY_PATH=${BINARY_PATH:-"$BIN_DIR/$APP_NAME"}

usage() {
	cat <<USAGE_EOF
Usage: scripts/uninstall.sh [--remove-binary]

Disable and stop the hjkl-for-mac LaunchAgent.

Options:
  --remove-binary   Also remove the installed binary.
  -h, --help        Show this help.

Environment overrides: PREFIX, BIN_DIR, BINARY_PATH.
USAGE_EOF
}

REMOVE_BINARY=0
for arg in "$@"; do
	case "$arg" in
		--remove-binary) REMOVE_BINARY=1 ;;
		-h|--help) usage; exit 0 ;;
		*) echo "error: unknown option: $arg" >&2; exit 1 ;;
	esac
done

[ "$(uname -s)" = "Darwin" ] || { echo "error: hjkl-for-mac only supports macOS" >&2; exit 1; }

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
else
	echo "Leaving binary in place: $BINARY_PATH (use --remove-binary to remove)."
fi
