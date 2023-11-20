extern crate cmake;
use cmake::Config;

// print build script logs
// macro_rules! p {
//     ($($tokens: tt)*) => {
//         println!("cargo:warning={}", format!($($tokens)*))
//     }
// }

fn main() {
    let dst = Config::new("kraken").build_target("kraken_static").build();

    // info
    let profile = std::env::var("PROFILE").unwrap();
    // p!("PROFILE : {}", profile);
    // p!("DST: {}", dst.display());

    // link
    println!(
        "cargo:rustc-link-search=native={}",
        format!("{}/build/bin/CMake/{}", dst.display(), profile)
    );
    println!("cargo:rustc-link-lib=static=kraken_static");
}
