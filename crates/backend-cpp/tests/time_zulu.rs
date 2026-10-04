mod common;
const LANGUAGE: &str = "cpp";
fn backend() -> ams_gra_oms_backend_cpp::CppBackend {
    ams_gra_oms_backend_cpp::CppBackend
}
include!("../../../tests/task061_carriers.rs");
