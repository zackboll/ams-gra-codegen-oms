mod common;
const LANGUAGE: &str = "ada";
fn backend() -> ams_gra_oms_backend_ada::AdaBackend {
    ams_gra_oms_backend_ada::AdaBackend
}
include!("../../../tests/task061_carriers.rs");
