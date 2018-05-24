//! Rust version eth-abi

// #![deny(warnings)]
#![deny(missing_docs)]

extern crate rustc_hex as hex;
extern crate ethereum_types;

use hex::FromHex;
use ethereum_types::{U256};

type Bytes = Vec<u8>;

/// Function parameter type enum
#[derive(Debug, Clone, PartialEq)]
pub enum ParamType {
    /// Address
    Address,
    /// Bytes
    Bytes,
    /// Signed Integer
    Int(usize),
    /// Unsigned Integer
    Uint(usize),
    /// Boolean
    Bool,
    /// String
    String,
    /// Dynamic Array
    Array(Box<ParamType>),
    /// Fixed size Bytes
    FixedBytes(usize),
    /// Fixed size Array
    FixedArray(Box<ParamType>, usize),
    /// Tuple
    Tuple(Vec<ParamType>)
}

impl ParamType {
    /// Parse type from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        if s.ends_with("[]") {
            let subtype = Self::from_str(&s[..(s.len()-2)])?;
            return Ok(ParamType::Array(Box::new(subtype)));
        }
        if s.chars().last() == Some(']') {
            let num = s.chars()
                .rev()
                .skip(1)
                .take_while(|c| *c != '[')
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();
            let len = num.parse::<usize>()
                .map_err(|e| format!("Invalid param type: {}, {:?}", s, e))?;
            let subtype = Self::from_str(&s[..(s.len()- num.len() - 2)])?;
            return Ok(ParamType::FixedArray(Box::new(subtype), len));
        }

        Ok(match s {
            "address" => ParamType::Address,
            "bool" => ParamType::Bool,
            "bytes" => ParamType::Bytes,
            "string" => ParamType::String,
            "int" => ParamType::Int(256),
            "uint" => ParamType::Uint(256),
            s if s.starts_with("int") => {
                let len = s[3..].parse::<usize>()
                    .map_err(|e| format!("Invalid param type: {}, {:?}", s, e))?;
                ParamType::Int(len)
            }
            s if s.starts_with("uint") => {
                let len = s[4..].parse::<usize>()
                    .map_err(|e| format!("Invalid param type: {}, {:?}", s, e))?;
                ParamType::Uint(len)
            }
            s if s.starts_with("bytes") => {
                let len = s[4..].parse::<usize>()
                    .map_err(|e| format!("Invalid param type: {}, {:?}", s, e))?;
                ParamType::FixedBytes(len)
            }
            _ => {
                return Err(format!("Invalid param type: {}", s))
            }
        })
    }

    /// Check if the type is dynamic
    pub fn is_dynamic(&self) -> bool {
        match self {
            ParamType::Bytes | ParamType::String | ParamType::Array(_) => true,
            ParamType::FixedArray(inner, len) if len > &0 => inner.is_dynamic(),
            ParamType::Tuple(inners) if inners.len() > 0 => {
                inners.iter().any(|t| t.is_dynamic())
            },
            _ => false
        }
    }
}


/// Encode a integer value
pub fn encode_int(value_str: &str, _len: u32) -> Bytes {
    let value = if value_str.starts_with("0x") {
        U256::from(value_str[2..].from_hex().unwrap().as_slice())
    } else {
        U256::from_dec_str(value_str).unwrap()
    };
    let mut buf = [0u8; 32];
    value.to_big_endian(&mut buf);
    buf.to_vec()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_param_type() {
        assert_eq!(ParamType::from_str("address"), Ok(ParamType::Address));
        assert_eq!(ParamType::from_str("bool"), Ok(ParamType::Bool));
        assert_eq!(ParamType::from_str("bytes"), Ok(ParamType::Bytes));
        assert_eq!(ParamType::from_str("string"), Ok(ParamType::String));
        assert_eq!(ParamType::from_str("int"), Ok(ParamType::Int(256)));
        assert_eq!(ParamType::from_str("int8"), Ok(ParamType::Int(8)));
        assert_eq!(ParamType::from_str("uint"), Ok(ParamType::Uint(256)));
        assert_eq!(ParamType::from_str("uint128"), Ok(ParamType::Uint(128)));
        assert_eq!(ParamType::from_str("int[]"), Ok(ParamType::Array(Box::new(ParamType::Int(256)))));
        assert_eq!(ParamType::from_str("int[5]"), Ok(ParamType::FixedArray(Box::new(ParamType::Int(256)), 5)));
        assert_eq!(
            ParamType::from_str("int[][]"),
            Ok(ParamType::Array(Box::new(
                ParamType::Array(Box::new(ParamType::Int(256)))
            )))
        );
        assert_eq!(ParamType::from_str("string[]"), Ok(ParamType::Array(Box::new(ParamType::String))));
    }

    #[test]
    fn test_encode_int() {
        let expected = "0000000000000000000000000000000000000000000000000000000000000003"
            .from_hex()
            .unwrap();
        assert_eq!(encode_int("3", 256), expected);
    }
}
