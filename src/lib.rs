mod decl;
mod decomp_data;
pub mod gameshark;
mod left_value;
mod typ;

pub use decomp_data::DecompData;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref DECOMP_DATA_STATIC: DecompData =
        bincode::deserialize_from(&include_bytes!("decomp_data.bincode")[..]).unwrap();
}
