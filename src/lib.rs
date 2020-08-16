pub mod gameshark;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::process::Command;
use std::process::Stdio;

use walkdir::WalkDir;

type SizeInt = u32;

#[derive(Debug)]
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
    Double,
    Ignored,
}

impl Type {
    fn from_clang<'tu>(typ: clang::Type<'tu>) -> Type {
        match typ.get_kind() {
            clang::TypeKind::Void
            | clang::TypeKind::FunctionPrototype
            | clang::TypeKind::Long
            | clang::TypeKind::IncompleteArray => Type::Ignored,
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
            clang::TypeKind::Double => Type::Double,
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

#[derive(Debug)]
pub struct StructField {
    pub offset: SizeInt,
    pub name: String,
    pub typ: Type,
}

#[derive(Debug)]
pub struct Struct {
    fields: Vec<StructField>,
}

impl Struct {
    fn from_clang<'tu>(typ: clang::Type<'tu>) -> Self {
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

#[derive(Debug)]
pub enum DeclKind {
    Fn,
    Var { typ: Type },
}

#[derive(Debug)]
pub struct Decl {
    pub kind: DeclKind,
    pub name: String,
    pub addr: SizeInt,
}

#[derive(Debug, Default)]
pub struct DecompData {
    pub decls: BTreeMap<SizeInt, Decl>,
    structs: HashMap<String, Struct>,
}

impl DecompData {
    pub fn load() -> Self {
        let decomp_path = std::env::current_dir().unwrap().join("sm64");

        if !decomp_path.exists() {
            assert!(Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg("https://github.com/n64decomp/sm64")
                .arg(&decomp_path)
                .status()
                .unwrap()
                .success());
        }

        let baserom_name = "baserom.us.z64";
        std::fs::copy(baserom_name, decomp_path.join(baserom_name)).unwrap();

        assert!(Command::new("make")
            .current_dir(&decomp_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());

        let mut syms = BTreeMap::<String, SizeInt>::new();

        for entry in WalkDir::new(decomp_path.join("build/us")) {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension() != Some(OsStr::new("map")) {
                continue;
            }
            let file = File::open(path).unwrap();
            let file = BufReader::new(file);
            for line in file.lines() {
                let line = line.unwrap();
                let items = line.split("                ").collect::<Vec<&str>>();
                if let &[empty, addr, sym] = items.as_slice() {
                    if empty != "" {
                        continue;
                    }
                    let addr = match addr.strip_prefix("0x") {
                        Some(addr) => addr,
                        None => continue,
                    };
                    let addr = match SizeInt::from_str_radix(addr, 0x10) {
                        Ok(addr) => addr,
                        Err(_) => continue,
                    };
                    if sym.find(' ').is_some() {
                        continue;
                    }
                    let sym = sym.to_string();
                    syms.insert(sym, addr);
                }
            }
        }

        let mut decomp_data = DecompData::default();

        let ctx = clang::Clang::new().unwrap();
        let index = clang::Index::new(&ctx, false, true);

        for entry in WalkDir::new(&decomp_path) {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.starts_with(decomp_path.join("tools")) {
                continue;
            }
            if path.extension() != Some(OsStr::new("c")) {
                continue;
            }
            let file_name = path.file_name().unwrap().to_str().unwrap();
            if file_name.ends_with(".inc.c")
                || file_name.ends_with("_fr.c")
                || file_name.ends_with("_de.c")
            {
                continue;
            }
            let trans_unit = index
                .parser(path)
                .arguments(&[
                    "-target",
                    "mips64-unknown-unknown",
                    "-m32",
                    "-nostdinc",
                    "-nostdlib",
                    "-fno-builtin",
                    "-DVERSION_US",
                    "-DF3D_OLD",
                    "-DTARGET_N64",
                    "-D_LANGUAGE_C",
                    "-DNON_MATCHING",
                    "-DAVOID_UB",
                    "-fpack-struct",
                    "-I",
                    decomp_path.join("include").to_str().unwrap(),
                    "-I",
                    decomp_path.join("include/libc").to_str().unwrap(),
                    "-I",
                    decomp_path.join("build/us").to_str().unwrap(),
                    "-I",
                    decomp_path.join("build/us/include").to_str().unwrap(),
                    "-I",
                    decomp_path.join("src").to_str().unwrap(),
                    "-I",
                    decomp_path.to_str().unwrap(),
                ])
                .parse()
                .unwrap();

            let entities = trans_unit.get_entity().get_children();

            for entity in &entities {
                let name = match entity.get_name() {
                    Some(name) => name,
                    None => continue,
                };

                let addr = match syms.get(&name) {
                    Some(addr) => *addr,
                    None => continue,
                };

                match entity.get_storage_class() {
                    Some(clang::StorageClass::Extern) => continue,
                    _ => {}
                }

                let kind = match entity.get_kind() {
                    clang::EntityKind::FunctionDecl => DeclKind::Fn,
                    clang::EntityKind::VarDecl => DeclKind::Var {
                        typ: Type::from_clang(entity.get_type().unwrap()),
                    },
                    _ => unimplemented!("clang entity: {:?}", entity),
                };

                let decl = Decl { kind, addr, name };
                decomp_data.decls.insert(addr, decl);
            }

            for decl in clang::sonar::find_structs(entities) {
                let struct_ = Struct::from_clang(decl.entity.get_type().unwrap());
                decomp_data.structs.insert(decl.name, struct_);
            }
        }

        decomp_data
    }

    fn size_of_type(&self, typ: &Type) -> Option<SizeInt> {
        match typ {
            Type::AnonStruct(struct_) => self.size_of_struct(&struct_),
            Type::Struct { name } => {
                let struct_ = self.structs.get(name)?;
                self.size_of_struct(struct_)
            }
            Type::Array {
                element_type,
                num_elements,
            } => self
                .size_of_type(&*element_type)?
                .checked_mul(*num_elements),
            Type::Int { num_bytes, .. } => Some(*num_bytes),
            Type::Pointer { .. } => Some(8),
            Type::Float => Some(4),
            Type::Double => Some(8),
            Type::Ignored => None,
        }
    }

    fn size_of_struct(&self, struct_: &Struct) -> Option<SizeInt> {
        struct_
            .fields
            .iter()
            .map(|field| self.size_of_type(&field.typ))
            .sum()
    }

    pub fn addr_to_lvalue(&self, addr: SizeInt) -> Option<LeftValue> {
        let decl = self.decls.values().rev().find(|decl| decl.addr <= addr)?;

        let typ = match &decl.kind {
            DeclKind::Fn => unimplemented!("function patching"),
            DeclKind::Var { typ } => typ,
        };

        let accum = LeftValue::Ident {
            name: decl.name.clone(),
        };

        self.addr_and_type_to_lvalue(accum, addr, typ, decl.addr)
    }

    fn addr_and_struct_to_lvalue(
        &self,
        accum: LeftValue,
        addr: SizeInt,
        struct_: &Struct,
        type_addr: SizeInt,
    ) -> Option<LeftValue> {
        let field = struct_
            .fields
            .iter()
            .rev()
            .find(|field| type_addr + field.offset <= addr)?;

        let accum = LeftValue::StructField {
            struct_: Box::new(accum),
            field_name: field.name.clone(),
        };

        let type_addr = type_addr + field.offset;

        self.addr_and_type_to_lvalue(accum, addr, &field.typ, type_addr)
    }

    fn addr_and_type_to_lvalue(
        &self,
        accum: LeftValue,
        addr: SizeInt,
        typ: &Type,
        type_addr: SizeInt,
    ) -> Option<LeftValue> {
        match typ {
            Type::AnonStruct(struct_) => {
                self.addr_and_struct_to_lvalue(accum, addr, struct_, type_addr)
            }
            Type::Struct { name } => {
                let struct_ = self.structs.get(name)?;
                self.addr_and_struct_to_lvalue(accum, addr, struct_, type_addr)
            }
            Type::Int { .. } | Type::Pointer { .. } | Type::Float | Type::Double => Some(accum),
            Type::Array {
                element_type,
                num_elements,
            } => {
                let element_type_size = self.size_of_type(element_type)?;
                let index = (addr - type_addr) / element_type_size;

                if index >= *num_elements {
                    return None;
                }

                let accum = LeftValue::ArrayIndex {
                    array: Box::new(accum),
                    index,
                };

                let type_addr = type_addr + index * element_type_size;

                self.addr_and_type_to_lvalue(accum, addr, element_type, type_addr)
            }
            Type::Ignored => unimplemented!("ignored type"),
        }
    }
}

#[derive(Debug)]
pub enum LeftValue {
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
        match self {
            LeftValue::Ident { name } => write!(f, "{}", name),
            LeftValue::ArrayIndex { array, index } => write!(f, "{}[{}]", array, index),
            LeftValue::StructField {
                struct_,
                field_name,
            } => write!(f, "{}.{}", struct_, field_name),
        }
    }
}
