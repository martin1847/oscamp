# oscamp
Experiments and course for oscamp.

## 进展 

支持`mkfs.fat`

```bash
brew install dosfstools
```


加载配置
```toml
# arceos/Cargo.toml
members = [

    "examples/helloworld",
```

```bash
make ARCH=riscv64 A=examples/helloworld run
```

## 支持宏内核用户程序

```bash
make clean
make
make payload
```