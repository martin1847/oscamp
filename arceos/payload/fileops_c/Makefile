TARGET := fileops

CC := riscv64-linux-musl-gcc
STRIP := riscv64-linux-musl-strip

CC := docker run --platform linux/riscv64 -v `pwd`:/tmp -w /tmp myrisc/alpine:3 gcc
STRIP := docker run --platform linux/riscv64 -v `pwd`:/tmp -w /tmp myrisc/alpine:3 strip


all: $(TARGET)

%: %.c
	$(CC) -static $< -o $@
	$(STRIP) $@

clean:
	@rm -rf ./$(TARGET)
