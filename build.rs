extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() -> miette::Result<()> {
    println!("cargo:rustc-link-search={}", env::var("LEAP_ARCH").unwrap());
    println!("cargo:rustc-link-lib=Leap");
    println!("cargo:rustc-link-lib=LeapRust");
    println!("cargo:rerun-if-changed=src/LeapRust.h");

    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("src/LeapRust.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed={}/libLeapRust.so", env::var("LEAP_ARCH").unwrap());
    //Add instructions to link to any C++ libraries you need.
    Ok(())
}
