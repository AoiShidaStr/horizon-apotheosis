#!/bin/bash
# Run on the VPS through the FIFO queue. Stages the crate into the mod's Horizon
# worktree, then checks the crate, runs its tests, checks both engine flavours and
# runs the registry checks on the packed .hmod/.hnmod.
. ~/.cargo/env
export CARGO_TARGET_DIR=/opt/build/target CARGO_BUILD_JOBS=4
cd /opt/build/wt/apotheosis || exit 1
bash /opt/build/mods/apotheosis/tools/vps/stage.sh || { echo "stage_exit=1"; exit 1; }

echo "== mod crate check (server flavour)"; date
nice -n 10 cargo check -p mod-apotheosis --message-format short 2>&1 | grep -vE "^[[:space:]]+(Compiling|Checking)"
echo "check_exit=${PIPESTATUS[0]}"

echo "== mod tests"; date
nice -n 10 cargo test -p mod-apotheosis --message-format short 2>&1 | grep -vE "^[[:space:]]+(Compiling|Checking)" | tail -100
echo "test_exit=${PIPESTATUS[0]}"

echo "== server engine check (no default features)"; date
nice -n 10 cargo check -p horizon-native-game --no-default-features --message-format short 2>&1 | grep -vE "^[[:space:]]+(Compiling|Checking)"
echo "server_exit=${PIPESTATUS[0]}"

echo "== client flavour check (default features)"; date
nice -n 10 cargo check -p horizon-native-game --message-format short 2>&1 | grep -vE "^[[:space:]]+(Compiling|Checking)"
echo "client_exit=${PIPESTATUS[0]}"

echo "== horizon-mod check: apotheosis.hmod"
nice -n 10 cargo run -p cargo-horizon-mod -- check /opt/build/mods/apotheosis/target/apotheosis.hmod --json 2>&1 | tail -60
echo "hmod_exit=${PIPESTATUS[0]}"

echo "== horizon-mod check: apotheosis.hnmod"
nice -n 10 cargo run -p cargo-horizon-mod -- check /opt/build/mods/apotheosis/target/apotheosis.hnmod --json 2>&1 | tail -60
echo "hnmod_exit=${PIPESTATUS[0]}"

date; echo DONE
