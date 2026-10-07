//! What the editor is told of a Ruby file as it is typed: what the compiler
//! says of the source, and what the project's other files make of the file's
//! name. Nothing runs and nothing waits for a realm, so the language answers
//! from here on whatever thread Godot asks.

use std::ffi::CString;

use crate::ancestry;
use crate::announcement::{self, Omission, Project};
use crate::compiler::{self, CompileError};
use crate::header::Header;
use crate::realm::{Declarations, Files, Roots};

/// What checking a file found: the error that stops it compiling, if any,
/// and every warning.
pub struct Validation {
    pub error: Option<CompileError>,
    pub warnings: Vec<Warning>,
}

/// A warning about the file, at the line it names.
pub struct Warning {
    pub line: u32,
    pub kind: Kind,
    pub message: String,
}

/// Where a warning comes from, which the editor lists it under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The compiler warns about the source.
    Compiler,
    /// Another file names the same constant, so neither loads by name.
    SharedConstant,
    /// Another node script shares the class's name, so none is listed by it.
    SharedName,
    /// The class defines a method its engine class binds, which the engine
    /// never calls.
    NativeMethodOverride,
}

impl Kind {
    /// The name the editor lists the warning under.
    pub fn code(self) -> &'static str {
        match self {
            Kind::Compiler => "COMPILER",
            Kind::SharedConstant => "SHARED_CONSTANT",
            Kind::SharedName => "SHARED_NAME",
            Kind::NativeMethodOverride => "NATIVE_METHOD_OVERRIDE",
        }
    }
}

/// What the engine says of its classes, which checking a file asks.
pub struct EngineClasses<'a> {
    /// Whether the engine class of that name is a node class.
    pub is_node: &'a dyn Fn(&str) -> bool,
    /// The engine class binding a method of that name, among the engine
    /// class of the first name and its ancestors.
    pub method_declarer: &'a dyn Fn(&str, &str) -> Option<String>,
}

/// What checking `source`, typed as the file at `path` among `files`, finds.
/// The other files are taken as they are, and this one as typed.
pub fn validation<F: Files + Sync>(
    files: &F,
    test_directories: &[String],
    template_directory: &str,
    engine: &EngineClasses,
    path: &str,
    source: &str,
) -> Validation {
    let name = CString::new(path).unwrap_or_default();
    let diagnostics = compiler::diagnostics(&name, source);
    let typed = DraftFiles {
        files,
        path,
        source,
    };
    let compiled = diagnostics.warnings.into_iter().map(|warning| Warning {
        line: warning.line,
        kind: Kind::Compiler,
        message: warning.message,
    });
    let shared_constant = shared_constant_warning(&typed, path);
    let shared_name = match Project::new(
        &typed,
        test_directories.to_vec(),
        template_directory.to_owned(),
        engine.is_node,
    )
    .announcement(path)
    {
        Err(Omission::SharedName { name, others }) => Some(Warning {
            line: 1,
            kind: Kind::SharedName,
            message: announcement::shared_name_warning(path, &name, &others),
        }),
        _ => None,
    };
    let overrides = native_method_overrides(&typed, engine, path, source);
    Validation {
        error: diagnostics.error,
        warnings: compiled
            .chain(shared_constant)
            .chain(shared_name)
            .chain(overrides)
            .collect(),
    }
}

// A warning at each method a node script's class defines that its engine
// class binds, which the engine calls directly rather than through the script,
// worded as GDScript's NATIVE_METHOD_OVERRIDE.
fn native_method_overrides(
    files: &impl Files,
    engine: &EngineClasses,
    path: &str,
    source: &str,
) -> Vec<Warning> {
    let header = Header::from_source(path, source, &files.roots());
    let Ok(ancestry) = ancestry::trace_ancestry(path, &header, files) else {
        return Vec::new();
    };
    let engine_class = ancestry.engine_class();
    if !(engine.is_node)(engine_class) {
        return Vec::new();
    }
    header
        .method_offsets()
        .filter_map(|(name, offset)| {
            let native = (engine.method_declarer)(engine_class, name)?;
            Some(Warning {
                line: line_at(source, offset),
                kind: Kind::NativeMethodOverride,
                message: format!(
                    "The method \"{name}()\" overrides a method from native class \"{native}\". This won't be called by the engine and may not work as expected."
                ),
            })
        })
        .collect()
}

// The line, counted from 1, that the byte at `offset` of `source` is on.
fn line_at(source: &str, offset: usize) -> u32 {
    let before = source.get(..offset).unwrap_or(source);
    u32::try_from(before.matches('\n').count() + 1).unwrap_or(u32::MAX)
}

