use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("decomp_data.bincode");

    let bytes = async {
        reqwest::get("https://github.com/sm64gs2pc/assets/raw/master/decomp_data.bincode")
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap()
    };
    let bytes = tokio::runtime::Runtime::new().unwrap().block_on(bytes);

    File::create(path).unwrap().write_all(&*bytes).unwrap();
}
