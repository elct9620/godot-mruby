//! The names the script editor offers where its caret stands in a Ruby file,
//! read from the tokens Prism lexes and never by running the file, so they
//! are answered without the realm.

use ruby_prism_sys::{
    PM_TOKEN_COLON_COLON, PM_TOKEN_CONSTANT, PM_TOKEN_IDENTIFIER, PM_TOKEN_METHOD_NAME,
};

use crate::lexer::{Token, lex};

/// The character the editor writes where its caret stands in the code it
/// asks about.
pub const CARET: char = '\u{FFFF}';

/// What a name the editor offers stands for, which decides its icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Class,
}

/// A name the editor may offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub kind: Kind,
    pub name: String,
}

/// What the engine names, which completing a name asks.
pub struct EngineNames<'a> {
    /// The names of the classes under `Godot`: the engine's classes and its
    /// value types.
    pub classes: &'a dyn Fn() -> Vec<String>,
}

/// The names that may complete the one the caret stands in within `code`;
/// the editor narrows them by what is typed.
pub fn collect_candidates(code: &str, engine: &EngineNames) -> Vec<Candidate> {
    let Some(caret) = code.find(CARET) else {
        return Vec::new();
    };
    let tokens = lex(code);
    let Some(at) = tokens
        .iter()
        .position(|token| token.start <= caret && caret < token.end && is_name(token))
    else {
        return Vec::new();
    };
    if !is_under_godot(code, &tokens[..at]) {
        return Vec::new();
    }
    (engine.classes)()
        .into_iter()
        .map(|name| Candidate {
            kind: Kind::Class,
            name,
        })
        .collect()
}

fn is_name(token: &Token) -> bool {
    matches!(
        token.kind,
        PM_TOKEN_IDENTIFIER | PM_TOKEN_CONSTANT | PM_TOKEN_METHOD_NAME
    )
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

    fn read_names(code: &str) -> Vec<String> {
        let classes = || vec!["Node2D".to_owned(), "Vector2".to_owned()];
        collect_candidates(
            &code.replace('|', &CARET.to_string()),
            &EngineNames { classes: &classes },
        )
        .into_iter()
        .map(|candidate| candidate.name)
        .collect()
    }

    // @behavior RQ-001
    #[test]
    fn a_name_under_godot_completes_to_the_engines_classes_and_value_types() {
        assert_eq!(read_names("x = Godot::No|"), ["Node2D", "Vector2"]);
    }

    // @behavior RQ-001
    #[test]
    fn godot_written_from_the_top_level_completes_as_godot() {
        assert_eq!(read_names("::Godot::|"), ["Node2D", "Vector2"]);
    }

    // @behavior RQ-002
    #[test]
    fn a_godot_inside_another_namespace_completes_to_no_engine_class() {
        assert!(read_names("Game::Godot::|").is_empty());
    }

    // @behavior RQ-002
    #[test]
    fn a_name_in_a_string_completes_to_no_engine_class() {
        assert!(read_names("\"Godot::|\"").is_empty());
    }
}
