#!/bin/sh
set -eu

LABEL="com.kazuki-hanai.hjkl-for-mac"
APP_NAME="hjkl-for-mac"

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

PREFIX=${PREFIX:-"$HOME/.local"}
BIN_DIR=${BIN_DIR:-"$PREFIX/bin"}
LAUNCH_AGENTS_DIR=${LAUNCH_AGENTS_DIR:-"$HOME/Library/LaunchAgents"}
LOG_DIR=${LOG_DIR:-"$HOME/Library/Logs"}

BINARY_PATH="$BIN_DIR/$APP_NAME"
PLIST_PATH="$LAUNCH_AGENTS_DIR/$LABEL.plist"
TEMPLATE_PATH="$REPO_DIR/launchd/$LABEL.plist.template"
STDOUT_LOG_PATH="$LOG_DIR/$APP_NAME.log"
STDERR_LOG_PATH="$LOG_DIR/$APP_NAME.err.log"

escape_sed_replacement() {
	printf '%s' "$1" | sed 's/[\/&]/\\&/g'
}

render_plist() {
	sed \
		-e "s/__BINARY_PATH__/$(escape_sed_replacement "$BINARY_PATH")/g" \
		-e "s/__STDOUT_LOG_PATH__/$(escape_sed_replacement "$STDOUT_LOG_PATH")/g" \
		-e "s/__STDERR_LOG_PATH__/$(escape_sed_replacement "$STDERR_LOG_PATH")/g" \
		"$TEMPLATE_PATH" > "$PLIST_PATH"
}

mkdir -p "$BIN_DIR" "$LAUNCH_AGENTS_DIR" "$LOG_DIR"

echo "Building $APP_NAME..."
cargo build --release --manifest-path "$REPO_DIR/Cargo.toml"

echo "Installing binary to $BINARY_PATH..."
install -m 0755 "$REPO_DIR/target/release/$APP_NAME" "$BINARY_PATH"

echo "Installing LaunchAgent to $PLIST_PATH..."
render_plist
plutil -lint "$PLIST_PATH"

echo "Reloading LaunchAgent..."
launchctl bootout "gui/$(id -u)/$LABEL" >/dev/null 2>&1 || true
launchctl bootstrap "gui/$(id -u)" "$PLIST_PATH"
launchctl enable "gui/$(id -u)/$LABEL"
launchctl kickstart -k "gui/$(id -u)/$LABEL"

cat <<EOF

Installed $APP_NAME as a LaunchAgent.

Next:
  1. Open System Settings -> Privacy & Security -> Accessibility.
  2. Allow this binary:
     $BINARY_PATH
  3. If it does not start immediately after permission is granted, run:
     launchctl kickstart -k gui/$(id -u)/$LABEL

Logs:
  $STDOUT_LOG_PATH
  $STDERR_LOG_PATH
EOF
