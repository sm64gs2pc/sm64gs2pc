use crate::typ::SizeInt;
use crate::typ::Type;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeclKind {
    Fn,
    Var { typ: Type },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decl {
    pub kind: DeclKind,
    pub name: String,
    pub addr: SizeInt,
}
