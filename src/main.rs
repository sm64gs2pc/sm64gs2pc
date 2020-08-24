use sm64gs2pc::gameshark;
use sm64gs2pc::DecompData;

use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    /// Path to Super Mario 64 US ROM
    #[structopt(long)]
    base_rom: PathBuf,

    /// Path where sm64 decompilation repo should be stored
    #[structopt(long)]
    repo: PathBuf,

    /// Path to file with GameShark code to convert
    #[structopt(long)]
    code: PathBuf,
}

fn main() {
    let opts = Opts::from_args();

    let decomp_data = DecompData::load(&opts.base_rom, &opts.repo);
    let codes = std::fs::read_to_string(opts.code)
        .unwrap()
        .parse::<gameshark::Codes>()
        .unwrap();
    let patch = decomp_data.gs_codes_to_patch(codes).unwrap();
    println!("{}", patch);
}
