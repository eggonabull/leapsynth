export LEAP_BASE=/home/drew/Downloads/LeapDeveloperKit_2.3.1+31549_linux/LeapSDK
export LLVM_CONFIG_PATH=/usr/bin/llvm-config-14
export LEAP_ARCH=$LEAP_BASE/lib/x64
export LD_LIBRARY_PATH=$LEAP_ARCH
# /home/drew/Downloads/LeapDeveloperKit_2.3.1+31549_linux/LeapSDK/lib/x64
# make -C src/ && cargo build && valgrind --leak-check=full target/debug/leaprust
make -C src/ && cargo build -vv && cargo run