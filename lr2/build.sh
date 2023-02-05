export LEAP_BASE=/home/drew/Downloads/LeapDeveloperKit_2.3.1+31549_linux/LeapSDK
export LLVM_CONFIG_PATH=/usr/bin/llvm-config-14
export LEAP_ARCH=$LEAP_BASE/lib/x64
export LD_LIBRARY_PATH=$LEAP_ARCH
export RUSTFLAGS="-Z macro-backtrace"
cargo build && RUST_BACKTRACE=full cargo run