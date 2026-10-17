//! What the editor is told of a Ruby file as it is typed: what the compiler
//! says of the source, and what the project's other files make of the file's
//! name. Nothing runs and nothing waits for a realm, so the language answers
//! from here on whatever thread Godot asks.

use std::ffi::CString;

use ruby_prism::{CallNode, DefNode, Node, Visit};

use crate::ancestry;
use crate::announcement::{self, Omission, Project};
use crate::bridge;
use crate::compiler::{self, CompileError};
use crate::header::{Header, locate_line};
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
    /// Another script shares the class's name, so none is listed by it.
    SharedName,
    /// The class defines a method its engine class binds, which the engine
    /// never calls.
    NativeMethodOverride,
    /// A member answering a StringName, which reaches Ruby as a Symbol, is
    /// compared with a String, so the comparison never holds.
    StringNameComparison,
}

impl Kind {
    /// The name the editor lists the warning under.
    pub fn code(self) -> &'static str {
        match self {
            Kind::Compiler => "COMPILER",
            Kind::SharedConstant => "SHARED_CONSTANT",
            Kind::SharedName => "SHARED_NAME",
            Kind::NativeMethodOverride => "NATIVE_METHOD_OVERRIDE",
            Kind::StringNameComparison => "STRING_NAME_COMPARISON",
        }
    }
}

/// What the engine says of its classes, which checking a file asks.
pub struct EngineClasses<'a> {
    /// Whether the engine has a class of that name.
    pub is_engine_class: &'a dyn Fn(&str) -> bool,
    /// The engine class binding a method of that name, among the engine
    /// class of the first name and its ancestors.
    pub method_declarer: &'a dyn Fn(&str, &str) -> Option<String>,
    /// Whether a property or a method of that name answers a StringName,
    /// among the engine class of the first name and its ancestors.
    pub is_string_name_member: &'a dyn Fn(&str, &str) -> bool,
}

/// What checking `source`, typed as the file at `path` among `files`, finds.
/// The other files are taken as they are, and this one as typed.
pub fn validate<F: Files + Sync>(
    files: &F,
    test_directories: &[String],
    template_directory: &str,
    engine: &EngineClasses,
    path: &str,
    source: &str,
) -> Validation {
    let name = CString::new(path).unwrap_or_default();
    let diagnostics = compiler::diagnose(&name, &bridge::rewrite_member_writes(source));
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
    let shared_constant = find_shared_constant_warning(&typed, path);
    let shared_name = match Project::new(
        &typed,
        test_directories.to_vec(),
        template_directory.to_owned(),
        engine.is_engine_class,
    )
    .announcement_by_path(path)
    {
        Err(Omission::SharedName { name, others }) => Some(Warning {
            line: 1,
            kind: Kind::SharedName,
            message: announcement::describe_shared_name(path, &name, &others),
        }),
        _ => None,
    };
    let overrides = collect_native_method_overrides(&typed, engine, path, source);
    let comparisons = collect_string_name_comparisons(&typed, engine, path, source);
    Validation {
        error: diagnostics
            .error
            .or_else(|| find_abstract_error(&typed, path, source)),
        warnings: compiled
            .chain(shared_constant)
            .chain(shared_name)
            .chain(overrides)
            .chain(comparisons)
            .collect(),
    }
}

// The error GDScript's analyzer gives a class that is not abstract yet has
// abstract methods: its own, or ones the abstract classes it extends leave
// to it. Ruby has no such analysis, so the editor is where it shows.
fn find_abstract_error(files: &impl Files, path: &str, source: &str) -> Option<CompileError> {
    let header = Header::from_source(path, source, &files.roots());
    let ancestry = ancestry::trace_ancestry(path, &header, files).ok()?;
    let ancestors = ancestry.files().iter().map(|(_, header)| header);
    let message = describe_abstract_refusal(&header, ancestors)?;
    Some(CompileError {
        line: locate_line(source, header.name_offset()),
        column: 1,
        message,
    })
}

