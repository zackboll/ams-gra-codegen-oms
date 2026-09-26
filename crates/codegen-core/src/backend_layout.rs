//! The single shared description of the model artifacts each backend emits.
//!
//! # Why this exists
//!
//! Each backend derives its generated model **file names** and its enclosing
//! **namespace/package identity** from the schema's namespace URI. Before the
//! Task 047 corrective those rules lived privately in each backend
//! (`model_file_name`, `model_header_name`, `namespace_name`, `package_name`),
//! so nothing else could ask *which files and which host-language unit will
//! type generation produce?* without re-implementing them.
//!
//! The service API preflight must ask exactly that, to decide before any
//! rendering whether the wrapper can be emitted beside the model. A second
//! implementation there would recreate the readiness/generation drift the
//! preflight exists to prevent. So the rules live here, once, and **both**
//! backend rendering and the service API preflight consume them.
//!
//! Every function keeps the exact spelling and the exact error text the
//! backends previously produced, so generated output is byte-for-byte
//! unchanged.
//!
//! # Derivation, stated once
//!
//! The URI is split on every non-ASCII-alphanumeric character and empty
//! components are dropped. Every component is therefore a non-empty run of
//! ASCII letters and digits and **never contains `_`, `-`, or `.`**:
//!
//! | Language | Model unit | Model files |
//! | --- | --- | --- |
//! | Rust | none (the wrapper mounts the file as its own module) | `{last}.rs` |
//! | C++ | `namespace {outer}::{inner}` | `{last}.hpp` |
//! | Ada | `package {Outer}.{Inner}` | `{outer}.ads`, `{outer}-{inner}.ads`, optional `{outer}-{inner}.adb` |
//!
//! `{last}`, `{outer}`, `{inner}` are the lowercased final components; Ada
//! capitalizes only the first character of each package component.

use crate::{BackendLanguage, CodegenError};
use ams_gra_oms_ir::SchemaIr;

/// The non-empty ASCII-alphanumeric components of a namespace URI, in order.
///
/// The one splitter every backend naming rule and the backend-name preflight
/// share.
#[must_use]
pub fn namespace_uri_components(uri: &str) -> Vec<&str> {
    uri.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect()
}

fn first_namespace_uri(schema: &SchemaIr, language: BackendLanguage) -> Result<&str, CodegenError> {
    schema
        .namespaces
        .first()
        .map(|namespace| namespace.uri.as_str())
        .ok_or_else(|| {
            error(format!(
                "{} generation requires one namespace",
                language.name()
            ))
        })
}

/// The last two components, or the backend's historical unsupported error.
fn outer_inner(uri: &str, language: BackendLanguage) -> Result<(&str, &str), CodegenError> {
    match namespace_uri_components(uri).as_slice() {
        [.., outer, inner] => Ok((outer, inner)),
        _ => Err(error(format!(
            "unsupported {} IR construct: namespace URI {uri}",
            language.name()
        ))),
    }
}

/// The lowercased last URI component: the Rust and C++ model file stem.
///
/// The backends historically passed this through their `snake_case`, which
/// on a non-empty ASCII-alphanumeric component is exactly ASCII lowercasing.
fn model_file_stem(schema: &SchemaIr, language: BackendLanguage) -> Result<String, CodegenError> {
    let uri = first_namespace_uri(schema, language)?;
    let stem = namespace_uri_components(uri)
        .last()
        .copied()
        .ok_or_else(|| {
            error(format!(
                "{} generation requires a named namespace",
                language.name()
            ))
        })?;
    Ok(stem.to_ascii_lowercase())
}

/// The file `backend-rust` writes for `schema`'s type model.
///
/// # Errors
///
/// Fails exactly as the Rust backend always has for a schema with no
/// namespace, or whose namespace URI has no alphanumeric component.
pub fn rust_model_file_name(schema: &SchemaIr) -> Result<String, CodegenError> {
    Ok(format!(
        "{}.rs",
        model_file_stem(schema, BackendLanguage::Rust)?
    ))
}

/// The header `backend-cpp` writes for `schema`'s type model.
///
/// # Errors
///
/// Fails exactly as the C++ backend always has for a schema with no
/// namespace, or whose namespace URI has no alphanumeric component.
pub fn cpp_model_header_name(schema: &SchemaIr) -> Result<String, CodegenError> {
    Ok(format!(
        "{}.hpp",
        model_file_stem(schema, BackendLanguage::Cpp)?
    ))
}

