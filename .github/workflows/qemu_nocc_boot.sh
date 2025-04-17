#!/bin/bash

set -u

SUCCESS="Terminating current task"
TIMEOUT=4s

# Clone STDOUT for live log reporting
exec 3>&1

run_svsm() {
  scripts/launch_guest.sh \
    --qemu ./tools/bin/qemu-system-x86_64 \
    --nocc |
    tee /proc/self/fd/3
}

echo "================================================================================"
timeout $TIMEOUT grep -q -m 1 "$SUCCESS" <(run_svsm)
RES=$?
echo "================================================================================"

case $RES in
0)
  echo "Test Pass!"
  exit 0
  ;;
124)
  echo "Test timeout"
  exit 1
  ;;
*)
  echo "Test Fail / Unknown error"
  exit 1
  ;;
esac
