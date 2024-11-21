#!/bin/sh

if [ $# -ne 1 ]; then
    printf "Usage: ./up_disk.sh [userapp bin path]\n"
    exit
fi

FILE=$1

if [ ! -f $FILE ]; then
    printf "File '$FILE' doesn't exist!\n"
    exit
fi

if [ ! -f ./disk.img ]; then
    printf "disk.img doesn't exist! Please 'make disk_img'\n"
    make disk_img
    # exit
fi

printf "Try Use Docker to Add file '$FILE' into disk.img\n"

# https://stackoverflow.com/questions/52348221/can-i-use-mount-inside-a-docker-alpine-container
docker run  --privileged --platform linux/riscv64  --rm -it  -v `pwd`:/tmp -w /tmp  myrisc/alpine:3 ./update_disk.sh $FILE