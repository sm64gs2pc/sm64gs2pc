pub use sm64gs2pc_core::*;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref DECOMP_DATA_STATIC: DecompData = bincode::deserialize_from(
        &include_bytes!(concat!(env!("OUT_DIR"), "/decomp_data.bincode"))[..]
    )
    .unwrap();
}
