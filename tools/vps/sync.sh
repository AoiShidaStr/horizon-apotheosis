#!/bin/bash
# Run on the PC: copy the native crate and the VPS scripts to the build server.
# Usage: bash tools/vps/sync.sh [check|test|full]   (with a mode, also starts the check in the background)
HERE="$(cd "$(dirname "$0")/../.." && pwd)"
VPS="root@<VPS>"
SSH="ssh -o BatchMode=yes -i $HOME<SSH_KEY>"
cd "$HERE" || exit 1
tar --exclude=target --exclude=__pycache__ -czf - native tools | $SSH $VPS 'mkdir -p /opt/build/mods/apotheosis && cd /opt/build/mods/apotheosis && rm -rf native tools && tar --no-same-owner -xzf - && sed -i "s/\r$//" tools/vps/*.sh'
if [ -n "$1" ]; then
  $SSH $VPS "nohup bash /opt/build/mods/apotheosis/tools/vps/check.sh $1 > /opt/build/apotheosis-check.log 2>&1 &"
  echo "started $1; log: /opt/build/apotheosis-check.log"
fi
