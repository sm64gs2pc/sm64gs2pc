use crate::typ::SizeInt;
use crate::typ::Type;

use std::fmt;

#[derive(Debug, Clone)]
pub struct LeftValue {
    pub kind: LeftValueKind,
    pub typ: Type,
    pub addr: SizeInt,
}

#[derive(Debug, Clone)]
pub enum LeftValueKind {
    Ident {
        name: String,
    },
    ArrayIndex {
        array: Box<LeftValue>,
        index: SizeInt,
    },
    StructField {
        struct_: Box<LeftValue>,
        field_name: String,
    },
}

impl fmt::Display for LeftValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.typ == Type::Float {
            write!(f, "*(uint32_t *) &{}", self.kind)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

impl fmt::Display for LeftValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LeftValueKind::Ident { name, .. } => write!(f, "{}", name),
            LeftValueKind::ArrayIndex { array, index, .. } => write!(f, "{}[{}]", array, index),
            LeftValueKind::StructField {
                struct_,
                field_name,
                ..
            } => write!(f, "{}.{}", struct_, field_name),
        }
    }
}