// The warning that other files name the constant the file at `path` names,
// as the class index warns of them when the realm takes the files in.
fn shared_constant_warning(files: &impl Files, path: &str) -> Option<Warning> {
    let roots = files.roots();
    let key = roots.key_by_path(path);
    let others: Vec<String> = files
        .paths()
        .into_iter()
        .filter(|other| other != path && roots.key_by_path(other) == key)
        .collect();
    let (all, none) = if others.len() == 1 {
        ("both", "neither")
    } else {
        ("all", "none")
    };
    (!others.is_empty()).then(|| Warning {
        line: 1,
        kind: Kind::SharedConstant,
        message: format!(
            "{path} and {} {all} name {}, so {none} loads by name",
            others.join(" and "),
            roots.name_by_path(path)
        ),
    })
}

// The project's files, with the one being typed read as typed: a file not
// saved yet is among them too.
struct DraftFiles<'a, F> {
    files: &'a F,
    path: &'a str,
    source: &'a str,
}

impl<F: Files + Sync> Files for DraftFiles<'_, F> {
    fn paths(&self) -> Vec<String> {
        let mut paths = self.files.paths();
        if !paths.iter().any(|path| path == self.path) {
            paths.push(self.path.to_owned());
        }
        paths
    }

    fn roots(&self) -> Roots {
        self.files.roots()
    }

    fn source(&self, path: &str) -> Result<String, String> {
        if path == self.path {
            Ok(self.source.to_owned())
        } else {
            self.files.source(path)
        }
    }

    fn declarations(&self, path: &str) -> Declarations {
        self.files.declarations(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ancestry::tests::Sources;

    fn validate_source(
        path: &str,
        source: &str,
        sources: &[(&'static str, &'static str)],
    ) -> Validation {
        let files = Sources(sources.iter().copied().collect());
        let test_directories = ["res://test".to_owned()];
        let is_node = |class: &str| class != "Resource";
        let method_declarer = |class: &str, method: &str| {
            (class == "Node2D" && method == "get_name").then(|| "Node".to_owned())
        };
        let engine = EngineClasses {
            is_node: &is_node,
            method_declarer: &method_declarer,
        };
        validation(
            &files,
            &test_directories,
            "res://script_templates",
            &engine,
            path,
            source,
        )
    }

    fn warnings_by_kind(validation: &Validation, kind: Kind) -> Vec<(u32, &str)> {
        validation
            .warnings
            .iter()
            .filter(|warning| warning.kind == kind)
            .map(|warning| (warning.line, warning.message.as_str()))
            .collect()
    }

    const POTION: &str = "class Potion\nend\n";

    // @behavior RK-004
    #[test]
    fn a_file_naming_what_another_file_names_is_warned_of() {
        let sources = [("res://potion.rb", POTION), ("res://po_tion.rb", POTION)];

        let checked = validate_source("res://potion.rb", POTION, &sources);

        assert_eq!(
            warnings_by_kind(&checked, Kind::SharedConstant),
            vec![(
                1,
                "res://potion.rb and res://po_tion.rb both name Potion, so neither loads by name"
            )]
        );
    }

    const ENEMY: &str = "class Enemy < Godot::Node2D\nend\n";
    const NESTED_ENEMY: &str = "module Bosses\n  class Enemy < Godot::Node2D\n  end\nend\n";

    // @behavior RK-005
    #[test]
    fn a_node_script_sharing_its_name_is_warned_of() {
        let sources = [
            ("res://enemy.rb", ENEMY),
            ("res://bosses/enemy.rb", NESTED_ENEMY),
        ];

        let checked = validate_source("res://enemy.rb", ENEMY, &sources);

        assert_eq!(
            warnings_by_kind(&checked, Kind::SharedName),
            vec![(
                1,
                "res://enemy.rb and res://bosses/enemy.rb define node scripts named Enemy, so none is listed by that name"
            )]
        );
    }

    // @behavior RK-006
    #[test]
    fn a_file_is_checked_as_it_is_typed() {
        let sources = [
            ("res://enemy.rb", "class Enemy\nend\n"),
            ("res://bosses/enemy.rb", NESTED_ENEMY),
        ];

        let checked = validate_source("res://enemy.rb", ENEMY, &sources);

        assert_eq!(warnings_by_kind(&checked, Kind::SharedName).len(), 1);
    }

    // @behavior RK-008
    #[test]
    fn a_node_script_defining_a_method_its_engine_class_binds_is_warned_of() {
        let source =
            "class Enemy < Godot::Node2D\n  def _ready\n  end\n\n  def get_name\n  end\nend\n";
        let sources = [("res://enemy.rb", ENEMY)];

        let checked = validate_source("res://enemy.rb", source, &sources);

        assert_eq!(
            warnings_by_kind(&checked, Kind::NativeMethodOverride),
            vec![(
                5,
                "The method \"get_name()\" overrides a method from native class \"Node\". This won't be called by the engine and may not work as expected."
            )]
        );
    }
}
