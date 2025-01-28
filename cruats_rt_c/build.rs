// © 2022 <_@habnab.it>
//
// SPDX-License-Identifier: MPL-2.0

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    cbindgen::generate(crate_dir)
        .expect("couldn't cbindgen")
        .write_to_file("cruats_rt_c.h");
}
