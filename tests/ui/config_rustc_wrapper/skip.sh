#!/bin/sh
# Stands in for a compile cache that answers a unit without the driver: when
# handed the driver as its first argument it runs rustc without it, so no
# fragment is written for the unit. Probes (`rustc -vV`) pass straight through.
case "$1" in
*/no-alloc-driver | no-alloc-driver) shift ;;
esac
exec "$@"
