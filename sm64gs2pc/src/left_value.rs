//! C lvalue types
//!
//! An lvalue (left value) is an expression that can appear on the left side of
//! an assignment in C. For example, in `foo = 2`, `foo` is an lvalue.
//! Similarly, `gMarioStates[0].vel[1]` is an lvalue.

use crate::typ::SizeInt;
use crate::typ::Type;

use std::fmt;

/// A C lvalue
#[derive(Debug, Clone)]
pub struct LeftValue {
    /// Kind of lvalue
    pub kind: LeftValueKind,

    /// Type of lvalue
    pub typ: Type,

    /// Address of lvalue
    pub addr: SizeInt,
}

/// A kind of lvalue
#[derive(Debug, Clone)]
pub enum LeftValueKind {
    /// An identifier expression, like `foo`
    Ident {
        /// Name of identifier (`foo`)
        name: String,
    },

    /// An array index expression, like `foo[0]`
    ArrayIndex {
        /// Lvalue of array (`foo`)
        array: Box<LeftValue>,
        /// Index of array access (`0`)
        index: SizeInt,
    },

    /// A struct field access, like `foo.bar`
    StructField {
        /// Lvalue of struct (`foo`)
        struct_: Box<LeftValue>,
        /// Name of accessed field (`bar`)
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
