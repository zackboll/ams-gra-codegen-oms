//! Codec-only synchronous views. Parent model rendering is deliberately untouched.
use ams_gra_oms_codegen_core::{
    ADA_CODEC_VIEWS_CHILD, ADA_CODEC_VISIT_BINARY, ADA_CODEC_VISIT_ELEMENT, AdaCodecViews,
    GeneratedFile, ada_codec_views_files,
};
use std::{fmt::Write as _, path::PathBuf};

pub(crate) fn generate(package: &[String; 2], views: &AdaCodecViews) -> Vec<GeneratedFile> {
    if views.is_empty() {
        return Vec::new();
    }
    let parent = package.join(".");
    let unit = format!("{parent}.{ADA_CODEC_VIEWS_CHILD}");
    let mut spec = format!(
        "package {unit} is\n   --  Synchronous read-only callbacks; no view may escape.\n   --  Do not mutate/finalize the source through another alias during a visit.\n"
    );
    let mut body = format!("package body {unit} is\n");
    for view in &views.sequences {
        let profile = format!(
            "procedure {ADA_CODEC_VISIT_ELEMENT}\n     (Container : {parent}.{}; Index : Standard.Positive;\n      Process : not null access procedure (Element : in {parent}.{}))",
            view.container, view.item
        );
        writeln!(spec, "   {profile};").unwrap();
        writeln!(body, "   {profile} is\n   begin").unwrap();
        if let Some(vectors) = &view.vectors {
            writeln!(
                body,
                "      {parent}.{vectors}.Query_Element (Container.Items, Index, Process);"
            )
            .unwrap();
        } else {
            body.push_str("      if Index > Container.Count then raise Standard.Constraint_Error; end if;\n      Process (Container.Items (Index).Value);\n");
        }
        writeln!(body, "   end {ADA_CODEC_VISIT_ELEMENT};").unwrap();
    }
    for name in &views.binary_carriers {
        let profile = format!(
            "procedure {ADA_CODEC_VISIT_BINARY}\n     (Item : {parent}.{name};\n      Process : not null access procedure (Octets : in {parent}.Binary_Vectors.Vector))"
        );
        writeln!(spec, "   {profile};").unwrap();
        writeln!(body, "   {profile} is\n   begin\n      Process (Item.Data);\n   end {ADA_CODEC_VISIT_BINARY};").unwrap();
    }
    writeln!(spec, "end {unit};").unwrap();
    writeln!(body, "end {unit};").unwrap();
    let [spec_path, body_path] = ada_codec_views_files(package);
    vec![
        GeneratedFile {
            relative_path: PathBuf::from(spec_path),
            contents: spec,
        },
        GeneratedFile {
            relative_path: PathBuf::from(body_path),
            contents: body,
        },
    ]
}
