# Persistent storage with Qemu pflash and FAT fs

Build coconut as usual.

Use the legacy (non-igvm) launch method.

example Qemu command line:
```
"${QEMU}" \
  -enable-kvm \
  -cpu EPYC-v4 \
  -smp 4 \
  \
  -machine q35,confidential-guest-support=sev0,memory-backend=ram1,kvm-type=protected \
  -object memory-backend-memfd-private,id=ram1,size=4G,share=true \
  -object sev-snp-guest,id=sev0,cbitpos=51,reduced-phys-bits=1,svsm=on \
  \
  -drive if=pflash,format=raw,unit=0,file="${FW}",readonly=on \
  -drive if=pflash,format=raw,unit=1,file="$VARS",snapshot=off,readonly=off \
  -drive if=pflash,format=raw,unit=2,file="${SVSM}",readonly=on \
  -drive if=pflash,unit=3,format=raw,file=./f3.raw,readonly=off \
  \
  -drive file="${IMG}",if=none,id=disk0,format=qcow2,snapshot=on \
  -device virtio-scsi-pci,id=scsi0,disable-legacy=on,iommu_platform=true \
  -device scsi-hd,drive=disk0 \
  -nic user,model=virtio-net-pci,hostfwd=tcp::5522-:22 \
  -nographic \
  -chardev stdio,id=s,signal=on \
  -serial chardev:s \
  -monitor none \
  -serial unix:./svsm-gdb,server=on,wait=off \
  -trace "kvm_sev_snp*" \
  -trace "os_*" \
  -trace "*pflash*
```

Important part is this line:
```
-drive if=pflash,unit=3,format=raw,file=./f3.raw,readonly=off \
```

The size of the pflash device is hard-coded to 1MB, see `svsm.rs` around line 443.

There is a pre-formatted image file (`f3.raw`) included in the repo. It is FAT-12 formatted
due to size constraints of the pflash. The driver can handle FAT-16 and FAT-32 as well.

