#!/bin/bash
# Run on the VPS. Type-check apotheosis: the mod crate (server flavour, no default features)
# and, with `full`, its tests, the dedicated-server engine (horizon-native-game, no default
# features) and the client flavour (default features).
. ~/.cargo/env
export CARGO_TARGET_DIR=/opt/build/target CARGO_BUILD_JOBS=4
cd /opt/build/wt/apotheosis
bash /opt/build/mods/apotheosis/tools/vps/stage.sh
MODE=${1:-check}
echo "== mod crate check (server flavour)"; date
nice -n 10 cargo check -p mod-apotheosis --message-format short 2>&1 | grep -vE "^\s+(Compiling|Checking)"
echo "check_exit=${PIPESTATUS[0]}"
if [ "$MODE" = full ] || [ "$MODE" = test ]; then
  echo "== mod tests"; date
  nice -n 10 cargo test -p mod-apotheosis --message-format short 2>&1 | grep -vE "^\s+(Compiling|Checking)" | tail -60
  echo "test_exit=${PIPESTATUS[0]}"
fi
if [ "$MODE" = full ]; then
  echo "== server engine check (no default features)"; date
  nice -n 10 cargo check -p horizon-native-game --no-default-features --message-format short 2>&1 | grep -vE "^\s+(Compiling|Checking)"
  echo "server_exit=${PIPESTATUS[0]}"
  echo "== client flavour check (default features)"; date
  nice -n 10 cargo check -p horizon-native-game --message-format short 2>&1 | grep -vE "^\s+(Compiling|Checking)"
  echo "client_exit=${PIPESTATUS[0]}"
fi
date; echo DONE
