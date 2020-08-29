//! Patch conversion with decompilation data

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
use std::iter::once;
#[cfg(feature = "loader")]
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;
use snafu::OptionExt;
use snafu::Snafu;

/// Symbol data from the [Super Mario 64 decompilation][1]
///
/// This information is used for converting GameShark codes to PC port patches.
/// It can be loaded from the decompilation codebase or a pre-compiled version
/// can be accessed at `DECOMP_DATA_STATIC`.
///
/// [1]: https://github.com/n64decomp/sm64
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DecompData {
    decls: BTreeMap<SizeInt, Decl>,
    structs: HashMap<String, Struct>,
}

#[derive(Debug, Clone, Snafu)]
pub enum ToPatchError {
    #[snafu(display(
        "{:#x}: This tool does not support GameShark codes that modify functions, only data",
        addr
    ))]
    FnPatch { addr: SizeInt },

    #[snafu(display("Tried to process ignored or unsupported type"))]
    IgnoredType,

    #[snafu(display("{:#x}: No declaration found for address", addr))]
    NoDecl { addr: SizeInt },

    #[snafu(display("No struct '{}' found", name))]
    NoStruct { name: String },

    #[snafu(display("{:#x}: No struct field found for address", addr))]
    NoField { addr: SizeInt },

    #[snafu(display("{:#x}: Code accesses an array out of bounds: {}", addr, lvalue))]
    ArrayOutOfBounds { addr: SizeInt, lvalue: LeftValue },

    #[snafu(display("{:#x}: Code assigns to a pointer", addr))]
    PointerAssign { addr: SizeInt },
}

