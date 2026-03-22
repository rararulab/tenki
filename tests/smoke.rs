mod common;

#[test]
fn binary_exists() {
    assert!(std::path::Path::new(env!("CARGO_BIN_EXE_tenki")).exists());
}
