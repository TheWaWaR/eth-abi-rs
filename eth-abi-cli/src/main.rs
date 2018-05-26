
extern crate clap;
extern crate eth_abi;
extern crate rustc_hex as hex;

use eth_abi::{encode, ParamType};
use hex::ToHex;

fn main() {
    let matches = clap::App::new("eth-abi CLI")
        .arg(
            clap::Arg::with_name("param")
                .long("param")
                .short("p")
                .takes_value(true)
                .multiple(true)
                .number_of_values(2)
                .help("Function parameters")
        )
        .get_matches();
    let mut param_iter = matches.values_of("param").unwrap().peekable();
    while param_iter.peek().is_some() {
        let (type_str, value_str) = (
            param_iter.next().unwrap(),
            param_iter.next().unwrap()
        );
        println!("type={}, value={}", type_str, value_str);
        let param_type = ParamType::from_str(type_str).unwrap();
        let value_string = value_str.replace("~", "-");
        println!("[Value]: {}", encode(&param_type, value_string.as_str()).unwrap().to_hex());
    }
}
