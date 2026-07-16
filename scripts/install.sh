#!/bin/sh
set -eu

# Build hjkl-for-mac, install the binary, and enable it as a per-user
# macOS LaunchAgent (auto-start at login). All launchd handling is done by
# the binary's own `enable` subcommand, so this script stays thin.

APP_NAME="hjkl-for-mac"

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

PREFIX=${PREFIX:-"$HOME/.local"}
BIN_DIR=${BIN_DIR:-"$PREFIX/bin"}
BINARY_PATH=${BINARY_PATH:-"$BIN_DIR/$APP_NAME"}

usage() {
	cat <<USAGE_EOF
Usage: scripts/install.sh [--no-build] [--no-enable]

Build, install, and enable hjkl-for-mac as a per-user LaunchAgent.

Options:
  --no-build     Reuse target/release/$APP_NAME instead of building.
  --no-enable    Install the binary only; do not enable the LaunchAgent.
  -h, --help     Show this help.

Environment overrides: PREFIX, BIN_DIR, BINARY_PATH.
USAGE_EOF
}

BUILD=1
ENABLE=1
for arg in "$@"; do
	case "$arg" in
		--no-build) BUILD=0 ;;
		--no-enable) ENABLE=0 ;;
		-h|--help) usage; exit 0 ;;
		*) echo "error: unknown option: $arg" >&2; exit 1 ;;
	esac
done

[ "$(uname -s)" = "Darwin" ] || { echo "error: hjkl-for-mac only supports macOS" >&2; exit 1; }
[ "$(id -u)" -ne 0 ] || { echo "error: run as your login user, not root" >&2; exit 1; }

SOURCE_BINARY="$REPO_DIR/target/release/$APP_NAME"

if [ "$BUILD" -eq 1 ]; then
	command -v cargo >/dev/null 2>&1 || { echo "error: cargo not found" >&2; exit 1; }
	echo "Building $APP_NAME..."
	cargo build --release --manifest-path "$REPO_DIR/Cargo.toml"
fi

[ -x "$SOURCE_BINARY" ] || { echo "error: release binary not found: $SOURCE_BINARY" >&2; exit 1; }

echo "Installing binary to $BINARY_PATH..."
mkdir -p "$BIN_DIR"
install -m 0755 "$SOURCE_BINARY" "$BINARY_PATH"

if [ "$ENABLE" -eq 1 ]; then
	echo "Enabling LaunchAgent..."
	"$BINARY_PATH" enable
fi

cat <<DONE_EOF

Done. Manage the service with:
  $BINARY_PATH status
  $BINARY_PATH start | stop | restart
  $BINARY_PATH enable | disable

If keys are not remapped yet, grant Accessibility permission to:
  $BINARY_PATH
System Settings -> Privacy & Security -> Accessibility
Then run: "$BINARY_PATH" restart
DONE_EOF
