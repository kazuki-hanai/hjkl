#!/bin/sh
# Backward-compatible wrapper. Prefer scripts/install.sh.
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$SCRIPT_DIR/install.sh" "$@"
