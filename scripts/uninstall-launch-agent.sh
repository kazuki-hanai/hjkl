#!/bin/sh
set -eu

LABEL="com.kazuki-hanai.hjkl-for-mac"
APP_NAME="hjkl-for-mac"

PREFIX=${PREFIX:-"$HOME/.local"}
BIN_DIR=${BIN_DIR:-"$PREFIX/bin"}
LAUNCH_AGENTS_DIR=${LAUNCH_AGENTS_DIR:-"$HOME/Library/LaunchAgents"}

BINARY_PATH="$BIN_DIR/$APP_NAME"
PLIST_PATH="$LAUNCH_AGENTS_DIR/$LABEL.plist"

echo "Unloading LaunchAgent if loaded..."
launchctl bootout "gui/$(id -u)/$LABEL" >/dev/null 2>&1 || true
launchctl bootout "gui/$(id -u)" "$PLIST_PATH" >/dev/null 2>&1 || true

echo "Removing LaunchAgent plist..."
rm -f "$PLIST_PATH"

if [ "${REMOVE_BINARY:-0}" = "1" ]; then
	echo "Removing binary..."
	rm -f "$BINARY_PATH"
else
	cat <<EOF
Leaving binary in place:
  $BINARY_PATH

To remove it too, run:
  REMOVE_BINARY=1 scripts/uninstall-launch-agent.sh
EOF
fi
