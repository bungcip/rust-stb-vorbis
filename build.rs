// build.rs

extern crate gcc;

use std::env;

fn main() {
    gcc::compile_library("libstb_vorbis.a", &["src/stb_vorbis.c"]);
    println!("cargo:root={}", env::var("OUT_DIR").unwrap());
}