impl DecompData {
    /// Load from the SM64 decompilation codebase
    ///
    /// This function:
    /// 1. Clones the SM64 decomp repo from git if not already cloned
    /// 2. Copies the base ROM from `base_rom` into the repo
    /// 3. Compiles the code
    /// 4. Walks the codebase and loads the data
    ///
    /// ## Parameters
    ///   * `base_rom` - Path to a `baserom.us.z64`
    ///   * `repo` - Path where the SM64 decompilation repo should be cloned
    ///
    /// ## Panics
    /// This function panics if any of its operations fail.
    #[cfg(feature = "loader")]
    pub fn load(base_rom: &Path, repo: &Path) -> Self {
        use std::ffi::OsStr;
        use std::fs::File;
        use std::io::BufRead;
        use std::io::BufReader;
        use std::process::Command;

        use walkdir::WalkDir;

        let repo = repo.join("sm64-decomp");

        // Check if SM64 decomp repo already cloned
        if !repo.exists() {
            // Clone SM64 decomp repo
            assert!(Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg("https://github.com/n64decomp/sm64")
                .arg(&repo)
                .status()
                .unwrap()
                .success());
        }

        // Copy ROM into repo
        std::fs::copy(base_rom, repo.join("baserom.us.z64")).unwrap();

        // Compile code
        assert!(Command::new("make")
            .current_dir(&repo)
            .status()
            .unwrap()
            .success());

        // Map from symbol name to address
        let mut syms = BTreeMap::<String, SizeInt>::new();

        // Iterate over `.map` files
        for entry in WalkDir::new(repo.join("build/us")) {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension() != Some(OsStr::new("map")) {
                continue;
            }

            // Iterate over `.map` file lines
            let file = File::open(path).unwrap();
            let file = BufReader::new(file);
            for line in file.lines() {
                let line = line.unwrap();
                let items = line.split("                ").collect::<Vec<&str>>();

                // Load symbol and address
                if let [empty, addr, sym] = *items.as_slice() {
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

        // Iterate over C source files
        for entry in WalkDir::new(&repo) {
            let entry = entry.unwrap();
            let path = entry.path();

            // Ignore tools since they aren't compiled into the ROM
            if path.starts_with(repo.join("tools")) {
                continue;
            }

            // Ignore non-C files
            if path.extension() != Some(OsStr::new("c")) {
                continue;
            }

            // Ignore certain files that have conflicts
            let file_name = path.file_name().unwrap().to_str().unwrap();
            if file_name.ends_with(".inc.c")
                || file_name.ends_with("_fr.c")
                || file_name.ends_with("_de.c")
            {
                continue;
            }

            // Parse C file
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

            // Iterate over entities in C file
            for entity in &entities {
                // Get entity name
                let name = match entity.get_name() {
                    Some(name) => name,
                    None => continue,
                };

                // Get entity address
                let addr = match syms.get(&name) {
                    Some(addr) => *addr,
                    None => continue,
                };

                // Ignore entities declared as `extern` to prevent duplicates
                if let Some(clang::StorageClass::Extern) = entity.get_storage_class() {
                    continue;
                }

                // Load declaration
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

            // Iterate over structs in C file
            for decl in clang::sonar::find_structs(entities) {
                // Load struct
                let struct_ = Struct::from_clang(decl.entity.get_type().unwrap());
                decomp_data.structs.insert(decl.name, struct_);
            }
        }

        decomp_data
    }

    /// Get the size of the type `typ` in bytes
    ///
    /// ## Errors
    /// This function fails if
    ///   * A struct lookup failed
    ///   * The type or one of its inner types is ignored
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

    /// Get the size of the struct `struct_` in bytes
    ///
    /// The struct is assumed to have no padding, because SM64 doesn't seem to
    /// have any struct padding.
    ///
    /// ## Errors
    /// This function fails if
    ///   * The type of a field or one of its inner types is ignored
    fn size_of_struct(&self, struct_: &Struct) -> Result<SizeInt, ToPatchError> {
        struct_
            .fields
            .iter()
            .map(|field| self.size_of_type(&field.typ))
            .sum()
    }

    /// Get the lvalue corresponding to the address
    ///
    /// For example, if `addr` is `0x8033B176`, the lvalue is
    /// `gMarioStates[0].flags`.
    fn addr_to_lvalue(&self, addr: SizeInt) -> Result<LeftValue, ToPatchError> {
        // Get the declaration containing the address
        let decl = self
            .decls
            .values()
            .rev()
            .find(|decl| decl.addr <= addr)
            .context(NoDecl { addr })?;

        // Get the declaration's type
        let typ = match &decl.kind {
            DeclKind::Fn => return Err(ToPatchError::FnPatch { addr }),
            DeclKind::Var { typ } => typ.clone(),
        };

        // Do recursion to accumulate the declaration into an lvalue. For
        // example, the declaration might be an array of structs, so the lvalue
        // should be a field on one of the structs.

        // Initial accumulator, the base declaration
        let accum = LeftValue {
            kind: LeftValueKind::Ident {
                name: decl.name.clone(),
            },
            typ,
            addr: decl.addr,
        };

        self.addr_accum_to_lvalue(accum, addr, decl.addr)
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
            .context(NoField { addr })?;

        let accum_addr = accum_addr + field.offset;

        let accum = LeftValue {
            kind: LeftValueKind::StructField {
                struct_: Box::new(accum),
                field_name: field.name.clone(),
            },
            typ: field.typ.clone(),
            addr: accum_addr,
        };

        self.addr_accum_to_lvalue(accum, addr, accum_addr)
    }

    /// Get the lvalue corresponding to the address, given an initial
    /// accumulator
    fn addr_accum_to_lvalue(
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
                    return Err(ToPatchError::ArrayOutOfBounds {
                        addr,
                        lvalue: accum,
                    });
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

                self.addr_accum_to_lvalue(accum, addr, accum_addr)
            }
            Type::Pointer { .. } => Err(ToPatchError::PointerAssign { addr }),
            Type::Ignored => Err(ToPatchError::IgnoredType),
        }
    }

    /// Convert a GameShark code line to a line of C source code
    fn gs_line_to_c(&self, code: gameshark::CodeLine) -> Result<String, ToPatchError> {
        let addr = code.addr() + 0x80000000;

        let c_source = match code {
            gameshark::CodeLine::Write8 { value, .. } => {
                self.format_write(gameshark::ValueSize::Bits8, value as u64, addr)
            }
            gameshark::CodeLine::Write16 { value, .. } => {
                self.format_write(gameshark::ValueSize::Bits16, value as u64, addr)
            }
            gameshark::CodeLine::IfEq8 { value, .. } => {
                self.format_check(gameshark::ValueSize::Bits8, value as u64, addr, true)
            }
            gameshark::CodeLine::IfEq16 { value, .. } => {
                self.format_check(gameshark::ValueSize::Bits16, value as u64, addr, true)
            }
            gameshark::CodeLine::IfNotEq8 { value, .. } => {
                self.format_check(gameshark::ValueSize::Bits8, value as u64, addr, false)
            }
            gameshark::CodeLine::IfNotEq16 { value, .. } => {
                self.format_check(gameshark::ValueSize::Bits16, value as u64, addr, false)
            }
        }?;

        let c_source = format!("/* {} */ {}", code, c_source);
        Ok(c_source)
    }

    /// Convert GameShark code to a patch in the unified diff format
    ///
    /// ## Parameters
    ///   * `name` - Name of cheat to be included in comment in patch
    ///   * `code` - GameShark code to convert
    pub fn gs_code_to_patch(
        &self,
        name: &str,
        code: gameshark::Code,
    ) -> Result<String, ToPatchError> {
        // Comment with name of cheat
        let name_comment = format!("    /* {} */", name);

        // Added C source code cheat lines
        let cheat_lines = code
            .0
            .into_iter()
            .map(|code_line| {
                // Convert to C and indent
                let line = self.gs_line_to_c(code_line)?;
                let line = format!("    {}", line);
                Ok(line)
            })
            // Have to create owned `String`s since `patch::Line` requires
            // `&str` which needs an owned value to reference
            .collect::<Result<Vec<String>, ToPatchError>>()?;

        // Added C source code cheat `patch::Line`s
        let cheat_lines = cheat_lines.iter().map(|line| patch::Line::Add(line));

        // All lines of patch
        let lines = once(patch::Line::Context("void run_gameshark_cheats(void) {"))
            // Add blank line between cheats
            .chain(once(patch::Line::Add("")))
            // Add comment
            .chain(once(patch::Line::Add(&name_comment)))
            // Add cheat
            .chain(cheat_lines)
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

    /// Create a line of C source code that does a write to an address
    ///
    /// ## Parameters
    ///   * `write_size` - Size of value to write
    ///   * `value` - Value to write
    ///   * `addr` - Address to write value
    fn format_write(
        &self,
        write_size: gameshark::ValueSize,
        value: u64,
        addr: SizeInt,
    ) -> Result<String, ToPatchError> {
        let lvalue = self.addr_to_lvalue(addr)?;

        // Get bit shift amount
        let shift = self.lvalue_get_shift(&lvalue, write_size, addr)?;

        // Update variables and do recursion if the write overlaps multiple
        // lvalues.
        let (
            // Bit shift amount
            shift,
            // Second write to append to output
            next_write,
            // Updated size of value to write
            write_size,
            // Updated value to write
            value,
        ) = match shift {
            // Write is entirely within one lvalue; keep the same variables.
            Some(shift) => (shift, None, write_size, value),

            // Write overlaps multiple lvalues
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

    /// Create a line of C source code that checks the value at an address
    ///
    /// ## Parameters
    ///   * `read_size` - Size of value to read
    ///   * `value` - Value to compare with
    ///   * `addr` - Address to read value from
    ///   * `check_eq` - Whether the operation is `==` or `!=`
    fn format_check(
        &self,
        read_size: gameshark::ValueSize,
        value: u64,
        addr: SizeInt,
        check_eq: bool,
    ) -> Result<String, ToPatchError> {
        let lvalue = self.addr_to_lvalue(addr)?;

        // Get bit shift amount
        let shift = self.lvalue_get_shift(&lvalue, read_size, addr)?;

        // Update variables and do recursion if the read overlaps multiple
        // lvalues.
        let (shift, next_read, read_size, value) = match shift {
            // Read is entirely within one lvalue; keep the same variables.
            Some(shift) => (shift, None, read_size, value),

            // Read overlaps multiple lvalues
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

    /// Get the left bit shift amount required to access a `value_size`d value
    /// at `addr` in `lvalue`
    ///
    /// ## Return values
    ///   * `Ok(Some(shift))` - Success
    ///   * `Ok(None)` - No shift exists, because `value_size` at `addr`
    ///                  overlaps the edge of the lvalue.
    ///   * `Err(err)` - Error getting size of lvalue
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
