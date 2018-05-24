extern crate rustc_hex as hex;
extern crate ethereum_types;

use hex::FromHex;
use ethereum_types::{U256};

type Bytes = Vec<u8>;

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
    fn test_encode_int() {
        let expected = "0000000000000000000000000000000000000000000000000000000000000003"
            .from_hex()
            .unwrap();
        assert_eq!(encode_int("3", 256), expected);
    }
}
