//! The names the script editor offers where its caret stands in a Ruby file,
//! read from the tokens Prism lexes and never by running the file, so they
//! are answered without the realm.

use std::collections::HashSet;
use std::sync::Arc;

use ruby_prism_sys::{
    PM_TOKEN_AMPERSAND_DOT, PM_TOKEN_COLON_COLON, PM_TOKEN_CONSTANT, PM_TOKEN_DOT,
    PM_TOKEN_IDENTIFIER, PM_TOKEN_KEYWORD_SELF, PM_TOKEN_METHOD_NAME, PM_TOKEN_UCOLON_COLON,
};

use crate::ancestry::{self, Lineage};
use crate::header::Header;
use crate::lexer::{Token, lex};
use crate::realm::{Definition, Files};
use crate::snapshot::Snapshot;

/// The character the editor writes where its caret stands in the code it
/// asks about.
pub const CARET: char = '\u{FFFF}';

/// What a name the editor offers stands for, which decides its icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Class,
    /// A method, with whether it takes arguments when that is known.
    Method {
        has_arguments: Option<bool>,
    },
    Property,
    Constant,
}

/// Where a name the editor offers is defined, which orders the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// The file being typed or a file its class inherits from.
    Script,
    /// Another file of the project.
    Project,
    Engine,
}

/// A name the editor may offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub kind: Kind,
    pub origin: Origin,
    pub name: String,
}

impl Candidate {
    /// The text the editor lists: a method shows whether it takes arguments,
    /// as Godot lists one.
    pub fn display(&self) -> String {
        match self.kind {
            Kind::Method {
                has_arguments: Some(true),
            } => format!("{}(…)", self.name),
            Kind::Method {
                has_arguments: Some(false),
            } => format!("{}()", self.name),
            _ => self.name.clone(),
        }
    }

    /// The text the editor writes: a method taking arguments opens its
    /// parentheses, and one taking none is called by its name alone, as Ruby
    /// writes it.
    pub fn insert_text(&self) -> String {
        match self.kind {
            Kind::Method {
                has_arguments: Some(true),
            } => format!("{}(", self.name),
            _ => self.name.clone(),
        }
    }
}

/// What the engine names, which completing a name asks.
pub struct EngineNames<'a> {
    /// The names of the classes under `Godot`: the engine's classes and its
    /// value types.
    pub classes: &'a dyn Fn() -> Vec<String>,
    /// The methods, properties and integer constants of the engine class of
    /// that name, its ancestors' included.
    pub members: &'a dyn Fn(&str) -> Vec<Candidate>,
}

/// What the file being typed names, read from its source and those of the
/// files its class inherits from.
#[derive(Debug, Default)]
pub struct ScriptNames {
    /// The class under `Godot` its class extends, if any.
    pub engine_class: Option<String>,
    /// The methods its class and the classes it inherits from define.
    pub methods: Vec<String>,
    /// The properties they export.
    pub properties: Vec<String>,
    /// The constants the class index names inside the namespaces the file is
    /// named in, the top level included.
    pub constants: Vec<String>,
}

/// What the file at `path`, typed as `code`, names among `files`: read from
/// the source as typed, from the files its class inherits from as they are,
/// and from what `snapshot` says their classes defined as they ran.
pub fn read_script_names(
    files: &impl Files,
    snapshot: &Snapshot,
    path: &str,
    code: &str,
) -> ScriptNames {
    let source = code.replace(CARET, "");
    let header = Header::from_source(path, &source, &files.roots());
    let ancestry = ancestry::trace_ancestry(path, &header, files)
        .ok()
        .map(Arc::new);
    let engine_class = ancestry
        .as_ref()
        .map(|ancestry| ancestry.engine_class().to_owned());
    let lineage = Lineage::new(path.to_owned(), &header, ancestry);
    ScriptNames {
        engine_class,
        methods: lineage
            .collect_methods(snapshot)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        properties: lineage
            .collect_properties(snapshot)
            .into_iter()
            .map(|property| property.name)
            .collect(),
        constants: collect_constants(files, path)
            .into_iter()
            .map(|(name, _)| name)
            .collect(),
    }
}

/// The constants a name written in the file at `path` reaches without a
/// namespace, each with what defines it: named as the file defining it
/// spells it, or as its directory's module is named.
pub fn collect_constants(files: &impl Files, path: &str) -> Vec<(String, Definition)> {
    let roots = files.roots();
    files
        .collect_definitions_around(path)
        .into_iter()
        .map(|definition| {
            let name = match &definition {
                Definition::Namespace(namespace) => namespace.name.clone(),
                Definition::File(file) => files
                    .source_by_path(file)
                    .map(|source| Header::from_source(file, &source, &roots).name().to_owned())
                    .ok()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| roots.name_by_path(file)),
            };
            (name, definition)
        })
        .collect()
}

// What the caret stands after, which decides the names Ruby resolves there.
enum Context {
    UnderGodot,
    OnSelf,
    Bare,
    Elsewhere,
}