/// The two nested C++ namespaces the model header opens, outermost first
/// (`namespace outer::inner { ... }`).
///
/// # Errors
///
/// Fails exactly as the C++ backend always has when the URI has fewer than
/// two alphanumeric components.
pub fn cpp_model_namespace(schema: &SchemaIr) -> Result<[String; 2], CodegenError> {
    let uri = first_namespace_uri(schema, BackendLanguage::Cpp)?;
    let (outer, inner) = outer_inner(uri, BackendLanguage::Cpp)?;
    Ok([outer.to_ascii_lowercase(), inner.to_ascii_lowercase()])
}

/// The Ada model package, parent first (`package Outer.Inner`).
///
/// # Errors
///
/// Fails exactly as the Ada backend always has when the URI has fewer than
/// two alphanumeric components or a component is not an Ada identifier.
pub fn ada_model_package(schema: &SchemaIr) -> Result<[String; 2], CodegenError> {
    let uri = first_namespace_uri(schema, BackendLanguage::Ada)?;
    let (outer, inner) = outer_inner(uri, BackendLanguage::Ada)?;
    Ok([ada_title(outer)?, ada_title(inner)?])
}

/// The Ada model file names; see [`ada_model_file_names`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaModelFiles {
    /// `package Outer is end Outer;`, always written.
    pub parent_spec: String,
    /// `package Outer.Inner is ...`, always written.
    pub spec: String,
    /// `package body Outer.Inner is ...`, written only when a declaration
    /// needs a body.
    pub body: String,
}

/// The files the Ada backend writes for `package`, named by GNAT's default
/// file-naming rule (lowercase, `.` becomes `-`).
#[must_use]
pub fn ada_model_file_names(package: &[String; 2]) -> AdaModelFiles {
    let parent = package[0].to_ascii_lowercase();
    let stem = format!("{parent}-{}", package[1].to_ascii_lowercase());
    AdaModelFiles {
        parent_spec: format!("{parent}.ads"),
        spec: format!("{stem}.ads"),
        body: format!("{stem}.adb"),
    }
}

/// The host-language unit the model is declared in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelUnit {
    /// Rust: the wrapper mounts the model file as a module of its own
    /// choosing, so the model contributes no URI-derived identifier.
    RustFile,
    /// C++: `namespace outer::inner`, outermost first.
    CppNamespace([String; 2]),
    /// Ada: library package `Outer.Inner`, parent first.
    AdaPackage([String; 2]),
}

/// One artifact type generation may write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelArtifact {
    pub relative_path: String,
    /// True when the backend writes it only for some schemas (the Ada body).
    /// Optional artifacts are still reserved by every layout check, so a
    /// verdict never depends on whether this particular schema needs one.
    pub optional: bool,
}

/// The complete model artifact layout one backend produces for one schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendModelLayout {
    pub language: BackendLanguage,
    /// Every model file path, in the order the backend emits them.
    pub artifacts: Vec<ModelArtifact>,
    pub unit: ModelUnit,
}

impl BackendModelLayout {
    /// Derive the layout with exactly the functions the backend renders with.
    ///
    /// # Errors
    ///
    /// Returns the backend's own naming error when the schema namespace
    /// cannot be mapped.
    pub fn for_schema(schema: &SchemaIr, language: BackendLanguage) -> Result<Self, CodegenError> {
        let (artifacts, unit) = match language {
            BackendLanguage::Rust => (
                vec![required(rust_model_file_name(schema)?)],
                ModelUnit::RustFile,
            ),
            BackendLanguage::Cpp => {
                let namespace = cpp_model_namespace(schema)?;
                (
                    vec![required(cpp_model_header_name(schema)?)],
                    ModelUnit::CppNamespace(namespace),
                )
            }
            BackendLanguage::Ada => {
                let package = ada_model_package(schema)?;
                let files = ada_model_file_names(&package);
                (
                    vec![
                        required(files.parent_spec),
                        required(files.spec),
                        ModelArtifact {
                            relative_path: files.body,
                            optional: true,
                        },
                    ],
                    ModelUnit::AdaPackage(package),
                )
            }
        };
        Ok(Self {
            language,
            artifacts,
            unit,
        })
    }
}

