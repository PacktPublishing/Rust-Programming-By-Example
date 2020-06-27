#![no_main]
#[macro_use] extern crate libfuzzer_sys;

mod error {
    include!("../../src/error.rs");
}

include!("../../src/cmd.rs");

fuzz_target!(|data: &[u8]| {
    let _ = Command::new(data.to_vec());
});
