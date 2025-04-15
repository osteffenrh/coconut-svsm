#!/bin/bash

set -u

SUCCESS="Terminating current task"
TIMEOUT=4s

# Clone STDOUT for live log reporting
exec 3>&1

run_svsm() {

  ./tools/bin/qemu-system-x86_64 \
    -cpu max,smep=on \
    -machine q35,confidential-guest-support=cgs0,memory-backend=mem0,igvm-cfg=igvm0,accel=tcg \
    -object memory-backend-memfd,size=1G,id=mem0,share=true,prealloc=false,reserve=false \
    -object igvm-cfg,id=igvm0,file=bin/coconut-qemu.igvm \
    -object nocc,id=cgs0 \
    -smp 4 \
    -no-reboot \
    -net none \
    -vga none \
    -nographic \
    -monitor none \
    -serial stdio | tee /proc/self/fd/3
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
