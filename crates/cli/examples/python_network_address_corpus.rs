//! Export the unchanged independent Task 062 expectations, not scanner output.
#[path = "../../../tests/task062_cases.rs"]
mod corpus;

fn main() {
    let cases = corpus::cases();
    assert!(cases.len() > 26_000);
    for (text, expected) in &cases {
        assert_eq!(corpus::independent_accepts(text), *expected, "{text:?}");
        assert_eq!(
            ams_gra_oms_codegen_core::Ipv6AddressModel::accepts(text.as_bytes()),
            *expected,
            "{text:?}"
        );
    }
    println!("{}", serde_json::to_string(&cases).unwrap());
}
