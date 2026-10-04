mod common;
const LANGUAGE: &str = "rust";
fn backend() -> ams_gra_oms_backend_rust::RustBackend {
    ams_gra_oms_backend_rust::RustBackend
}
include!("../../../tests/task061_carriers.rs");
