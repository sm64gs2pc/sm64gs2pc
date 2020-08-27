//! Parser for Nintendo 64 GameShark codes.
//!
//! Based on this reference: <https://macrox.gshi.org/The%20Hacking%20Text.htm>
//!
//! ```
//! use sm64gs2pc::gameshark::Code;
//! use sm64gs2pc::gameshark::CodeLine;
//!
//! assert_eq!(
//!     "8129CE9C 2400\n8129CEC0 2400".parse::<Code>().unwrap(),
//!     Code(vec![
//!         CodeLine::Write16 {
//!             addr: 0x0029CE9C,
//!             value: 0x2400,
//!         },
//!         CodeLine::Write16 {
//!             addr: 0x0029CEC0,
//!             value: 0x2400,
//!         },
//!     ])
//! );
//! ```

use crate::typ::SizeInt;

use std::fmt;
use std::str::FromStr;

use snafu::ensure;
use snafu::ResultExt;
use snafu::Snafu;

/// Error parsing a GameShark code
#[derive(Debug, Snafu)]
pub enum ParseError {
    /// Error parsing hex string
    #[snafu(display("GameShark code integer parse: {}", source))]
    ParseIntError {
        /// Error parsing the integer
        source: std::num::ParseIntError,
    },

    /// Error with general code format
    #[snafu(display("GameShark code format error"))]
    FormatError,

    /// Unsupported GameShark code type
    #[snafu(display("Unknown GameShark code type"))]
    CodeTypeError,
}

/// A parsed line of a Nintendo 64 GameShark code
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CodeLine {
    /// 8-bit Write
    ///
    /// ```text
    /// 80XXXXXX 00YY
    /// ```
    ///
    /// Writes `YY` to address `XXXXXX`.
    Write8 {
        /// Address of write `XXXXXX`
        addr: SizeInt,
        /// Written value `YY`
        value: u8,
    },

    /// 16-bit Write
    ///
    /// ```text
    /// 81XXXXXX YYYY
    /// ```
    ///
    /// Writes `YYYY` to address `XXXXXX`.
    Write16 {
        /// Address of write `XXXXXX`
        addr: SizeInt,
        /// Written value `YYYY`
        value: u16,
    },

    /// 8-bit check equal
    ///
    /// ```text
    /// D0XXXXXX 00YY
    /// ZZZZZZZZ ZZZZ
    /// ```
    ///
    /// Execute the code `ZZZZZZZZ ZZZZ` if and only if the value in address
    /// `XXXXXX` is `YY`.
    IfEq8 {
        /// Address of read `XXXXXX`
        addr: SizeInt,
        /// Compared value `YY`
        value: u8,
    },

    /// 16-bit check equal
    ///
    /// ```text
    /// D1XXXXXX YYYY
    /// ZZZZZZZZ ZZZZ
    /// ```
    ///
    /// Execute the code `ZZZZZZZZ ZZZZ` if and only if the value in address
    /// `XXXXXX` is `YYYY`.
    IfEq16 {
        /// Address of read `XXXXXX`
        addr: SizeInt,
        /// Compared value `YYYY`
        value: u16,
    },

    /// 8-bit check unequal
    ///
    /// ```text
    /// D2XXXXXX 00YY
    /// ZZZZZZZZ ZZZZ
    /// ```
    ///
    /// Execute the code `ZZZZZZZZ ZZZZ` if and only if the value in address
    /// `XXXXXX` is *not* `YY`.
    IfNotEq8 {
        /// Address of read `XXXXXX`
        addr: SizeInt,
        /// Compared value `YY`
        value: u8,
    },

    /// 16-bit check unequal
    ///
    /// ```text
    /// D3XXXXXX YYYY
    /// ZZZZZZZZ ZZZZ
    /// ```
    ///
    /// Execute the code `ZZZZZZZZ ZZZZ` if and only if the value in address
    /// `XXXXXX` is *not* `YYYY`.
    IfNotEq16 {
        /// Address of read `XXXXXX`
        addr: SizeInt,
        /// Compared value `YYYY`
        value: u16,
    },
}

impl CodeLine {
    /// Get the address that this code writes to or reads from
    pub fn addr(self) -> SizeInt {
        match self {
            CodeLine::Write8 { addr, .. } => addr,
            CodeLine::Write16 { addr, .. } => addr,
            CodeLine::IfEq8 { addr, .. } => addr,
            CodeLine::IfEq16 { addr, .. } => addr,
            CodeLine::IfNotEq8 { addr, .. } => addr,
            CodeLine::IfNotEq16 { addr, .. } => addr,
        }
    }
}

impl FromStr for CodeLine {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split `TTXXXXXX YYYY` into `TTXXXXXX` and `YYYY`
        let tokens = s.split_whitespace().collect::<Vec<&str>>();
        let (type_addr, value) = if let [type_addr, value] = *tokens.as_slice() {
            Ok((type_addr, value))
        } else {
            Err(ParseError::FormatError)
        }?;

        ensure!(type_addr.len() == 8, FormatError);
        ensure!(value.len() == 4, FormatError);

