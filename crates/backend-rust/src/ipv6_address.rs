//! Standalone renderer; capability integration follows compiler evidence.
pub(super) fn render(name: &str) -> String {
    // The semantic scanner is language-neutral core logic expressed in Rust.
    // Reuse its body, not a classifier, and keep every helper private.
    let model = include_str!("../../codegen-core/src/ipv6_address.rs");
    let start = model.find("impl Ipv6AddressModel {").unwrap();
    let end = model.find("\n#[cfg(test)]").unwrap();
    let scanner = model[start..end]
        .replace("impl Ipv6AddressModel", &format!("impl {name}"))
        .replace("pub fn accepts", "fn accepts");
    format!(
        "#[derive(Clone, Debug, PartialEq, Eq)]\npub struct {name} {{ value: String }}\nimpl {name} {{\n    pub fn new(value: &str) -> Option<Self> {{\n        if !Self::accepts(value.as_bytes()) {{ return None; }}\n        Some(Self {{ value: value.to_owned() }})\n    }}\n    pub fn as_str(&self) -> &str {{ &self.value }}\n}}\n{scanner}"
    )
}

#[cfg(test)]
#[path = "../../../tests/task062_probes.rs"]
mod probes;

#[cfg(test)]
mod tests {
    #[test]
    fn standalone_ipv6_compiler_corpus_and_lifecycle() {
        super::probes::run("rust", &super::render("Address"));
    }
}
