#!/bin/sh

if [ $# -ne 1 ]; then
    printf "Usage: ./update.sh [userapp path]\n"
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

printf "Try Use Docker to Write file '$FILE' into disk.img\n"

docker run  --privileged --platform linux/riscv64  --rm -it  -v `pwd`:/tmp -w /tmp  myrisc/alpine:3 ./update_disk.sh $FILE