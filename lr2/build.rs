extern crate cpp_build;

use std::env;

fn main() {
    let include_path = env::var("LEAP_BASE").unwrap() + "/include";
    cpp_build::Config::new().include(include_path).build("src/main.rs");
    println!("cargo:rustc-link-search={}", env::var("LEAP_ARCH").unwrap());
    println!("cargo:rustc-link-lib=Leap");
    //println!("cargo:rustc-link-lib=LeapRust");
}
