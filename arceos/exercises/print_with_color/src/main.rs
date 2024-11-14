#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // println!("\\e[31m[WithColor]: Hello, Arceos!\\e[0m");
    println!("[WithColor]: Hello, Arceos, Martin !");
}