        // Parse code-type address and value
        let type_addr = SizeInt::from_str_radix(type_addr, 0x10).context(ParseIntError)?;
        let value16 = u16::from_str_radix(value, 0x10).context(ParseIntError)?;
        let value8 = value16 as u8;

        // Extract code type and address
        //
        // Convert `TTXXXXXX` into `TT` and `00XXXXXX`
        let code_type = type_addr >> (8 * 3);
        let addr = type_addr & 0x00FFFFFF;

        match code_type {
            0x80 => Ok(CodeLine::Write8 {
                addr,
                value: value8,
            }),
            0x81 => Ok(CodeLine::Write16 {
                addr,
                value: value16,
            }),
            0xD0 => Ok(CodeLine::IfEq8 {
                addr,
                value: value8,
            }),
            0xD1 => Ok(CodeLine::IfEq16 {
                addr,
                value: value16,
            }),
            0xD2 => Ok(CodeLine::IfNotEq8 {
                addr,
                value: value8,
            }),
            0xD3 => Ok(CodeLine::IfNotEq16 {
                addr,
                value: value16,
            }),
            _ => Err(ParseError::CodeTypeError),
        }
    }
}

impl fmt::Display for CodeLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CodeLine::Write8 { addr, value } => write!(f, "80{:06X} {:04X}", addr, value),
            CodeLine::Write16 { addr, value } => write!(f, "81{:06X} {:04X}", addr, value),
            CodeLine::IfEq8 { addr, value } => write!(f, "D0{:06X} {:04X}", addr, value),
            CodeLine::IfEq16 { addr, value } => write!(f, "D1{:06X} {:04X}", addr, value),
            CodeLine::IfNotEq8 { addr, value } => write!(f, "D2{:06X} {:04X}", addr, value),
            CodeLine::IfNotEq16 { addr, value } => write!(f, "D3{:06X} {:04X}", addr, value),
        }
    }
}

/// A parsed Nintendo 64 GameShark code
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(pub Vec<CodeLine>);

impl FromStr for Code {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let code = s
            .lines()
            // Ignore leading and trailing whitespace
            .map(|line| line.trim())
            // Ignore empty lines
            .filter(|line| !line.is_empty())
            // Parse line
            .map(|line| line.parse::<CodeLine>())
            .collect::<Result<Vec<CodeLine>, Self::Err>>()?;

        Ok(Code(code))
    }
}

/// Size of a value written or read from a GameShark code
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueSize {
    /// 8-Bit value
    Bits8,
    /// 16-Bit value
    Bits16,
}

impl ValueSize {
    /// Amount of bytes of the value
    ///
    /// ```
    /// use sm64gs2pc::gameshark::ValueSize;
    ///
    /// assert_eq!(ValueSize::Bits8.num_bytes(), 1);
    /// assert_eq!(ValueSize::Bits16.num_bytes(), 2);
    /// ```
    pub fn num_bytes(self) -> SizeInt {
        match self {
            ValueSize::Bits8 => 1,
            ValueSize::Bits16 => 2,
        }
    }

    /// Get mask that can be bitwise AND'ed with an integer to isolate the value
    /// size.
    ///
    /// ```
    /// use sm64gs2pc::gameshark::ValueSize;
    ///
    /// assert_eq!(ValueSize::Bits8.mask(), 0xff);
    /// assert_eq!(ValueSize::Bits16.mask(), 0xffff);
    ///
    /// assert_eq!(ValueSize::Bits8.mask() & 0xaabbccdd, 0xdd);
    /// ```
    pub fn mask(self) -> u64 {
        match self {
            ValueSize::Bits8 => 0xff,
            ValueSize::Bits16 => 0xffff,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_code() {
        // Code from:
        // https://sites.google.com/site/sm64gameshark/codes/level-reset-star-select
        let code = "8129CE9C 2400\n\
                    8129CEC0 2400\n\
                    D033AFA1 0020\n\
                    8033B21E 0008\n\
                    \n\
                    D033AFA1  0020  \n\
                    8133B262 0000 \n \
                    D033AFA1 0020\n \
                    8133B218   0000\n\
                    D033AFA1 0020 \n\
                    8033B248  0002\n\
                    D033AFA1 0020 \n\
                    81361414 0005 ";
        assert_eq!(
            code.parse::<Code>().unwrap(),
            Code(vec![
                CodeLine::Write16 {
                    addr: 0x0029CE9C,
                    value: 0x2400,
                },
                CodeLine::Write16 {
                    addr: 0x0029CEC0,
                    value: 0x2400,
                },
                CodeLine::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                CodeLine::Write8 {
                    addr: 0x0033B21E,
                    value: 0x08,
                },
                CodeLine::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                CodeLine::Write16 {
                    addr: 0x0033B262,
                    value: 0x00,
                },
                CodeLine::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                CodeLine::Write16 {
                    addr: 0x0033B218,
                    value: 0x00,
                },
                CodeLine::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                CodeLine::Write8 {
                    addr: 0x0033B248,
                    value: 0x02,
                },
                CodeLine::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                CodeLine::Write16 {
                    addr: 0x00361414,
                    value: 0x05,
                }
            ])
        );
    }
}
