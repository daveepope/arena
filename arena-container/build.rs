use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let toml_path = match env::var("ARENA_CONTAINER_DEFAULTS_TOML") {
        Ok(path) => PathBuf::from(path),
        Err(_) => Path::new(&manifest_dir).join("../container_defaults.toml"),
    };
    println!("cargo:rerun-if-changed={}", toml_path.display());

    let raw = fs::read_to_string(&toml_path).expect("read container_defaults.toml");
    let data: toml::Value = toml::from_str(&raw).expect("parse container_defaults.toml");
    let images = data
        .get("image")
        .and_then(toml::Value::as_array)
        .expect("container_defaults.toml must contain [[image]] entries");

    let mut entries: Vec<BTreeMap<String, String>> = images
        .iter()
        .map(|entry| {
            let table = entry.as_table().expect("each [[image]] must be a table");
            let id = table
                .get("id")
                .and_then(toml::Value::as_str)
                .expect("image id");
            let image = table
                .get("image")
                .and_then(toml::Value::as_str)
                .expect("image name");
            let tag = table
                .get("tag")
                .and_then(toml::Value::as_str)
                .expect("image tag");
            BTreeMap::from([
                ("id".to_string(), id.to_string()),
                ("image".to_string(), image.to_string()),
                ("tag".to_string(), tag.to_string()),
            ])
        })
        .collect();
    entries.sort_by(|left, right| left["id"].cmp(&right["id"]));

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let out_path = Path::new(&out_dir).join("default_images.rs");
    fs::write(out_path, render_default_images_rs(&entries)).expect("write default_images.rs");
}

fn render_default_images_rs(entries: &[BTreeMap<String, String>]) -> String {
    let mut const_blocks = Vec::new();
    let mut const_names = Vec::new();
    for entry in entries {
        let const_name = entry["id"].to_ascii_uppercase();
        const_names.push(const_name.clone());
        const_blocks.push(format!(
            "pub const {const_name}: DefaultImageRef = DefaultImageRef {{\n    id: \"{}\",\n    image: \"{}\",\n    tag: \"{}\",\n}};",
            entry["id"], entry["image"], entry["tag"]
        ));
    }
    let const_section = const_blocks.join("\n\n");
    let all_entries = const_names
        .iter()
        .map(|name| name.as_str())
        .collect::<Vec<_>>()
        .join(",\n    ");
    format!(
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
         pub struct DefaultImageRef {{\n\
             pub id: &'static str,\n\
             pub image: &'static str,\n\
             pub tag: &'static str,\n\
         }}\n\
         \n\
         impl DefaultImageRef {{\n\
             pub const fn image_ref(self) -> (&'static str, &'static str) {{\n\
                 (self.image, self.tag)\n\
             }}\n\
         }}\n\
         \n\
         {const_section}\n\
         \n\
         pub const ALL: &[DefaultImageRef] = &[\n\
             {all_entries},\n\
         ];\n"
    )
}
