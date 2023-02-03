extern crate cpp_build;

use std::env;
use std::path::PathBuf;

fn main() {
    let include_path = env::var("LEAP_BASE").unwrap() + "/include";
    println!("cargo:rustc-link-search={}", env::var("LEAP_ARCH").unwrap());
    println!("cargo:rustc-link-lib=Leap");
    //println!("cargo:rustc-link-lib=LeapRust");
    //println!("cargo:rerun-if-changed=src/LeapRust.h");
    cpp_build::Config::new().include(include_path).build("src/main.rs");
}
