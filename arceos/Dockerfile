
FROM alpine:3.20.3
RUN apk add libc-dev gcc dosfstools make sudo

# 用来编译musl/app  和更新 disk.img
# docker buildx build --platform linux/riscv64  -t myrisc/alpine:3 .
# docker run --rm --mount type=bind,source="$(pwd)",dst=/ros  -w /ros myrisc/alpine:3 objdump -D target/riscv64gc-unknown-none-elf/release/skernel2
