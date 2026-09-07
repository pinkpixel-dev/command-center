#!/bin/sh
# Checks the one thing that goes wrong on a NAS and says so plainly, rather
# than letting it surface as a failed write halfway through startup.
set -e

DATA_DIR="${COMMAND_CENTER_DATA_DIR:-/var/lib/command-center}"

if [ ! -d "$DATA_DIR" ]; then
  echo "command-center: $DATA_DIR does not exist. Mount a volume there." >&2
  exit 1
fi

# A bind mount arrives with the host's ownership, which is usually not this
# container's user. The fix is on the host, so the message names what to run.
if [ ! -w "$DATA_DIR" ]; then
  echo "command-center: $DATA_DIR is not writable by uid $(id -u):$(id -g)." >&2
  echo "  On the host, run: chown -R $(id -u):$(id -g) <the directory you mounted>" >&2
  echo "  Or start the container with --user to match the directory's owner." >&2
  exit 1
fi

exec "$@"
