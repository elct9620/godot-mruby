//! The script templates the editor offers for a Ruby script: GDScript's
//! built-in ones for the base classes a node script can extend, in Ruby, and
//! how one is made into the source of a new file.

use crate::realm::{self, Roots};

/// A built-in template, as the editor lists it under its base class.
pub struct Template {
    pub inherit: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// The source, with `_CLASS_`, `_BASE_` and `_TS_` for the file's class,
    /// its superclass and one level of indentation.
    pub content: &'static str,
}

/// GDScript's built-in templates whose base class a node script can extend.
pub const BUILT_INS: [Template; 5] = [
    Template {
        inherit: "Object",
        name: "Empty",
        description: "Empty template suitable for all Objects.",
        content: "class _CLASS_ < _BASE_\nend\n",
    },
    Template {
        inherit: "Node",
        name: "Default",
        description: "Base template for Node with default Godot cycle methods.",
        content: include_str!("template/node_default.rb.template"),
    },
    Template {
        inherit: "CharacterBody2D",
        name: "Basic Movement",
        description: "Classic movement for gravity games (platformer, ...).",
        content: include_str!("template/character_body_2d_basic_movement.rb.template"),
    },
    Template {
        inherit: "CharacterBody3D",
        name: "Basic Movement",
        description: "Classic movement for gravity games (FPS, TPS, ...).",
        content: include_str!("template/character_body_3d_basic_movement.rb.template"),
    },
    Template {
        inherit: "EditorPlugin",
        name: "Plugin",
        description: "Basic plugin template.",
        content: include_str!("template/editor_plugin_plugin.rb.template"),
    },
];

/// The built-in templates listed under `base`.
pub fn built_ins(base: &str) -> impl Iterator<Item = (usize, &'static Template)> {
    BUILT_INS
        .iter()
        .enumerate()
        .filter(move |(_, template)| template.inherit == base)
}

/// The source `content` makes for the file named `file_name`, whose class
/// extends `superclass` as Ruby writes it, indented a level by `indent`.
pub fn source(content: &str, file_name: &str, superclass: &str, indent: &str) -> String {
    content
        .replace("_BASE_", superclass)
        .replace("_CLASS_SNAKE_CASE_", file_name)
        .replace("_CLASS_", &realm::camelize(file_name))
        .replace("_TS_", indent)
}

/// How a class extends the base the editor names: an engine class under
/// `Godot`, or a script's path, quoted, as the constant it spells, written
/// from the top level so a namespace the file opens does not shadow it.
pub fn superclass(base: &str, roots: &Roots) -> String {
    match base
        .strip_prefix('"')
        .and_then(|base| base.strip_suffix('"'))
    {
        Some(path) => format!("::{}", roots.name_of(path)),
        None => format!("Godot::{base}"),
    }
}

/// `source`, made for the file whose constant is `constant`, inside the
/// modules of the namespaces the constant spells, each a level deeper by
/// `indent`, since a template knows only the file's name.
pub fn source_in_namespaces(source: &str, constant: &str, indent: &str) -> String {
    let namespaces: Vec<&str> = constant.split("::").collect();
    let Some((_, namespaces)) = namespaces.split_last() else {
        return source.to_owned();
    };
    namespaces
        .iter()
        .rev()
        .fold(source.to_owned(), |inner, namespace| {
            let indented: Vec<String> = inner
                .trim_end()
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        String::new()
                    } else {
                        format!("{indent}{line}")
                    }
                })
                .collect();
            format!("module {namespace}\n{}\nend\n", indented.join("\n"))
        })
}

#[cfg(test)]
mod tests {
    use super::{BUILT_INS, built_ins, source, source_in_namespaces, superclass};
    use crate::compiler;
    use crate::realm::Roots;

    // @behavior RY-001
    #[test]
    fn a_base_class_offers_the_templates_gdscript_has_for_it() {
        let named = |base: &str| {
            built_ins(base)
                .map(|(_, template)| template.name)
                .collect::<Vec<_>>()
        };

        assert_eq!(named("Object"), ["Empty"]);
        assert_eq!(named("Node"), ["Default"]);
        assert_eq!(named("CharacterBody2D"), ["Basic Movement"]);
        assert_eq!(named("CharacterBody3D"), ["Basic Movement"]);
        assert_eq!(named("EditorPlugin"), ["Plugin"]);
        assert!(named("EditorScript").is_empty());
    }

    // @behavior RY-002
    #[test]
    fn a_template_made_for_a_file_names_its_class_after_the_file() {
        let made = source(BUILT_INS[0].content, "enemy_ship", "Godot::Node", "  ");

        assert!(made.starts_with("class EnemyShip < Godot::Node\n"));
    }

    // @behavior RY-003
    #[test]
    fn a_template_made_on_an_engine_class_extends_it_under_godot() {
        assert_eq!(superclass("Node2D", &Roots::default()), "Godot::Node2D");
    }

    // @behavior RY-004
    #[test]
    fn a_template_made_on_a_ruby_script_extends_that_scripts_class() {
        let roots = Roots::new(["res://src".to_owned()]);

        assert_eq!(
            superclass("\"res://src/enemies/boss.rb\"", &roots),
            "::Enemies::Boss"
        );
    }

    // @behavior RY-005
    #[test]
    fn every_built_in_template_compiles_once_made() {
        for template in &BUILT_INS {
            let base = format!("Godot::{}", template.inherit);
            let made = source(template.content, "made", &base, "\t");

            let checked = compiler::diagnostics(c"made.rb", &made);

            assert!(
                checked.error.is_none(),
                "{} {}:\n{made}",
                template.inherit,
                template.name
            );
        }
    }

    // @behavior RY-006
    #[test]
    fn a_template_indents_as_the_editor_does() {
        let made = source(BUILT_INS[1].content, "made", "Godot::Node", "    ");

        assert!(made.contains("\n    def _ready\n"));
    }

    // @behavior RY-007
    #[test]
    fn a_template_saved_under_a_namespace_opens_the_namespaces_its_path_spells() {
        let made = source(BUILT_INS[1].content, "hero_ship", "Godot::Node", "  ");

        let nested = source_in_namespaces(&made, "Enemies::Ships::HeroShip", "  ");

        assert!(
            nested
                .starts_with("module Enemies\n  module Ships\n    class HeroShip < Godot::Node\n")
        );
        assert!(nested.ends_with("    end\n  end\nend\n"));
        assert!(nested.contains("\n\n      # Called every frame."));
    }

    // @behavior RY-007
    #[test]
    fn a_template_saved_under_a_root_directory_stays_as_it_was_made() {
        let made = source(BUILT_INS[0].content, "hero", "Godot::Node", "  ");

        assert_eq!(source_in_namespaces(&made, "Hero", "  "), made);
    }

    // @behavior RY-007
    #[test]
    fn a_template_ending_in_blank_lines_closes_its_namespaces_right_after_it() {
        let nested =
            source_in_namespaces("class Hero < Godot::Node\nend\n\n", "Enemies::Hero", "\t");

        assert_eq!(
            nested,
            "module Enemies\n\tclass Hero < Godot::Node\n\tend\nend\n"
        );
    }
}
