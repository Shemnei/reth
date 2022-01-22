# Building `riscv-tests`

1) Run docker container

```bash
# in `.devpod`
./open-dev-shell.sh
```

2) Build `riscv-tests`

```bash
git clone https://github.com/riscv-software-src/riscv-tests
cd riscv-tests
git submodule update --init --recursive
autoconf
./configure --prefix=$RISCV/target
make
```

3) The compiled `elf` tests are in `riscv-tests/isa`
