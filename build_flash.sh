#!/bin/sh
#set -ex 

dev_reg='^\/sys\/block\/([A-Z]*[a-z]*[0-9]*)+\/device\/model'
devices=$(ls /sys/block/*/device/model)

for device in $devices; do
  echo "Checking $device..."
  if grep -q RP2 "$device"; then
    echo "Found RP2 at $device."
    if [[ $device =~ $dev_reg ]]; then
      partition="${BASH_REMATCH[1]}1"
      mountpoint -qx /dev/$partition
      if [[ $? -eq 0 ]]; then 
        echo "/dev/$partition found"
      else
        echo "No device: /dev/$partition"
        exit -1
      fi
      mountpoint -q /mnt/bletroller
      if [[ $? -eq 0 ]]; then
        echo "Already mounted to flash"
      else
        sudo mkdir -p /mnt/bletroller
        #sudo mount -o gid=users,fmask=113,dmask=002 /dev/$partition /mnt/bletroller
        udisksctl mount -b /dev/sda1
        echo "Mounted /mnt/bletroller to flash"
      fi
    fi

    # build & flash
    cargo run
    exit 0
  fi
done
