//! Where the script editor goes for a name in a Ruby file: the line of the
//! project defining it, or the engine's page for it. The name is read as
//! completion offers it at that place, so lookup and completion agree.

use std::sync::Arc;

use crate::ancestry::{self, Lineage};
use crate::completion::{self, CARET, EngineNames, Kind, Origin};
use crate::header::{Header, locate_line};
use crate::realm::{Definition, Files};
use crate::snapshot::Snapshot;

/// Where a name is defined.
#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    /// A line, counted from 1, of the file at `path`.
    Line { path: String, line: u32 },
    /// A class under `Godot`.
    EngineClass { class: String },
    /// A member of an engine class.
    EngineMember {
        class: String,
        member: Member,
        name: String,
    },
}

/// What an engine class's member is, which names the page section it is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Member {
    Method,
    Property,
    Constant,
}

/// Where `symbol`, at the caret in `code` typed as the file at `path`, is
/// defined among `files` and the engine; nothing for a name completion
/// would not offer there.
pub fn find_target(
    files: &impl Files,
    snapshot: &Snapshot,
    path: &str,
    code: &str,
    symbol: &str,
    engine: &EngineNames,
) -> Option<Target> {
    let script = completion::read_script_names(files, snapshot, path, code);
    let candidate = completion::collect_candidates(code, &script, engine)
        .into_iter()
        .find(|candidate| candidate.name == symbol)?;
    let engine_member = |member| {
        Some(Target::EngineMember {
            class: script.engine_class.clone()?,
            member,
            name: symbol.to_owned(),
        })
    };
    match (candidate.origin, candidate.kind) {
        (_, Kind::Class) => Some(Target::EngineClass {
            class: candidate.name,
        }),
        (Origin::Engine, Kind::Method { .. }) => engine_member(Member::Method),
        (Origin::Engine, Kind::Property) => engine_member(Member::Property),
        (Origin::Engine, Kind::Constant) => engine_member(Member::Constant),
        (Origin::Script, Kind::Method { .. }) => locate_method(files, path, code, symbol),
        (Origin::Project, _) => locate_constant(files, path, symbol),
        (Origin::Script, _) => None,
    }
}

// The line defining the method `name` in the file's class or the nearest
// class it inherits from defining one.
fn locate_method(files: &impl Files, path: &str, code: &str, name: &str) -> Option<Target> {
    let source = code.replace(CARET, "");
    let header = Header::from_source(path, &source, &files.roots());
    let ancestry = ancestry::trace_ancestry(path, &header, files)
        .ok()
        .map(Arc::new);
    let lineage = Lineage::new(path.to_owned(), &header, ancestry);
    lineage.files().find_map(|(file, file_header)| {
        let (_, offset) = file_header
            .method_offsets()
            .find(|(method, _)| *method == name)?;
        let file_source = if file == path {
            source.clone()
        } else {
            files.source_by_path(file).ok()?
        };
        Some(Target::Line {
            path: file.to_owned(),
            line: locate_line(&file_source, offset),
        })
    })
}

// The line of the statement naming the constant `name` in the file defining
// it; a directory's module has no such line.
fn locate_constant(files: &impl Files, path: &str, name: &str) -> Option<Target> {
    let (_, definition) = completion::collect_constants(files, path)
        .into_iter()
        .find(|(constant, _)| constant == name)?;
    let Definition::File(file) = definition else {
        return None;
    };
    let source = files.source_by_path(&file).ok()?;
    let header = Header::from_source(&file, &source, &files.roots());
    Some(Target::Line {
        line: locate_line(&source, header.name_offset()),
        path: file,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ancestry::tests::Sources;
    use crate::completion::Candidate;

    const BOSS: &str = "res://enemies/boss.rb";
    const ENEMY: &str = "res://enemies/enemy.rb";
    const BOSS_SOURCE: &str =
        "module Enemies\n  class Boss < Enemy\n    def roar\n      |\n    end\n  end\nend\n";

    fn collect_engine_members(_class: &str) -> Vec<Candidate> {
        vec![Candidate {
            kind: Kind::Method {
                has_arguments: Some(true),
            },
            origin: Origin::Engine,
            name: "add_child".to_owned(),
        }]
    }

    // The target of `symbol` once `text`, its caret where `|` is, is typed
    // into the boss.s method.
    fn find_target_at_caret(symbol: &str, text: &str) -> Option<Target> {
        let files = Sources(
            [
                ("res://loot.rb", "# The loot table.\nmodule Loot\nend\n"),
                (BOSS, BOSS_SOURCE),
                (
                    ENEMY,
                    "module Enemies\n  class Enemy < Godot::Node2D\n\n    def strike; end\n  end\nend\n",
                ),
                ("res://enemies/drops/coin.rb", "class Coin\nend\n"),
            ]
            .into_iter()
            .collect(),
        );
        let classes = || vec!["Node2D".to_owned()];
        let engine = EngineNames {
            classes: &classes,
            members: &collect_engine_members,
        };
        let code = BOSS_SOURCE.replace('|', &text.replace('|', &CARET.to_string()));
        find_target(&files, &Snapshot::default(), BOSS, &code, symbol, &engine)
    }

    // @behavior RQ-016
    #[test]
    fn a_method_of_the_file_leads_to_its_def_line() {
        assert_eq!(
            find_target_at_caret("roar", "ro|ar"),
            Some(Target::Line {
                path: BOSS.to_owned(),
                line: 3
            })
        );
    }

    // @behavior RQ-016
    #[test]
    fn an_inherited_method_leads_to_its_def_line_in_the_ancestors_file() {
        assert_eq!(
            find_target_at_caret("strike", "self.str|ike"),
            Some(Target::Line {
                path: ENEMY.to_owned(),
                line: 4
            })
        );
    }

    // @behavior RQ-017
    #[test]
    fn a_projects_constant_leads_to_the_statement_naming_it() {
        assert_eq!(
            find_target_at_caret("Loot", "Lo|ot"),
            Some(Target::Line {
                path: "res://loot.rb".to_owned(),
                line: 2
            })
        );
    }

    // @behavior RQ-017
    #[test]
    fn a_directorys_module_leads_nowhere() {
        assert_eq!(find_target_at_caret("Drops", "Dr|ops"), None);
    }

    // @behavior RQ-018
    #[test]
    fn an_engine_member_leads_to_its_engine_classs_page() {
        assert_eq!(
            find_target_at_caret("add_child", "add_|child"),
            Some(Target::EngineMember {
                class: "Node2D".to_owned(),
                member: Member::Method,
                name: "add_child".to_owned()
            })
        );
    }

    // @behavior RQ-018
    #[test]
    fn an_engine_class_leads_to_its_page() {
        assert_eq!(
            find_target_at_caret("Node2D", "Godot::No|de2D"),
            Some(Target::EngineClass {
                class: "Node2D".to_owned()
            })
        );
    }

    // @behavior RQ-019
    #[test]
    fn a_name_completion_would_not_offer_leads_nowhere() {
        assert_eq!(find_target_at_caret("strike", "enemy.str|ike"), None);
    }
}
