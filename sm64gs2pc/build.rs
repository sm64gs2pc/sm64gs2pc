use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let decomp_data = sm64gs2pc_core::DecompData::load(
        &Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("baserom.us.z64"),
        &tempfile::TempDir::new().unwrap().path().join("sm64"),
    );

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("decomp_data.bincode");

    bincode::serialize_into(BufWriter::new(File::create(path).unwrap()), &decomp_data).unwrap();
}
