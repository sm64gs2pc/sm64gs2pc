use crate::typ::SizeInt;

use std::fmt;

#[derive(Debug)]
pub struct LeftValue {
    pub kind: LeftValueKind,
    pub size: SizeInt,
    pub addr: SizeInt,
}

#[derive(Debug)]
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
        write!(f, "{}", self.kind)
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
