//! Writes the names of each value type's members and methods, as the API
//! the extension is built against lists them, so a value type's class can
//! define them when it is made, with the number of arguments each method
//! requires; the engine lists none of these at runtime.

use std::fmt::Write as _;
use std::path::Path;

use serde_json::Value;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let api: Value = serde_json::from_str(&gdextension_api::version_4_6::load_extension_api_json())
        .expect("gdextension-api ships valid JSON");
    let mut table = String::from("&[\n");
    for class in list_entries(&api["builtin_classes"]) {
        let members: Vec<&str> = list_entries(&class["members"]).map(read_name).collect();
        let methods: Vec<String> = list_entries(&class["methods"])
            .map(|method| {
                let required = list_entries(&method["arguments"])
                    .filter(|argument| argument.get("default_value").is_none())
                    .count();
                let is_static = method["is_static"].as_bool().unwrap_or(false);
                format!(
                    "({:?}, {}, {required}, {is_static})",
                    read_name(method),
                    method["hash"]
                )
            })
            .collect();
        writeln!(
            table,
            "    ({:?}, &{members:?}, &[{}]),",
            read_name(class),
            methods.join(", ")
        )
        .expect("writing to a String succeeds");
    }
    table.push(']');
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    std::fs::write(Path::new(&out).join("value_names.rs"), table).expect("OUT_DIR is writable");
}

fn list_entries(list: &Value) -> impl Iterator<Item = &Value> {
    list.as_array().into_iter().flatten()
}

fn read_name(entry: &Value) -> &str {
    entry["name"].as_str().unwrap_or_default()
}
