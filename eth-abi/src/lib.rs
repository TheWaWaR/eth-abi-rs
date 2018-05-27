//! Rust version eth-abi

// #![deny(warnings)]
#![deny(missing_docs)]

extern crate ethereum_types;
extern crate rustc_hex as hex;

use ethereum_types::U256;
use hex::FromHex;

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
    /// TODO: fixed<M>x<N>: Signed fixed-point decimal number
    Fixed(usize, usize),
    /// TODO: Unsigned variant of fixed<M>x<N>
    Ufixed(usize, usize),
    /// String
    String,
    /// Dynamic Array
    Array(Box<ParamType>),
    /// Fixed size Bytes
    FixedBytes(usize),
    /// Fixed size Array
    FixedArray(Box<ParamType>, usize),
    /// Tuple
    Tuple(Vec<ParamType>),
}

impl ParamType {
    /// Parse type from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        if s.ends_with("[]") {
            let subtype = Self::from_str(&s[..(s.len() - 2)])?;
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
            let subtype = Self::from_str(&s[..(s.len() - num.len() - 2)])?;
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
                let len = s[3..]
                    .parse::<usize>()
                    .map_err(|e| format!("Invalid param type: {}, {:?}", s, e))?;
                if len < 8 || len > 256 || len % 8 != 0 {
                    return Err(format!("Invalid param type: {}", s));
                }
                ParamType::Int(len)
            }
            s if s.starts_with("uint") => {
                let len = s[4..]
                    .parse::<usize>()
                    .map_err(|e| format!("Invalid param type: {}, {:?}", s, e))?;
                if len < 8 || len > 256 || len % 8 != 0 {
                    return Err(format!("Invalid param type: {}", s));
                }
                ParamType::Uint(len)
            }
            s if s.starts_with("bytes") => {
                let len = s[4..]
                    .parse::<usize>()
                    .map_err(|e| format!("Invalid param type: {}, {:?}", s, e))?;
                if len <= 0 || len > 32 {
                    return Err(format!("Invalid param type: {}", s));
                }
                ParamType::FixedBytes(len)
            }
            _ => return Err(format!("Invalid param type: {}", s)),
        })
    }

    /// Padded value length
    pub fn value_length(&self, value_str: &str) -> usize {
        32
    }

    /// Check if this param type can be dynamic
    pub fn maybe_dynamic(&self) -> bool {
        match self {
            ParamType::Bytes
            | ParamType::String
            | ParamType::Array(_)
            | ParamType::FixedArray(_, _)
            | ParamType::Tuple(_) => true,
            _ => false,
        }
    }

    /// Check if the type is dynamic
    pub fn is_dynamic(&self) -> bool {
        match self {
            ParamType::Bytes | ParamType::String | ParamType::Array(_) => true,
            ParamType::FixedArray(subtype, len) if *len > 0 => subtype.is_dynamic(),
            ParamType::Tuple(subtypes) if subtypes.len() > 0 => {
                subtypes.iter().any(|t| t.is_dynamic())
            }
            _ => false,
        }
    }
}

enum ParamItem<'a> {
    Fixed {
        param_type: ParamType,
        value_str: &'a str,
    },
    Dynamic {
        offset: Option<usize>,
        param_type: ParamType,
        value_str: &'a str,
    },
}

/// Params
pub struct Params<'a> {
    items: Vec<(ParamType, &'a str)>,
}

impl<'a> Params<'a> {
    /// Encode all params
    pub fn encode(&mut self) -> Result<Bytes, String> {
        let mut total_offset: usize = 0;
        let mut items: Vec<ParamItem> = self.items
            .iter()
            .map(|(param_type, value_str)| match param_type.maybe_dynamic() {
                true => {
                    total_offset += 32;
                    ParamItem::Dynamic {
                        offset: None,
                        param_type: param_type.clone(),
                        value_str: value_str,
                    }
                }
                false => {
                    total_offset += param_type.value_length(value_str);
                    ParamItem::Fixed {
                        param_type: param_type.clone(),
                        value_str: value_str,
                    }
                }
            })
            .collect();

        let mut buf: Vec<u8> = Vec::new();
        while !items.is_empty() {
            let mut next_items: Vec<ParamItem> = Vec::new();
            items.iter_mut().for_each(|item| match item {
                ParamItem::Dynamic {
                    ref mut offset,
                    param_type,
                    value_str,
                } => {
                    *offset = Some(total_offset);
                    total_offset += 32 + param_type.value_length(value_str);
                }
                _ => {}
            });
            items = next_items;
        }
        Ok(buf)
    }
}

fn parse_bytes(value_str: &str) -> (usize, Bytes) {
    let mut value_bytes = if value_str.starts_with("0x") {
        value_str[2..].from_hex().unwrap()
    } else {
        value_str.as_bytes().to_vec()
    };
    let len = value_bytes.len();
    if value_bytes.len() % 32 > 0 {
        let padding_len = 32 - (value_bytes.len() % 32);
        value_bytes.extend(std::iter::repeat(0u8).take(padding_len).collect::<Vec<_>>());
    }
    (len, value_bytes)
}

