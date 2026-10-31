#!/bin/sh
# Stands in for a custom compiler (a wrapper script adding `--cfg` flags, say).
# The checker compiles with the toolchain's own rustc and ignores the key; if
# this ever ran, the build would fail with no verdict at all.
echo "error: the configured build.rustc must not be used by the checker" >&2
exit 1
