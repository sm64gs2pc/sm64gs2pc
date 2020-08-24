use crate::decl::Decl;
use crate::decl::DeclKind;
use crate::gameshark;
use crate::left_value::LeftValue;
use crate::left_value::LeftValueKind;
use crate::typ::SizeInt;
use crate::typ::Struct;
use crate::typ::Type;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::iter::once;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use serde::Deserialize;
use serde::Serialize;
use snafu::OptionExt;
use snafu::Snafu;
use walkdir::WalkDir;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DecompData {
    decls: BTreeMap<SizeInt, Decl>,
    structs: HashMap<String, Struct>,
}

#[derive(Debug, Clone, Snafu)]
pub enum ToPatchError {
    #[snafu(display(
        "This tool does not support GameShark codes that modify functions, only data"
    ))]
    FnPatch,

    #[snafu(display("Tried to process ignored or unsupported type"))]
    IgnoredType,

    #[snafu(display("No declaration found for address"))]
    NoDecl,

    #[snafu(display("No struct '{}' found", name))]
    NoStruct { name: String },

    #[snafu(display("No struct field found for address"))]
    NoField,

    #[snafu(display("Code accesses an array out of bounds"))]
    ArrayOutOfBounds,

    #[snafu(display("Code assigns to a pointer"))]
    PointerAssign,
}