/// Encode a single value by type
pub fn encode_single(param_type: &ParamType, value_str: &str) -> Result<Bytes, String> {
    match param_type {
        ParamType::Address => {
            let value_bytes = if value_str.starts_with("0x") {
                &value_str[2..]
            } else {
                &value_str[..]
            };
            encode_single(&ParamType::Uint(160), value_bytes)
        }
        ParamType::Uint(m) | ParamType::Int(m) => {
            let mut negative = false;
            let value = if value_str.starts_with("0x") {
                U256::from(value_str[2..].from_hex().unwrap().as_slice())
            } else if value_str.starts_with("-") {
                match param_type {
                    ParamType::Uint(_) => {
                        return Err(format!(
                            "Invalid value={} for type={:?}",
                            value_str, param_type
                        ));
                    }
                    _ => {}
                }
                negative = true;
                U256::from_dec_str(&value_str[1..]).unwrap()
            } else {
                U256::from_dec_str(value_str).unwrap()
            };
            if *m < 256 && value >= U256::from(2).pow(U256::from(*m)) {
                return Err(format!(
                    "Overflow value={}, type={:?}",
                    value_str, param_type
                ));
            }
            let value = if negative {
                (!value) + U256::one()
            } else {
                value
            };
            let mut buf = [0u8; 32];
            value.to_big_endian(&mut buf);
            Ok(buf.to_vec())
        }
        ParamType::Bool => {
            let value_str = match value_str {
                "true" => "1",
                "false" => "0",
                _ => return Err(format!("Invalid value for bool: {}", value_str)),
            };
            Ok(encode_single(&ParamType::Uint(8), value_str)?)
        }
        ParamType::Fixed(m, n) => {
            Ok(vec![])
        }
        ParamType::Ufixed(m, n) => {
            Ok(vec![])
        }
        ParamType::FixedBytes(m) => {
            let (len, value_bytes) = parse_bytes(value_str);
            if len > *m {
                Err(format!("Error value length: value={}", value_str))
            } else {
                Ok(value_bytes)
            }
        }
        ParamType::Bytes => {
            let mut buf: Vec<u8> = Vec::new();
            let (len, value_bytes) = parse_bytes(value_str);
            if len > value_str.chars().count() {
                Err(format!("Value is not bytes: {}", value_str))
            } else {
                // TODO: ugly
                let len_string = format!("{}", len);
                buf.extend(encode_single(&ParamType::Uint(256), len_string.as_str()).unwrap());
                buf.extend(value_bytes);
                Ok(buf)
            }
        }
        ParamType::String => {
            let mut buf: Vec<u8> = Vec::new();
            let (len, value_bytes) = parse_bytes(value_str);
            // TODO: ugly
            let len_string = format!("{}", len);
            buf.extend(encode_single(&ParamType::Uint(256), len_string.as_str()).unwrap());
            buf.extend(value_bytes);
            Ok(buf)
        }
        // ==== Dynamic Types ====
        _ => {
            Err(format!("Cannot encode single dynamic type: {:?}", param_type))
        }
        // ParamType::Array(subtype) => {
        //     // TODO: dynamic
        //     Ok(vec![])
        // }
        // ParamType::FixedArray(subtype, m) => {
        //     // TODO: maybe dynamic
        //     Ok(vec![])
        // }
        // ParamType::Tuple(subtypes) => {
        //     // TODO: maybe dynamic
        //     Ok(vec![])
        // },
    }
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
        assert_eq!(
            ParamType::from_str("int[]"),
            Ok(ParamType::Array(Box::new(ParamType::Int(256))))
        );
        assert_eq!(
            ParamType::from_str("int[5]"),
            Ok(ParamType::FixedArray(Box::new(ParamType::Int(256)), 5))
        );
        assert_eq!(
            ParamType::from_str("int[][]"),
            Ok(ParamType::Array(Box::new(ParamType::Array(Box::new(
                ParamType::Int(256)
            )))))
        );
        assert_eq!(ParamType::from_str("uint"), Ok(ParamType::Uint(256)));
        assert_eq!(ParamType::from_str("uint128"), Ok(ParamType::Uint(128)));
        assert_eq!(
            ParamType::from_str("string[]"),
            Ok(ParamType::Array(Box::new(ParamType::String)))
        );
    }

    #[test]
    fn test_encode_single_int() {
        let expected = "0000000000000000000000000000000000000000000000000000000000000003"
            .from_hex()
            .unwrap();
        let param_type = ParamType::from_str("uint").unwrap();
        assert_eq!(encode_single(&param_type, "3").unwrap(), expected);
        assert_eq!(encode_single(&param_type, "0x03").unwrap(), expected);

        let expected = "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffeb3"
            .from_hex()
            .unwrap();
        let param_type = ParamType::from_str("int").unwrap();
        assert_eq!(encode_single(&param_type, "-333").unwrap(), expected);
        assert_eq!(
            encode_single(
                &param_type,
                "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffeb3"
            ).unwrap(),
            expected
        );
    }

    #[test]
    fn test_encode_single_bool() {
        let expected_false = "0000000000000000000000000000000000000000000000000000000000000000"
            .from_hex()
            .unwrap();
        let expected_true = "0000000000000000000000000000000000000000000000000000000000000001"
            .from_hex()
            .unwrap();
        let param_type = ParamType::from_str("bool").unwrap();
        assert_eq!(encode_single(&param_type, "true").unwrap(), expected_true);
        assert_eq!(encode_single(&param_type, "false").unwrap(), expected_false);
    }
}
