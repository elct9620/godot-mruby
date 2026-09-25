//! What the editor is told of a Ruby file as it is typed: what the compiler
//! says of the source, and what the project's other files make of the file's
//! name. Nothing runs and nothing waits for a realm, so the language answers
//! from here on whatever thread Godot asks.

use std::ffi::CString;

use crate::announcement::{self, Omission, Project};
use crate::compiler::{self, CompileError};
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
}

impl Kind {
    /// The name the editor lists the warning under.
    pub fn code(self) -> &'static str {
        match self {
            Kind::Compiler => "COMPILER",
            Kind::SharedConstant => "SHARED_CONSTANT",
            Kind::SharedName => "SHARED_NAME",
        }
    }
}

/// What checking `source`, typed as the file at `path` among `files`, finds.
/// The other files are taken as they are, and this one as typed.
pub fn validation<F: Files + Sync>(
    files: &F,
    test_directories: &[String],
    template_directory: &str,
    is_node: &dyn Fn(&str) -> bool,
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
        test_directories,
        template_directory.to_owned(),
        is_node,
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
    Validation {
        error: diagnostics.error,
        warnings: compiled.chain(shared_constant).chain(shared_name).collect(),
    }
}

// The warning that other files name the constant the file at `path` names,
// as the class index warns of them when the realm takes the files in.
fn shared_constant_warning(files: &impl Files, path: &str) -> Option<Warning> {
    let roots = files.roots();
    let key = roots.key_of(path);
    let others: Vec<String> = files
        .paths()
        .into_iter()
        .filter(|other| other != path && roots.key_of(other) == key)
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
            roots.name_of(path)
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

    fn validation_of(
        path: &str,
        source: &str,
        sources: &[(&'static str, &'static str)],
    ) -> Validation {
        let files = Sources(sources.iter().copied().collect());
        let test_directories = ["res://test".to_owned()];
        let is_node = |class: &str| class != "Resource";
        validation(
            &files,
            &test_directories,
            "res://script_templates",
            &is_node,
            path,
            source,
        )
    }

    fn warnings_of(validation: &Validation, kind: Kind) -> Vec<(u32, &str)> {
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

        let checked = validation_of("res://potion.rb", POTION, &sources);

        assert_eq!(
            warnings_of(&checked, Kind::SharedConstant),
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

        let checked = validation_of("res://enemy.rb", ENEMY, &sources);

        assert_eq!(
            warnings_of(&checked, Kind::SharedName),
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

        let checked = validation_of("res://enemy.rb", ENEMY, &sources);

        assert_eq!(warnings_of(&checked, Kind::SharedName).len(), 1);
    }
}