impl DecompData {
    pub fn load(base_rom: &Path, repo: &Path) -> Self {
        if !repo.exists() {
            assert!(Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg("https://github.com/n64decomp/sm64")
                .arg(repo)
                .status()
                .unwrap()
                .success());
        }

        let cache_file_path = repo.join("sm64gs2pc.msgpack");

        if cache_file_path.exists() {
            return rmp_serde::decode::from_read(BufReader::new(
                File::open(cache_file_path).unwrap(),
            ))
            .unwrap();
        }

        std::fs::copy(base_rom, repo.join("baserom.us.z64")).unwrap();

        assert!(Command::new("make")
            .current_dir(repo)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());

        let mut syms = BTreeMap::<String, SizeInt>::new();

        for entry in WalkDir::new(repo.join("build/us")) {
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

        for entry in WalkDir::new(repo) {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.starts_with(repo.join("tools")) {
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
                    repo.join("include").to_str().unwrap(),
                    "-I",
                    repo.join("include/libc").to_str().unwrap(),
                    "-I",
                    repo.join("build/us").to_str().unwrap(),
                    "-I",
                    repo.join("build/us/include").to_str().unwrap(),
                    "-I",
                    repo.join("src").to_str().unwrap(),
                    "-I",
                    repo.to_str().unwrap(),
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

        rmp_serde::encode::write(&mut File::create(cache_file_path).unwrap(), &decomp_data)
            .unwrap();

        decomp_data
    }

    fn size_of_type(&self, typ: &Type) -> Result<SizeInt, ToPatchError> {
        match typ {
            Type::AnonStruct(struct_) => self.size_of_struct(&struct_),
            Type::Struct { name } => {
                let struct_ = self.structs.get(name).context(NoStruct { name })?;
                self.size_of_struct(struct_)
            }
            Type::Array {
                element_type,
                num_elements,
            } => self
                .size_of_type(&*element_type)
                .map(|size| size * num_elements),
            Type::Int { num_bytes, .. } => Ok(*num_bytes),
            Type::Pointer { .. } => Ok(8),
            Type::Float => Ok(4),
            Type::Ignored => Err(ToPatchError::IgnoredType),
        }
    }

    fn size_of_struct(&self, struct_: &Struct) -> Result<SizeInt, ToPatchError> {
        struct_
            .fields
            .iter()
            .map(|field| self.size_of_type(&field.typ))
            .sum()
    }

    fn addr_to_lvalue(&self, addr: SizeInt) -> Result<LeftValue, ToPatchError> {
        let decl = self
            .decls
            .values()
            .rev()
            .find(|decl| decl.addr <= addr)
            .context(NoDecl)?;

        let typ = match &decl.kind {
            DeclKind::Fn => return Err(ToPatchError::FnPatch),
            DeclKind::Var { typ } => typ.clone(),
        };

        let accum = LeftValue {
            kind: LeftValueKind::Ident {
                name: decl.name.clone(),
            },
            typ,
            addr: decl.addr,
        };

        self.addr_and_type_to_lvalue(accum, addr, decl.addr)
    }

    fn addr_and_struct_to_lvalue(
        &self,
        accum: LeftValue,
        addr: SizeInt,
        struct_: &Struct,
        accum_addr: SizeInt,
    ) -> Result<LeftValue, ToPatchError> {
        let field = struct_
            .fields
            .iter()
            .rev()
            .find(|field| accum_addr + field.offset <= addr)
            .context(NoField)?;

        let accum_addr = accum_addr + field.offset;

        let accum = LeftValue {
            kind: LeftValueKind::StructField {
                struct_: Box::new(accum),
                field_name: field.name.clone(),
            },
            typ: field.typ.clone(),
            addr: accum_addr,
        };

        self.addr_and_type_to_lvalue(accum, addr, accum_addr)
    }

    fn addr_and_type_to_lvalue(
        &self,
        accum: LeftValue,
        addr: SizeInt,
        accum_addr: SizeInt,
    ) -> Result<LeftValue, ToPatchError> {
        match accum.typ.clone() {
            Type::AnonStruct(struct_) => {
                self.addr_and_struct_to_lvalue(accum, addr, &struct_, accum_addr)
            }
            Type::Struct { name } => {
                let struct_ = self.structs.get(&name).context(NoStruct { name })?;
                self.addr_and_struct_to_lvalue(accum, addr, struct_, accum_addr)
            }
            Type::Int { .. } | Type::Float => Ok(accum),
            Type::Array {
                element_type,
                num_elements,
            } => {
                let element_type_size = self.size_of_type(&element_type)?;
                let index = (addr - accum_addr) / element_type_size;

                if index >= num_elements {
                    return Err(ToPatchError::ArrayOutOfBounds);
                }

                let accum_addr = accum_addr + index * element_type_size;

                let accum = LeftValue {
                    kind: LeftValueKind::ArrayIndex {
                        array: Box::new(accum),
                        index,
                    },
                    typ: *element_type,
                    addr: accum_addr,
                };

                self.addr_and_type_to_lvalue(accum, addr, accum_addr)
            }
            Type::Pointer { .. } => Err(ToPatchError::PointerAssign),
            Type::Ignored => unimplemented!("ignored type"),
        }
    }

    /// Convert a GameShark code to a line of C source code
    fn gs_code_to_c(&self, code: gameshark::Code) -> Result<String, ToPatchError> {
        let addr = code.addr() + 0x80000000;

        let c_source = match code {
            gameshark::Code::Write8 { value, .. } => {
                self.format_write(gameshark::ValueSize::Bits8, value as u64, addr)
            }
            gameshark::Code::Write16 { value, .. } => {
                self.format_write(gameshark::ValueSize::Bits16, value as u64, addr)
            }
            gameshark::Code::IfEq8 { value, .. } => {
                self.format_check(gameshark::ValueSize::Bits8, value as u64, addr, true)
            }
            gameshark::Code::IfEq16 { value, .. } => {
                self.format_check(gameshark::ValueSize::Bits16, value as u64, addr, true)
            }
            gameshark::Code::IfNotEq8 { value, .. } => {
                self.format_check(gameshark::ValueSize::Bits8, value as u64, addr, false)
            }
            gameshark::Code::IfNotEq16 { value, .. } => {
                self.format_check(gameshark::ValueSize::Bits16, value as u64, addr, false)
            }
        }?;

        let c_source = format!("/* {} */ {}", code, c_source);
        Ok(c_source)
    }

    /// Convert GameShark codes to a patch in the unified diff format
    pub fn gs_codes_to_patch(&self, codes: gameshark::Codes) -> Result<String, ToPatchError> {
        // Added C source code lines
        let added_lines = codes
            .0
            .into_iter()
            .map(|code| {
                // Convert to C and indent
                let line = self.gs_code_to_c(code)?;
                let line = format!("    {}", line);
                Ok(line)
            })
            // Have to create owned `String`s since `patch::Line` requires
            // `&str` which needs an owned value to reference
            .collect::<Result<Vec<String>, ToPatchError>>()?;

        // Added C source code `patch::Line`s
        let added_lines = added_lines.iter().map(|line| patch::Line::Add(line));

        // All lines of patch
        let lines = once(patch::Line::Context("void run_gameshark_cheats(void) {"))
            // Add blank line between cheats
            .chain(once(patch::Line::Add("")))
            // Add cheat
            .chain(added_lines)
            // Detect blank line between cheats
            .chain(once(patch::Line::Context("")))
            .collect::<Vec<patch::Line>>();

        let patch = patch::Patch {
            old: patch::File {
                path: Cow::from("a/src/game/gameshark.c"),
                meta: None,
            },
            new: patch::File {
                path: Cow::from("b/src/game/gameshark.c"),
                meta: None,
            },
            hunks: vec![patch::Hunk {
                old_range: patch::Range { start: 4, count: 2 },
                new_range: patch::Range {
                    start: 4,
                    count: lines.len() as u64,
                },
                lines,
            }],
            end_newline: true,
        }
        .to_string();

        Ok(patch)
    }

    fn format_write(
        &self,
        write_size: gameshark::ValueSize,
        value: u64,
        addr: SizeInt,
    ) -> Result<String, ToPatchError> {
        let lvalue = self.addr_to_lvalue(addr)?;

        let shift = self.lvalue_get_shift(&lvalue, write_size, addr)?;

        let (shift, next_write, write_size, value) = match shift {
            Some(shift) => (shift, None, write_size, value),
            None => (
                0,
                Some(self.format_write(gameshark::ValueSize::Bits8, value & 0xff, addr + 1)?),
                gameshark::ValueSize::Bits8,
                value >> 8,
            ),
        };

        let next_write = match next_write {
            Some(s) => format!(" {}", s),
            None => String::new(),
        };

        Ok(format!(
            "{} = ({} & {:#x}) | {:#x};{}",
            lvalue,
            lvalue,
            !(write_size.mask() << shift),
            value << shift,
            next_write
        ))
    }

    fn format_check(
        &self,
        read_size: gameshark::ValueSize,
        value: u64,
        addr: SizeInt,
        check_eq: bool,
    ) -> Result<String, ToPatchError> {
        let lvalue = self.addr_to_lvalue(addr)?;

        let shift = self.lvalue_get_shift(&lvalue, read_size, addr)?;

        let (shift, next_read, read_size, value) = match shift {
            Some(shift) => (shift, None, read_size, value),
            None => (
                0,
                Some(self.format_check(
                    gameshark::ValueSize::Bits8,
                    value & 0xff,
                    addr + 1,
                    check_eq,
                )?),
                gameshark::ValueSize::Bits8,
                value >> 8,
            ),
        };

        let next_read = match next_read {
            Some(s) => format!(" {}", s),
            None => String::new(),
        };

        Ok(format!(
            "if (({} & {:#x}) {} {:#x}){}",
            lvalue,
            read_size.mask() << shift,
            if check_eq { "==" } else { "!=" },
            value << shift,
            next_read,
        ))
    }

    fn lvalue_get_shift(
        &self,
        lvalue: &LeftValue,
        value_size: gameshark::ValueSize,
        addr: SizeInt,
    ) -> Result<Option<SizeInt>, ToPatchError> {
        let lvalue_size = self.size_of_type(&lvalue.typ)?;

        Ok(lvalue_size
            .checked_sub(value_size.num_bytes())
            .and_then(|size_diff| size_diff.checked_sub(addr - lvalue.addr))
            .map(|diff_diff| diff_diff * 8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_int(decomp_data: &mut DecompData, addr: SizeInt, num_bytes: SizeInt, name: &str) {
        decomp_data.decls.insert(
            addr,
            Decl {
                addr,
                kind: DeclKind::Var {
                    typ: Type::Int {
                        signed: false,
                        num_bytes,
                    },
                },
                name: name.to_owned(),
            },
        );
    }

    fn add_float(decomp_data: &mut DecompData, addr: SizeInt, name: &str) {
        decomp_data.decls.insert(
            addr,
            Decl {
                addr,
                kind: DeclKind::Var { typ: Type::Float },
                name: name.to_owned(),
            },
        );
    }

    fn decomp_data() -> DecompData {
        let mut data = DecompData::default();
        add_int(&mut data, 0x8000, 1, "A");
        add_int(&mut data, 0x8001, 1, "B");
        add_int(&mut data, 0x8002, 1, "C");
        add_int(&mut data, 0x8003, 1, "D");
        add_int(&mut data, 0x8004, 4, "E");
        add_int(&mut data, 0x8008, 4, "F");
        add_int(&mut data, 0x800c, 2, "G");
        add_int(&mut data, 0x800e, 2, "H");
        add_float(&mut data, 0x8010, "f0");
        data
    }

    #[test]
    fn test_format_write() {
        let data = decomp_data();

        assert_eq!(
            data.format_write(gameshark::ValueSize::Bits8, 0xaa, 0x8000)
                .unwrap(),
            "A = (A & 0xffffffffffffff00) | 0xaa;"
        );
        assert_eq!(
            data.format_write(gameshark::ValueSize::Bits8, 0xaa, 0x800c)
                .unwrap(),
            "G = (G & 0xffffffffffff00ff) | 0xaa00;"
        );
        assert_eq!(
            data.format_write(gameshark::ValueSize::Bits8, 0xaa, 0x8004)
                .unwrap(),
            "E = (E & 0xffffffff00ffffff) | 0xaa000000;"
        );
        assert_eq!(
            data.format_write(gameshark::ValueSize::Bits8, 0xaa, 0x800d)
                .unwrap(),
            "G = (G & 0xffffffffffffff00) | 0xaa;"
        );
        assert_eq!(
            data.format_write(gameshark::ValueSize::Bits16, 0xabcd, 0x800e)
                .unwrap(),
            "H = (H & 0xffffffffffff0000) | 0xabcd;"
        );

        // Write spans multiple ints
        assert_eq!(
            data.format_write(gameshark::ValueSize::Bits16, 0xabcd, 0x8000)
                .unwrap(),
            "A = (A & 0xffffffffffffff00) | 0xab; B = (B & 0xffffffffffffff00) | 0xcd;"
        );
        assert_eq!(
            data.format_write(gameshark::ValueSize::Bits16, 0xabcd, 0x8003)
                .unwrap(),
            "D = (D & 0xffffffffffffff00) | 0xab; E = (E & 0xffffffff00ffffff) | 0xcd000000;"
        );
        assert_eq!(
            data.format_write(gameshark::ValueSize::Bits16, 0xabcd, 0x8007)
                .unwrap(),
            "E = (E & 0xffffffffffffff00) | 0xab; F = (F & 0xffffffff00ffffff) | 0xcd000000;"
        );

        // Floats
        assert_eq!(
            data.format_write(gameshark::ValueSize::Bits16, 0xabcd, 0x8010)
                .unwrap(),
            "*(uint32_t *) &f0 = (*(uint32_t *) &f0 & 0xffffffff0000ffff) | 0xabcd0000;"
        );
    }

    #[test]
    fn test_format_check() {
        let data = decomp_data();

        assert_eq!(
            data.format_check(gameshark::ValueSize::Bits8, 0xaa, 0x8000, true)
                .unwrap(),
            "if ((A & 0xff) == 0xaa)"
        );
        assert_eq!(
            data.format_check(gameshark::ValueSize::Bits8, 0xaa, 0x800c, true)
                .unwrap(),
            "if ((G & 0xff00) == 0xaa00)"
        );
        assert_eq!(
            data.format_check(gameshark::ValueSize::Bits8, 0xaa, 0x8004, true)
                .unwrap(),
            "if ((E & 0xff000000) == 0xaa000000)"
        );
        assert_eq!(
            data.format_check(gameshark::ValueSize::Bits8, 0xaa, 0x800d, true)
                .unwrap(),
            "if ((G & 0xff) == 0xaa)"
        );
        assert_eq!(
            data.format_check(gameshark::ValueSize::Bits16, 0xabcd, 0x800e, true)
                .unwrap(),
            "if ((H & 0xffff) == 0xabcd)"
        );

        // Check spans multiple ints
        assert_eq!(
            data.format_check(gameshark::ValueSize::Bits16, 0xabcd, 0x8000, true)
                .unwrap(),
            "if ((A & 0xff) == 0xab) if ((B & 0xff) == 0xcd)"
        );
        assert_eq!(
            data.format_check(gameshark::ValueSize::Bits16, 0xabcd, 0x8003, true)
                .unwrap(),
            "if ((D & 0xff) == 0xab) if ((E & 0xff000000) == 0xcd000000)"
        );
        assert_eq!(
            data.format_check(gameshark::ValueSize::Bits16, 0xabcd, 0x8007, true)
                .unwrap(),
            "if ((E & 0xff) == 0xab) if ((F & 0xff000000) == 0xcd000000)"
        );
    }
}
