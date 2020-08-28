use serde::Deserialize;
use serde::Serialize;

pub type SizeInt = u32;

/// A C type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    /// An anonymous (unnamed) struct, like `struct { int x }`
    AnonStruct(Struct),

    /// An named struct, like `struct foo`
    Struct {
        /// Name of the struct (`foo`)
        name: String,
    },

    /// An array, like `int foo[10]`
    Array {
        /// Type of each element (`int`)
        element_type: Box<Type>,
        /// Amount of elements in array (`10`)
        num_elements: SizeInt,
    },

    /// An integer, like `uint32_t`
    Int {
        /// Whether the integer is signed
        signed: bool,
        /// Size of integer in bytes
        num_bytes: SizeInt,
    },

    /// A pointer, like `Foo *`
    Pointer {
        /// The inner type (`Foo`)
        inner_type: Box<Type>,
    },

    /// The primitive `float` type
    Float,

    /// Type is ignored by this tool
    Ignored,
}

impl Type {
    /// Convert from a `clang::Type` to a `Type`
    ///
    /// ## Panics
    ///   * The `clang::Type` is unsupported
    ///   * Internal error converting type
    #[cfg(feature = "loader")]
    pub fn from_clang(typ: clang::Type) -> Type {
        match typ.get_kind() {
            clang::TypeKind::Void
            | clang::TypeKind::FunctionPrototype
            | clang::TypeKind::Long
            | clang::TypeKind::IncompleteArray
            | clang::TypeKind::Double => Type::Ignored,
            clang::TypeKind::SChar | clang::TypeKind::CharS => Type::Int {
                signed: true,
                num_bytes: 1,
            },
            clang::TypeKind::UChar => Type::Int {
                signed: false,
                num_bytes: 1,
            },
            clang::TypeKind::Short => Type::Int {
                signed: true,
                num_bytes: 2,
            },
            clang::TypeKind::UShort => Type::Int {
                signed: false,
                num_bytes: 2,
            },
            clang::TypeKind::Int => Type::Int {
                signed: true,
                num_bytes: 4,
            },
            clang::TypeKind::UInt => Type::Int {
                signed: false,
                num_bytes: 4,
            },
            clang::TypeKind::LongLong => Type::Int {
                signed: true,
                num_bytes: 8,
            },
            clang::TypeKind::ULongLong => Type::Int {
                signed: false,
                num_bytes: 8,
            },
            clang::TypeKind::Float => Type::Float,
            clang::TypeKind::Pointer => Type::Pointer {
                inner_type: Box::new(Type::from_clang(typ.get_pointee_type().unwrap())),
            },
            clang::TypeKind::Record => Type::AnonStruct(Struct::from_clang(typ)),
            clang::TypeKind::ConstantArray => Type::Array {
                element_type: Box::new(Type::from_clang(typ.get_element_type().unwrap())),
                num_elements: typ.get_size().unwrap() as SizeInt,
            },
            clang::TypeKind::Typedef => Type::from_clang(
                typ.get_declaration()
                    .unwrap()
                    .get_typedef_underlying_type()
                    .unwrap(),
            ),
            clang::TypeKind::Elaborated => {
                let name = typ.get_declaration().unwrap().get_name();

                match name {
                    Some(name) => Type::Struct { name },
                    None => Type::Ignored,
                }
            }
            _ => unimplemented!("clang type: {:?}, decl: {:?}", typ, typ.get_declaration()),
        }
    }
}

/// A C struct field
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructField {
    /// Amount of bytes between start of struct and this field
    pub offset: SizeInt,
    // Name of field
    pub name: String,
    // Type of field
    pub typ: Type,
}

/// A C struct
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Struct {
    /// Fields of struct
    pub fields: Vec<StructField>,
}

impl Struct {
    /// Convert from a `clang::Type` to a `Struct`
    ///
    /// ## Panics
    ///   * The `clang::Type` is not a struct
    ///   * Internal error converting struct
    #[cfg(feature = "loader")]
    pub fn from_clang(typ: clang::Type) -> Self {
        let fields = typ
            .get_fields()
            .unwrap()
            .into_iter()
            .map(|field| {
                let name = field.get_name().unwrap();
                StructField {
                    offset: typ.get_offsetof(&name).unwrap() as SizeInt / 8,
                    name,
                    typ: Type::from_clang(field.get_type().unwrap()),
                }
            })
            .collect::<Vec<StructField>>();

        Struct { fields }
    }
}