const fn required(relative_path: String) -> ModelArtifact {
    ModelArtifact {
        relative_path,
        optional: false,
    }
}

/// The Ada package-component spelling: leading capital, rest verbatim, which
/// must then be an Ada identifier.
fn ada_title(value: &str) -> Result<String, CodegenError> {
    let mut characters = value.chars();
    let first = characters
        .next()
        .ok_or_else(|| error("empty Ada namespace component"))?;
    let title = format!("{}{}", first.to_ascii_uppercase(), characters.as_str());
    let is_identifier = title.as_bytes()[0].is_ascii_alphabetic()
        && title
            .bytes()
            .all(|character| character.is_ascii_alphanumeric() || character == b'_')
        && !title.contains("__")
        && !title.ends_with('_');
    if is_identifier {
        Ok(title)
    } else {
        Err(error(format!(
            "unsupported Ada IR construct: Ada identifier {title:?}"
        )))
    }
}

fn error(message: impl Into<String>) -> CodegenError {
    CodegenError {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::NamespaceDecl;

    fn schema(uri: &str) -> SchemaIr {
        SchemaIr {
            schema_version: None,
            namespaces: vec![NamespaceDecl {
                uri: uri.to_owned(),
                preferred_prefix: None,
            }],
            types: Vec::new(),
            messages: Vec::new(),
        }
    }

    #[test]
    fn real_uci_namespace_maps_to_the_established_layout() {
        let uci = schema("https://www.vdl.afrl.af.mil/programs/oam");
        assert_eq!(rust_model_file_name(&uci).unwrap(), "oam.rs");
        assert_eq!(cpp_model_header_name(&uci).unwrap(), "oam.hpp");
        assert_eq!(cpp_model_namespace(&uci).unwrap(), ["programs", "oam"]);
        assert_eq!(ada_model_package(&uci).unwrap(), ["Programs", "Oam"]);
        assert_eq!(
            BackendModelLayout::for_schema(&uci, BackendLanguage::Ada)
                .unwrap()
                .artifacts,
            vec![
                required("programs.ads".into()),
                required("programs-oam.ads".into()),
                ModelArtifact {
                    relative_path: "programs-oam.adb".into(),
                    optional: true,
                },
            ]
        );
    }

    #[test]
    fn errors_keep_the_backends_historical_text() {
        let no_namespace = SchemaIr {
            namespaces: Vec::new(),
            ..schema("")
        };
        assert_eq!(
            rust_model_file_name(&no_namespace).unwrap_err().message,
            "Rust generation requires one namespace"
        );
        assert_eq!(
            ada_model_package(&no_namespace).unwrap_err().message,
            "Ada generation requires one namespace"
        );
        assert_eq!(
            cpp_model_header_name(&schema("::")).unwrap_err().message,
            "C++ generation requires a named namespace"
        );
        assert_eq!(
            cpp_model_namespace(&schema("urn")).unwrap_err().message,
            "unsupported C++ IR construct: namespace URI urn"
        );
        assert_eq!(
            ada_model_package(&schema("urn:x:1abc"))
                .unwrap_err()
                .message,
            "unsupported Ada IR construct: Ada identifier \"1abc\""
        );
    }

    /// Punctuation never survives into a file stem or a unit component, and
    /// case is folded everywhere except Ada components after the first letter.
    #[test]
    fn components_are_split_on_every_non_alphanumeric_and_folded() {
        let mixed = schema("urn:serviceApi:serviceName");
        assert_eq!(rust_model_file_name(&mixed).unwrap(), "servicename.rs");
        assert_eq!(
            cpp_model_namespace(&mixed).unwrap(),
            ["serviceapi", "servicename"]
        );
        assert_eq!(
            ada_model_package(&mixed).unwrap(),
            ["ServiceApi", "ServiceName"]
        );
        let underscored = schema("urn:service_api:service_name");
        assert_eq!(
            cpp_model_namespace(&underscored).unwrap(),
            ["service", "name"]
        );
        assert_eq!(cpp_model_header_name(&underscored).unwrap(), "name.hpp");
        assert_eq!(
            namespace_uri_components("a.b-c_d:e//f"),
            ["a", "b", "c", "d", "e", "f"]
        );
    }
}
