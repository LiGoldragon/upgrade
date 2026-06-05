use std::{env, path::PathBuf};

use schema_rust_next::build::{GenerationDriver, GenerationPlan};

fn main() {
    SchemaBuild::from_environment().run();
}

struct SchemaBuild {
    crate_root: PathBuf,
}

impl SchemaBuild {
    fn from_environment() -> Self {
        Self {
            crate_root: PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir set")),
        }
    }

    fn run(&self) {
        println!("cargo:rerun-if-changed=schema/lib.schema");
        println!("cargo:rerun-if-changed=src/schema/lib.rs");

        GenerationDriver::new(GenerationPlan::component_runtime_compatibility(
            &self.crate_root,
            "upgrade",
            "0.1.0",
        ))
        .generate()
        .expect("generate upgrade schema artifacts")
        .write_or_check("UPGRADE_UPDATE_SCHEMA_ARTIFACTS")
        .expect("checked-in upgrade schema artifacts are fresh");
    }
}