/// The names that may complete the one the caret stands in within `code`;
/// the editor narrows them by what is typed. A name the file and the engine
/// both define is offered once, as the file's.
pub fn collect_candidates(
    code: &str,
    script: &ScriptNames,
    engine: &EngineNames,
) -> Vec<Candidate> {
    let candidates = match read_context(code) {
        Context::UnderGodot => (engine.classes)()
            .into_iter()
            .map(|name| Candidate {
                kind: Kind::Class,
                origin: Origin::Engine,
                name,
            })
            .collect(),
        Context::OnSelf => collect_members(script, engine)
            .into_iter()
            .filter(|candidate| candidate.kind != Kind::Constant)
            .collect(),
        Context::Bare => {
            let mut candidates = collect_members(script, engine);
            candidates.extend(script.constants.iter().map(|name| Candidate {
                kind: Kind::Constant,
                origin: Origin::Project,
                name: name.clone(),
            }));
            candidates
        }
        Context::Elsewhere => Vec::new(),
    };
    let mut names = HashSet::new();
    candidates
        .into_iter()
        .filter(|candidate| names.insert(candidate.name.clone()))
        .collect()
}

// What an object of the file's class answers: its own methods and exports,
// then its engine class's members.
fn collect_members(script: &ScriptNames, engine: &EngineNames) -> Vec<Candidate> {
    let methods = script.methods.iter().map(|name| Candidate {
        kind: Kind::Method {
            has_arguments: None,
        },
        origin: Origin::Script,
        name: name.clone(),
    });
    let properties = script.properties.iter().map(|name| Candidate {
        kind: Kind::Property,
        origin: Origin::Script,
        name: name.clone(),
    });
    let members = script
        .engine_class
        .as_deref()
        .map(|class| (engine.members)(class))
        .unwrap_or_default();
    methods.chain(properties).chain(members).collect()
}

fn read_context(code: &str) -> Context {
    let Some(caret) = code.find(CARET) else {
        return Context::Elsewhere;
    };
    let tokens = lex(code);
    let Some(at) = tokens
        .iter()
        .position(|token| token.start <= caret && caret < token.end && is_name(token))
    else {
        return Context::Elsewhere;
    };
    let before = &tokens[..at];
    match before {
        [.., receiver, call] if is_call(call) => {
            if receiver.kind == PM_TOKEN_KEYWORD_SELF {
                Context::OnSelf
            } else {
                Context::Elsewhere
            }
        }
        [.., colons] if colons.kind == PM_TOKEN_UCOLON_COLON => Context::Elsewhere,
        [.., colons] if colons.kind == PM_TOKEN_COLON_COLON => {
            if is_under_godot(code, before) {
                Context::UnderGodot
            } else {
                Context::Elsewhere
            }
        }
        _ => Context::Bare,
    }
}

fn is_name(token: &Token) -> bool {
    matches!(
        token.kind,
        PM_TOKEN_IDENTIFIER | PM_TOKEN_CONSTANT | PM_TOKEN_METHOD_NAME
    )
}

fn is_call(token: &Token) -> bool {
    matches!(token.kind, PM_TOKEN_DOT | PM_TOKEN_AMPERSAND_DOT)
}

