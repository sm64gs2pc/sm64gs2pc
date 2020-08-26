use serde::Deserialize;
use serde::Serialize;

pub type SizeInt = u32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    AnonStruct(Struct),
    Struct {
        name: String,
    },
    Array {
        element_type: Box<Type>,
        num_elements: SizeInt,
    },
    Int {
        signed: bool,
        num_bytes: SizeInt,
    },
    Pointer {
        inner_type: Box<Type>,
    },
    Float,
    Ignored,
}

impl Type {
    #[cfg(feature = "loader")]
    pub fn from_clang<'tu>(typ: clang::Type<'tu>) -> Type {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructField {
    pub offset: SizeInt,
    pub name: String,
    pub typ: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Struct {
    pub fields: Vec<StructField>,
}

impl Struct {
    #[cfg(feature = "loader")]
    pub fn from_clang<'tu>(typ: clang::Type<'tu>) -> Self {
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
