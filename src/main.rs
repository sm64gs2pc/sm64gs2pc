use sm64gs2pc::gameshark;

use std::io::Write;
use std::path::PathBuf;

use structopt::StructOpt;

/// Parsed command-line arguments
#[derive(StructOpt)]
#[structopt(about)]
struct Opts {
    /// Name of GameShark cheat
    #[structopt(long)]
    name: String,

    /// Path to file with GameShark code to convert
    #[structopt(long)]
    code: PathBuf,
}

fn try_main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();

    // Parse GameShark code
    let code = std::fs::read_to_string(opts.code)?.parse::<gameshark::Code>()?;

    // Convert code to patch
    let patch = sm64gs2pc::DECOMP_DATA_STATIC.gs_code_to_patch(&opts.name, code)?;

    // Print patch
    std::io::stdout().write_all(patch.as_bytes())?;

    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("sm64gs2pc: error: {}", err);
    }
}