// What refuses the class `header` reads, as GDScript words it, when it is
// not abstract yet has abstract methods: its own, or ones left by the
// abstract classes in `ancestors`, nearest first, up to the first that is
// not abstract.
fn describe_abstract_refusal<'a>(
    header: &Header,
    ancestors: impl IntoIterator<Item = &'a Header>,
) -> Option<String> {
    if header.is_abstract() {
        return None;
    }
    let name = header.name();
    if !header.abstract_methods().is_empty() {
        return Some(format!(
            "Class \"{name}\" is not abstract but contains abstract methods. Mark the class as \"@abstract\" or remove \"@abstract\" from all methods in this class."
        ));
    }
    let mut implemented: Vec<&str> = header.methods().collect();
    for ancestor in ancestors {
        if !ancestor.is_abstract() {
            break;
        }
        let abstract_methods = ancestor.abstract_methods();
        if let Some(method) = abstract_methods
            .iter()
            .find(|method| !implemented.contains(&method.as_str()))
        {
            return Some(format!(
                "Class \"{name}\" must implement \"{}.{method}()\" and other inherited abstract methods or be marked as \"@abstract\".",
                ancestor.name()
            ));
        }
        implemented.extend(
            ancestor
                .methods()
                .filter(|method| !abstract_methods.iter().any(|marked| marked == method)),
        );
    }
    None
}

// A warning at each method a script's class defines that its engine
// class binds, which the engine calls directly rather than through the script,
// worded as GDScript's NATIVE_METHOD_OVERRIDE.
fn collect_native_method_overrides(
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
    if !(engine.is_engine_class)(engine_class) {
        return Vec::new();
    }
    header
        .method_offsets()
        .filter_map(|(name, offset)| {
            let native = (engine.method_declarer)(engine_class, name)?;
            Some(Warning {
                line: locate_line(source, offset),
                kind: Kind::NativeMethodOverride,
                message: format!(
                    "The method \"{name}()\" overrides a method from native class \"{native}\". This won't be called by the engine and may not work as expected."
                ),
            })
        })
        .collect()
}

// A warning at each comparison, in a method the script's class defines, of
// a String with one of its engine class's members answering a StringName,
// which Ruby is given as a Symbol. Only a member called on the object itself
// is known, since nothing tells the class of any other receiver.
fn collect_string_name_comparisons(
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
    if !(engine.is_engine_class)(engine_class) {
        return Vec::new();
    }
    let mut finder = ComparisonFinder {
        is_string_name_member: &|member| (engine.is_string_name_member)(engine_class, member),
        is_in_method: false,
        comparisons: Vec::new(),
    };
    finder.visit(&ruby_prism::parse(source.as_bytes()).node());
    finder
        .comparisons
        .into_iter()
        .map(|(member, offset)| Warning {
            line: locate_line(source, offset),
            kind: Kind::StringNameComparison,
            message: format!(
                "\"{member}\" answers a StringName, which reaches Ruby as a Symbol, so comparing it with a String never holds; compare it with a Symbol."
            ),
        })
        .collect()
}

// Finds, inside instance methods, `==` and `!=` between a String literal and
// a member called on the object itself that answers a StringName.
struct ComparisonFinder<'a> {
    is_string_name_member: &'a dyn Fn(&str) -> bool,
    is_in_method: bool,
    comparisons: Vec<(String, usize)>,
}

impl ComparisonFinder<'_> {
    // The member `node` calls on the object itself, if it is one answering a
    // StringName.
    fn read_string_name_member(&self, node: &Node) -> Option<String> {
        let call = node.as_call_node()?;
        let on_self = call
            .receiver()
            .is_none_or(|receiver| receiver.as_self_node().is_some());
        if !on_self || call.arguments().is_some() {
            return None;
        }
        let member = String::from_utf8_lossy(call.name().as_slice()).into_owned();
        (self.is_string_name_member)(&member).then_some(member)
    }
}

