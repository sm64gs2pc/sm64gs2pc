//! Parser for Nintendo 64 GameShark codes.
//!
//! Based on this reference: <https://macrox.gshi.org/The%20Hacking%20Text.htm>
//!
//! ```
//! use sm64gs2pc::gameshark::Code;
//! use sm64gs2pc::gameshark::Codes;
//!
//! assert_eq!(
//!     "8129CE9C 2400\n8129CEC0 2400".parse::<Codes>().unwrap(),
//!     Codes(vec![
//!         Code::Write16 {
//!             addr: 0x0029CE9C,
//!             value: 0x2400,
//!         },
//!         Code::Write16 {
//!             addr: 0x0029CEC0,
//!             value: 0x2400,
//!         },
//!     ])
//! );
//! ```

use crate::SizeInt;

use std::fmt;
use std::str::FromStr;

use snafu::ensure;
use snafu::ResultExt;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum ParseError {
    #[snafu(display("GameShark code integer parse: {}", source))]
    ParseIntError { source: std::num::ParseIntError },

    #[snafu(display("GameShark code format error"))]
    FormatError,

    #[snafu(display("Unknown GameShark code type"))]
    CodeTypeError,
}

/// A parsed Nintendo 64 GameShark code
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
    /// 8-bit Write
    ///
    /// ```text
    /// 80XXXXXX 00YY
    /// ```
    ///
    /// Writes `YY` to address `XXXXXX`.
    Write8 { addr: SizeInt, value: u8 },

    /// 16-bit Write
    ///
    /// ```text
    /// 81XXXXXX YYYY
    /// ```
    ///
    /// Writes `YYYY` to address `XXXXXX`.
    Write16 { addr: SizeInt, value: u16 },

    /// 8-bit check equal
    ///
    /// ```text
    /// D0XXXXXX 00YY
    /// ZZZZZZZZ ZZZZ
    /// ```
    ///
    /// Execute the code `ZZZZZZZZ ZZZZ` if and only if the value in address
    /// `XXXXXX` is `YY`.
    IfEq8 { addr: SizeInt, value: u8 },

    /// 16-bit check equal
    ///
    /// ```text
    /// D1XXXXXX YYYY
    /// ZZZZZZZZ ZZZZ
    /// ```
    ///
    /// Execute the code `ZZZZZZZZ ZZZZ` if and only if the value in address
    /// `XXXXXX` is `YYYY`.
    IfEq16 { addr: SizeInt, value: u16 },

    /// 8-bit check unequal
    ///
    /// ```text
    /// D2XXXXXX 00YY
    /// ZZZZZZZZ ZZZZ
    /// ```
    ///
    /// Execute the code `ZZZZZZZZ ZZZZ` if and only if the value in address
    /// `XXXXXX` is *not* `YY`.
    IfNotEq8 { addr: SizeInt, value: u8 },

    /// 16-bit check unequal
    ///
    /// ```text
    /// D3XXXXXX YYYY
    /// ZZZZZZZZ ZZZZ
    /// ```
    ///
    /// Execute the code `ZZZZZZZZ ZZZZ` if and only if the value in address
    /// `XXXXXX` is *not* `YYYY`.
    IfNotEq16 { addr: SizeInt, value: u16 },
}

impl Code {
    pub fn addr(self) -> SizeInt {
        match self {
            Code::Write8 { addr, .. } => addr,
            Code::Write16 { addr, .. } => addr,
            Code::IfEq8 { addr, .. } => addr,
            Code::IfEq16 { addr, .. } => addr,
            Code::IfNotEq8 { addr, .. } => addr,
            Code::IfNotEq16 { addr, .. } => addr,
        }
    }

    pub fn value(self) -> u16 {
        match self {
            Code::Write8 { value, .. } => value as u16,
            Code::Write16 { value, .. } => value,
            Code::IfEq8 { value, .. } => value as u16,
            Code::IfEq16 { value, .. } => value,
            Code::IfNotEq8 { value, .. } => value as u16,
            Code::IfNotEq16 { value, .. } => value,
        }
    }
}

impl FromStr for Code {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split `TTXXXXXX YYYY` into `TTXXXXXX` and `YYYY`
        let tokens = s.split(' ').collect::<Vec<&str>>();
        let (type_addr, value) = if let &[type_addr, value] = tokens.as_slice() {
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
            0x80 => Ok(Code::Write8 {
                addr,
                value: value8,
            }),
            0x81 => Ok(Code::Write16 {
                addr,
                value: value16,
            }),
            0xD0 => Ok(Code::IfEq8 {
                addr,
                value: value8,
            }),
            0xD1 => Ok(Code::IfEq16 {
                addr,
                value: value16,
            }),
            0xD2 => Ok(Code::IfNotEq8 {
                addr,
                value: value8,
            }),
            0xD3 => Ok(Code::IfNotEq16 {
                addr,
                value: value16,
            }),
            _ => Err(ParseError::CodeTypeError),
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Code::Write8 { addr, value } => write!(f, "80{:06X} {:04X}", addr, value),
            Code::Write16 { addr, value } => write!(f, "81{:06X} {:04X}", addr, value),
            Code::IfEq8 { addr, value } => write!(f, "D0{:06X} {:04X}", addr, value),
            Code::IfEq16 { addr, value } => write!(f, "D1{:06X} {:04X}", addr, value),
            Code::IfNotEq8 { addr, value } => write!(f, "D2{:06X} {:04X}", addr, value),
            Code::IfNotEq16 { addr, value } => write!(f, "D3{:06X} {:04X}", addr, value),
        }
    }
}

/// A list of parsed Nintendo 64 GameShark codes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Codes(pub Vec<Code>);

impl FromStr for Codes {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let codes = s
            .lines()
            .map(|line| line.parse::<Code>())
            .collect::<Result<Vec<Code>, Self::Err>>()?;

        Ok(Codes(codes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_codes() {
        // Code from:
        // https://sites.google.com/site/sm64gameshark/codes/level-reset-star-select
        let codes = "8129CE9C 2400\n\
                     8129CEC0 2400\n\
                     D033AFA1 0020\n\
                     8033B21E 0008\n\
                     D033AFA1 0020\n\
                     8133B262 0000\n\
                     D033AFA1 0020\n\
                     8133B218 0000\n\
                     D033AFA1 0020\n\
                     8033B248 0002\n\
                     D033AFA1 0020\n\
                     81361414 0005";
        assert_eq!(
            codes.parse::<Codes>().unwrap(),
            Codes(vec![
                Code::Write16 {
                    addr: 0x0029CE9C,
                    value: 0x2400,
                },
                Code::Write16 {
                    addr: 0x0029CEC0,
                    value: 0x2400,
                },
                Code::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                Code::Write8 {
                    addr: 0x0033B21E,
                    value: 0x08,
                },
                Code::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                Code::Write16 {
                    addr: 0x0033B262,
                    value: 0x00,
                },
                Code::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                Code::Write16 {
                    addr: 0x0033B218,
                    value: 0x00,
                },
                Code::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                Code::Write8 {
                    addr: 0x0033B248,
                    value: 0x02,
                },
                Code::IfEq8 {
                    addr: 0x0033AFA1,
                    value: 0x20,
                },
                Code::Write16 {
                    addr: 0x00361414,
                    value: 0x05,
                }
            ])
        );
    }
}
