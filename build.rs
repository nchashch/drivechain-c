use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(cbindgen::Config {
            language: cbindgen::Language::C,
            ..cbindgen::Config::default()
        })
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("./target/bindings.h");
}
