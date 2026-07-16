#!/bin/sh
# Backward-compatible wrapper. Prefer scripts/uninstall.sh.
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$SCRIPT_DIR/uninstall.sh" "$@"
