use sm64gs2pc::gameshark;

use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    /// Path to file with GameShark code to convert
    #[structopt(long)]
    code: PathBuf,
}

fn main() {
    let opts = Opts::from_args();

    let codes = std::fs::read_to_string(opts.code)
        .unwrap()
        .parse::<gameshark::Codes>()
        .unwrap();

    let patch = sm64gs2pc::DECOMP_DATA_STATIC
        .gs_codes_to_patch(codes)
        .unwrap();

    println!("{}", patch);
}
