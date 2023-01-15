export LLVM_CONFIG_PATH=/usr/bin/llvm-config-14
export LD_LIBRARY_PATH=/home/drew/Downloads/LeapDeveloperKit_2.3.1+31549_linux/LeapSDK/lib/x64
# make -C src/ && cargo build && valgrind target/debug/leaprust
make -C src/ && cargo build && cargo run