// Whether `before` ends in `Godot::` naming the top-level Godot, as
// `::Godot::` does, and not one inside another namespace.
fn is_under_godot(code: &str, before: &[Token]) -> bool {
    let [rest @ .., godot, colons] = before else {
        return false;
    };
    colons.kind == PM_TOKEN_COLON_COLON
        && godot.kind == PM_TOKEN_CONSTANT
        && &code[godot.start..godot.end] == "Godot"
        && rest
            .last()
            .is_none_or(|outer| outer.kind != PM_TOKEN_COLON_COLON)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ancestry::tests::Sources;

    fn make_script() -> ScriptNames {
        ScriptNames {
            engine_class: Some("Node2D".to_owned()),
            methods: vec!["point".to_owned(), "rotate".to_owned()],
            properties: vec!["speed".to_owned()],
            constants: vec!["Inventory".to_owned()],
        }
    }

    // Members named after their engine class, so a test sees which class
    // was asked; `rotate` is named as the script's own method is.
    fn collect_engine_members(class: &str) -> Vec<Candidate> {
        let member = |kind, name: String| Candidate {
            kind,
            origin: Origin::Engine,
            name,
        };
        let method = Kind::Method {
            has_arguments: Some(true),
        };
        vec![
            member(method, format!("add_child_of_{class}")),
            member(Kind::Property, format!("position_of_{class}")),
            member(Kind::Constant, format!("READY_OF_{class}")),
            member(method, "rotate".to_owned()),
        ]
    }

    fn collect_offers(code: &str, script: &ScriptNames) -> Vec<Candidate> {
        let classes = || vec!["Node2D".to_owned(), "Vector2".to_owned()];
        let engine = EngineNames {
            classes: &classes,
            members: &collect_engine_members,
        };
        collect_candidates(&code.replace('|', &CARET.to_string()), script, &engine)
    }

    fn read_names(code: &str, script: &ScriptNames) -> Vec<String> {
        collect_offers(code, script)
            .into_iter()
            .map(|candidate| candidate.name)
            .collect()
    }

    // @behavior RQ-001
    #[test]
    fn a_name_under_godot_completes_to_the_engines_classes_and_value_types() {
        assert_eq!(
            read_names("x = Godot::No|", &make_script()),
            ["Node2D", "Vector2"]
        );
    }

    // @behavior RQ-001
    #[test]
    fn godot_written_from_the_top_level_completes_as_godot() {
        assert_eq!(
            read_names("::Godot::|", &make_script()),
            ["Node2D", "Vector2"]
        );
    }

    // @behavior RQ-002
    #[test]
    fn a_godot_inside_another_namespace_completes_to_no_engine_class() {
        assert!(read_names("Game::Godot::|", &make_script()).is_empty());
    }

    // @behavior RQ-002
    #[test]
    fn a_name_in_a_string_completes_to_no_engine_class() {
        assert!(read_names("\"Godot::|\"", &make_script()).is_empty());
    }

    // @behavior RQ-005
    #[test]
    fn a_name_after_self_completes_to_the_scripts_and_engine_classs_members() {
        assert_eq!(
            read_names("self.po|", &make_script()),
            [
                "point",
                "rotate",
                "speed",
                "add_child_of_Node2D",
                "position_of_Node2D"
            ]
        );
    }

    // @behavior RQ-006
    #[test]
    fn a_bare_name_completes_to_the_members_and_the_constants_in_scope() {
        assert_eq!(
            read_names("  po|", &make_script()),
            [
                "point",
                "rotate",
                "speed",
                "add_child_of_Node2D",
                "position_of_Node2D",
                "READY_OF_Node2D",
                "Inventory"
            ]
        );
    }

    // @behavior RQ-007
    #[test]
    fn a_name_after_another_receiver_completes_to_nothing() {
        assert!(read_names("enemy.po|", &make_script()).is_empty());
    }

    // @behavior RQ-008
    #[test]
    fn a_file_extending_no_engine_class_completes_to_its_own_names_alone() {
        let script = ScriptNames {
            engine_class: None,
            ..make_script()
        };
        assert_eq!(
            read_names("  po|", &script),
            ["point", "rotate", "speed", "Inventory"]
        );
    }

    fn read_project_names(path: &str, code: &str) -> ScriptNames {
        let files = Sources(
            [
                ("res://http_client.rb", "class HTTPClient\nend\n"),
                ("res://enemies/boss.rb", "module Enemies\n  class Boss < Enemy\n  end\nend\n"),
                (
                    "res://enemies/enemy.rb",
                    "module Enemies\n  class Enemy < Godot::Node2D\n    def strike; end\n  end\nend\n",
                ),
                ("res://enemies/drops/coin.rb", "module Enemies\n  module Drops\n    class Coin\n    end\n  end\nend\n"),
            ]
            .into_iter()
            .collect(),
        );
        read_script_names(&files, &Snapshot::default(), path, code)
    }

    const BOSS: &str = "res://enemies/boss.rb";
    const BOSS_DRAFT: &str = "module Enemies\n  class Boss < Enemy\n    def roar\n      st\u{FFFF}\n    end\n  end\nend\n";

    // @behavior RQ-013
    #[test]
    fn a_files_names_include_its_own_and_its_ancestors_methods() {
        let names = read_project_names(BOSS, BOSS_DRAFT);
        assert_eq!(
            (names.engine_class, names.methods),
            (
                Some("Node2D".to_owned()),
                vec!["roar".to_owned(), "strike".to_owned()]
            )
        );
    }

    // @behavior RQ-014
    #[test]
    fn the_constants_around_a_file_are_spelled_as_their_files_write_them() {
        assert_eq!(
            read_project_names(BOSS, BOSS_DRAFT).constants,
            ["HTTPClient", "Enemies", "Boss", "Enemy", "Drops"]
        );
    }

    // @behavior RQ-009
    #[test]
    fn a_method_taking_arguments_opens_its_parentheses() {
        let method = Candidate {
            kind: Kind::Method {
                has_arguments: Some(true),
            },
            origin: Origin::Engine,
            name: "add_child".to_owned(),
        };
        assert_eq!(
            (method.display(), method.insert_text()),
            ("add_child(…)".to_owned(), "add_child(".to_owned())
        );
    }

    // @behavior RQ-009
    #[test]
    fn a_method_taking_no_arguments_is_written_by_its_name_alone() {
        let method = Candidate {
            kind: Kind::Method {
                has_arguments: Some(false),
            },
            origin: Origin::Engine,
            name: "is_inside_tree".to_owned(),
        };
        assert_eq!(
            (method.display(), method.insert_text()),
            ("is_inside_tree()".to_owned(), "is_inside_tree".to_owned())
        );
    }
}
