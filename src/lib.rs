#![warn(missing_docs)]

//! Tools for converting Super Mario 64 GameShark codes to SM64 PC port patches
//!
//! ```
//! use sm64gs2pc::gameshark;
//!
//! let code = "8133B176 0015".parse::<gameshark::Code>().unwrap();
//! let patch = sm64gs2pc::DECOMP_DATA_STATIC
//!     .gs_code_to_patch("Always have Metal Cap", code)
//!     .unwrap();
//!
//! println!("{}", patch);
//! ```

mod decl;
mod decomp_data;
pub mod gameshark;
mod left_value;
mod typ;

pub use decomp_data::DecompData;

use lazy_static::lazy_static;

lazy_static! {
    /// A pre-compiled `DecompData`
    ///
    /// This is compiled into the crate and is automatically deserialized from
    /// bincode on the first access.
    pub static ref DECOMP_DATA_STATIC: DecompData =
        bincode::deserialize_from(&include_bytes!("decomp_data.bincode")[..]).unwrap();
}
