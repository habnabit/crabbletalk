// © 2022 <_@habnab.it>
//
// SPDX-License-Identifier: MPL-2.0

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bindgen::callbacks::ParseCallbacks;

#[derive(Debug, Copy, Clone)]
struct Callbacks;

impl ParseCallbacks for Callbacks {
    fn include_file(&self, filename: &str) {
        bindgen::CargoCallbacks::include_file(&bindgen::CargoCallbacks, filename)
    }

    fn add_derives(&self, _name: &str) -> Vec<String> {
        vec![
            "Default".to_owned(),
            "FromBytes".to_owned(),
            "AsBytes".to_owned(),
        ]
    }
}

fn main() -> Result<()> {
    let bindings = bindgen::Builder::default()
        .header("at.h")
        .parse_callbacks(Box::new(Callbacks))
        .generate()
        .map_err(|()| anyhow::anyhow!("couldn't bindgen parse"))?;

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("at.rs"))
        .context("whilst writing out the bindgen result")?;

    Ok(())
}
