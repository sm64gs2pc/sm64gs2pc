//! C declaration types

use crate::typ::SizeInt;
use crate::typ::Type;

use serde::Deserialize;
use serde::Serialize;

/// A kind of C declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeclKind {
    // A function
    Fn,

    // A variable
    Var {
        // Type of the variable
        typ: Type,
    },
}

/// A C declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decl {
    /// The kind of declaration
    pub kind: DeclKind,

    /// Name of declaration (function or variable name)
    pub name: String,

    /// Address of declaration
    pub addr: SizeInt,
}