impl<'pr> Visit<'pr> for ComparisonFinder<'_> {
    fn visit_def_node(&mut self, node: &DefNode<'pr>) {
        let was_in_method = self.is_in_method;
        self.is_in_method = node.receiver().is_none();
        ruby_prism::visit_def_node(self, node);
        self.is_in_method = was_in_method;
    }

    fn visit_call_node(&mut self, node: &CallNode<'pr>) {
        let name = node.name().as_slice();
        if self.is_in_method && (name == b"==" || name == b"!=") {
            let argument = node
                .arguments()
                .and_then(|arguments| arguments.arguments().iter().next());
            if let (Some(receiver), Some(argument)) = (node.receiver(), argument) {
                let member = match (receiver.as_string_node(), argument.as_string_node()) {
                    (Some(_), None) => self.read_string_name_member(&argument),
                    (None, Some(_)) => self.read_string_name_member(&receiver),
                    _ => None,
                };
                if let Some(member) = member {
                    self.comparisons
                        .push((member, node.location().start_offset()));
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }
}

// The warning that other files name the constant the file at `path` names,
// as the class index warns of them when the realm takes the files in.
fn find_shared_constant_warning(files: &impl Files, path: &str) -> Option<Warning> {
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

    fn source_by_path(&self, path: &str) -> Result<String, String> {
        if path == self.path {
            Ok(self.source.to_owned())
        } else {
            self.files.source_by_path(path)
        }
    }

    fn declarations_by_path(&self, path: &str) -> Declarations {
        self.files.declarations_by_path(path)
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
        let is_engine_class = |_: &str| true;
        let method_declarer = |class: &str, method: &str| {
            (class == "Node2D" && method == "get_name").then(|| "Node".to_owned())
        };
        let is_string_name_member =
            |class: &str, member: &str| class == "Node2D" && member == "name";
        let engine = EngineClasses {
            is_engine_class: &is_engine_class,
            method_declarer: &method_declarer,
            is_string_name_member: &is_string_name_member,
        };
        validate(
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
                "res://enemy.rb and res://bosses/enemy.rb define scripts named Enemy, so none is listed by that name"
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

    // @behavior RK-009
    #[test]
    fn a_string_name_member_compared_with_a_string_is_warned_of() {
        let source = "class Enemy < Godot::Node2D\n  def player?\n    name == \"Player\"\n  end\n\n  def boss?\n    \"Boss\" != self.name || name == :Boss || label == \"Boss\"\n  end\nend\nname == \"Top\"\n";
        let sources = [("res://enemy.rb", ENEMY)];

        let checked = validate_source("res://enemy.rb", source, &sources);

        let message = "\"name\" answers a StringName, which reaches Ruby as a Symbol, so comparing it with a String never holds; compare it with a Symbol.";
        assert_eq!(
            warnings_by_kind(&checked, Kind::StringNameComparison),
            vec![(3, message), (7, message)]
        );
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

    const SHAPE: &str = "class Shape < Godot::Node2D\n  abstract\n\n  abstract def draw\n  end\n\n  abstract :area\nend\n";

    // @behavior RK-010
    #[test]
    fn a_class_with_abstract_methods_that_is_not_abstract_is_refused() {
        let source = "class Shape < Godot::Node2D\n  abstract def draw\n  end\nend\n";
        let sources = [("res://shape.rb", source)];

        let checked = validate_source("res://shape.rb", source, &sources);

        let error = checked.error.expect("an error");
        assert_eq!(
            (error.line, error.message.as_str()),
            (
                1,
                "Class \"Shape\" is not abstract but contains abstract methods. Mark the class as \"@abstract\" or remove \"@abstract\" from all methods in this class."
            )
        );
    }

    // @behavior RK-011
    #[test]
    fn a_class_leaving_an_inherited_abstract_method_unwritten_is_refused() {
        let source = "class Circle < Shape\n  def draw\n  end\nend\n";
        let sources = [("res://shape.rb", SHAPE), ("res://circle.rb", source)];

        let checked = validate_source("res://circle.rb", source, &sources);

        let error = checked.error.expect("an error");
        assert_eq!(
            (error.line, error.message.as_str()),
            (
                1,
                "Class \"Circle\" must implement \"Shape.area()\" and other inherited abstract methods or be marked as \"@abstract\"."
            )
        );
    }

    // @behavior RK-012
    #[test]
    fn a_class_writing_every_inherited_abstract_method_is_valid() {
        let source = "class Circle < Shape\n  def draw\n  end\n\n  def area\n  end\nend\n";
        let sources = [("res://shape.rb", SHAPE), ("res://circle.rb", source)];

        let checked = validate_source("res://circle.rb", source, &sources);

        assert!(checked.error.is_none());
    }
}
