//! The script editor's colours for a Ruby file, from the tokens Prism reads
//! in the whole file.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;

use godot::classes::{ClassDb, EditorInterface, EditorSyntaxHighlighter, IEditorSyntaxHighlighter};
use godot::prelude::*;
use ruby_prism_sys::{
    pm_lex_callback_t, pm_node_destroy, pm_parse, pm_parser_free, pm_parser_init, pm_parser_t,
    pm_token_t, pm_token_type_t,
};

use crate::language;

/// What a stretch of a line is to its reader, which decides its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Text,
    Keyword,
    ControlFlow,
    String,
    Symbol,
    Comment,
    Number,
    Operator,
    Variable,
    Function,
    EngineClass,
    Constant,
}

/// Where a role starts on its line, counted in characters, as the editor
/// counts columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Highlight {
    pub column: usize,
    pub role: Role,
}

/// Each line's highlights, in the order the line reads. A role runs until
/// the next highlight, or to the end of its line.
pub fn highlight(source: &str, is_engine_class: impl Fn(&str) -> bool) -> Vec<Vec<Highlight>> {
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(source.match_indices('\n').map(|(at, _)| at + 1))
        .collect();
    let mut marks = vec![BTreeMap::new(); line_starts.len()];
    for (start, end, role) in assign_roles(source, &lex(source), is_engine_class) {
        let mut line = line_starts.partition_point(|&at| at <= start) - 1;
        while line < line_starts.len() && line_starts[line] < end.max(start + 1) {
            let line_start = line_starts[line];
            let line_end = line_starts
                .get(line + 1)
                .map_or(source.len(), |next| next - 1);
            let column = |at: usize| {
                source[line_start..at.clamp(line_start, line_end)]
                    .chars()
                    .count()
            };
            marks[line].insert(column(start), role);
            if end < line_end {
                marks[line].insert(column(end), Role::Text);
            }
            line += 1;
        }
    }
    marks
        .into_iter()
        .map(|line| {
            let mut highlights: Vec<Highlight> = Vec::new();
            for (column, role) in line {
                if highlights.last().is_none_or(|last| last.role != role) {
                    highlights.push(Highlight { column, role });
                }
            }
            highlights
        })
        .collect()
}

struct Token {
    kind: pm_token_type_t,
    start: usize,
    end: usize,
}

// Every token Prism lexes while it parses the source, in the order they stand
// in it.
fn lex(source: &str) -> Vec<Token> {
    unsafe extern "C" fn collect(
        data: *mut c_void,
        parser: *mut pm_parser_t,
        token: *mut pm_token_t,
    ) {
        // SAFETY: `data` is the vector `lex` hands Prism for this parse, and
        // the parser and token are Prism's, alive for this call.
        let (tokens, source, token) =
            unsafe { (&mut *data.cast::<Vec<Token>>(), (*parser).start, &*token) };
        // SAFETY: a token lies within the source the parser was given.
        let offset = |at: *const u8| unsafe { at.offset_from_unsigned(source) };
        tokens.push(Token {
            kind: token.type_,
            start: offset(token.start),
            end: offset(token.end),
        });
    }

    let mut tokens: Vec<Token> = Vec::new();
    let mut callback = pm_lex_callback_t {
        data: (&raw mut tokens).cast(),
        callback: Some(collect),
    };
    let mut parser = Box::new(MaybeUninit::<pm_parser_t>::uninit());
    // SAFETY: as `ruby_prism::parse` does: the parser is initialised before it
    // is used, the source and callback outlive the parse, and the node and
    // parser are freed once it ends.
    unsafe {
        let parser = parser.as_mut_ptr();
        pm_parser_init(parser, source.as_ptr(), source.len(), ptr::null());
        (*parser).lex_callback = &raw mut callback;
        let node = pm_parse(parser);
        pm_node_destroy(parser, node);
        pm_parser_free(parser);
    }
    tokens
}

// Each token's stretch of the source with the role it plays, reading a
// token by the ones around it where its kind alone does not tell.
fn assign_roles(
    source: &str,
    tokens: &[Token],
    is_engine_class: impl Fn(&str) -> bool,
) -> Vec<(usize, usize, Role)> {
    use ruby_prism_sys::*;

    let mut roles = Vec::new();
    let mut defining = false;
    let mut naming_symbol = false;
    for (index, token) in tokens.iter().enumerate() {
        let text = &source[token.start..token.end];
        let next = tokens.get(index + 1);
        let role = match token.kind {
            PM_TOKEN_EOF
            | PM_TOKEN_NEWLINE
            | PM_TOKEN_IGNORED_NEWLINE
            | PM_TOKEN_MISSING
            | PM_TOKEN_NOT_PROVIDED => continue,
            _ if naming_symbol => Role::Symbol,
            PM_TOKEN_DOT => Role::Operator,
            _ if defining && next.is_some_and(|next| next.kind == PM_TOKEN_DOT) => {
                role_by_kind(token.kind, text, &is_engine_class)
            }
            _ if defining => Role::Function,
            PM_TOKEN_IDENTIFIER | PM_TOKEN_METHOD_NAME
                if next.is_some_and(|next| {
                    next.kind == PM_TOKEN_PARENTHESIS_LEFT && next.start == token.end
                }) =>
            {
                Role::Function
            }
            kind => role_by_kind(kind, text, &is_engine_class),
        };
        naming_symbol = token.kind == PM_TOKEN_SYMBOL_BEGIN && text == ":";
        defining = match token.kind {
            PM_TOKEN_KEYWORD_DEF => true,
            PM_TOKEN_DOT => defining,
            _ => defining && next.is_some_and(|next| next.kind == PM_TOKEN_DOT),
        };
        roles.push((token.start, token.end, role));
    }
    roles
}

fn role_by_kind(kind: pm_token_type_t, text: &str, is_engine_class: impl Fn(&str) -> bool) -> Role {
    use ruby_prism_sys::*;

    match kind {
        PM_TOKEN_KEYWORD_DO..=PM_TOKEN_KEYWORD_WHEN
        | PM_TOKEN_KEYWORD_ALIAS..=PM_TOKEN_KEYWORD___LINE__ => {
            if language::is_control_flow(text) {
                Role::ControlFlow
            } else {
                Role::Keyword
            }
        }
        PM_TOKEN_STRING_BEGIN
        | PM_TOKEN_STRING_CONTENT
        | PM_TOKEN_STRING_END
        | PM_TOKEN_HEREDOC_START
        | PM_TOKEN_HEREDOC_END
        | PM_TOKEN_CHARACTER_LITERAL
        | PM_TOKEN_PERCENT_LOWER_I
        | PM_TOKEN_PERCENT_LOWER_W
        | PM_TOKEN_PERCENT_LOWER_X
        | PM_TOKEN_PERCENT_UPPER_I
        | PM_TOKEN_PERCENT_UPPER_W
        | PM_TOKEN_WORDS_SEP
        | PM_TOKEN_BACKTICK
        | PM_TOKEN_REGEXP_BEGIN
        | PM_TOKEN_REGEXP_END => Role::String,
        PM_TOKEN_SYMBOL_BEGIN | PM_TOKEN_LABEL | PM_TOKEN_LABEL_END => Role::Symbol,
        PM_TOKEN_COMMENT
        | PM_TOKEN_EMBDOC_BEGIN
        | PM_TOKEN_EMBDOC_LINE
        | PM_TOKEN_EMBDOC_END
        | PM_TOKEN___END__ => Role::Comment,
        PM_TOKEN_FLOAT..=PM_TOKEN_FLOAT_RATIONAL_IMAGINARY
        | PM_TOKEN_INTEGER..=PM_TOKEN_INTEGER_RATIONAL_IMAGINARY
        | PM_TOKEN_UMINUS_NUM => Role::Number,
        PM_TOKEN_INSTANCE_VARIABLE
        | PM_TOKEN_CLASS_VARIABLE
        | PM_TOKEN_GLOBAL_VARIABLE
        | PM_TOKEN_BACK_REFERENCE
        | PM_TOKEN_NUMBERED_REFERENCE => Role::Variable,
        PM_TOKEN_CONSTANT if is_engine_class(text) => Role::EngineClass,
        PM_TOKEN_CONSTANT => Role::Constant,
        PM_TOKEN_IDENTIFIER | PM_TOKEN_METHOD_NAME => Role::Text,
        _ if text.bytes().all(|byte| byte.is_ascii_punctuation()) => Role::Operator,
        _ => Role::Text,
    }
}

/// The highlighter the script editor offers for Ruby files, which colours
/// each line in the colours the editor's theme gives each role.
#[derive(GodotClass)]
#[class(base = EditorSyntaxHighlighter, init, tool)]
pub struct RubySyntaxHighlighter {
    palette: Palette,
    coloring: RefCell<Coloring>,
    base: Base<EditorSyntaxHighlighter>,
}

// The source last coloured, kept with its colours, since the editor asks
// line by line and a line's colours turn on the lines before it.
#[derive(Default)]
struct Coloring {
    source: String,
    lines: Vec<Vec<Highlight>>,
}

#[derive(Default)]
struct Palette {
    colors: Vec<(Role, Color)>,
}

impl Palette {
    // The theme's colours, under the names GDScript's highlighter reads; a
    // Ruby symbol takes a StringName's, which is what it arrives as.
    fn from_settings() -> Self {
        let Some(settings) = EditorInterface::singleton().get_editor_settings() else {
            return Self::default();
        };
        let color = |name: &str| {
            settings
                .get_setting(&format!("text_editor/theme/highlighting/{name}"))
                .try_to::<Color>()
                .ok()
        };
        let text = color("text_color").unwrap_or(Color::WHITE);
        let string = color("string_color").unwrap_or(text);
        let colors = [
            (Role::Text, "text_color"),
            (Role::Keyword, "keyword_color"),
            (Role::ControlFlow, "control_flow_keyword_color"),
            (Role::String, "string_color"),
            (Role::Symbol, "gdscript/string_name_color"),
            (Role::Comment, "comment_color"),
            (Role::Number, "number_color"),
            (Role::Operator, "symbol_color"),
            (Role::Variable, "member_variable_color"),
            (Role::Function, "function_color"),
            (Role::EngineClass, "engine_type_color"),
            (Role::Constant, "user_type_color"),
        ]
        .into_iter()
        .map(|(role, name)| {
            let fallback = if role == Role::Symbol { string } else { text };
            (role, color(name).unwrap_or(fallback))
        })
        .collect();
        Self { colors }
    }

    fn color_by_role(&self, role: Role) -> Color {
        self.colors
            .iter()
            .find(|(each, _)| *each == role)
            .map_or(Color::WHITE, |(_, color)| *color)
    }
}

#[godot_api]
impl IEditorSyntaxHighlighter for RubySyntaxHighlighter {
    fn get_name(&self) -> GString {
        language::NAME.into()
    }

    fn get_supported_languages(&self) -> PackedStringArray {
        PackedStringArray::from(&[GString::from(language::NAME)])
    }

    // Each script the editor opens gets its own, bound to that script's text.
    fn create(&self) -> Option<Gd<EditorSyntaxHighlighter>> {
        Some(Self::new_gd().upcast())
    }

    fn update_cache(&mut self) {
        self.palette = Palette::from_settings();
    }

    fn get_line_syntax_highlighting(&self, line: i32) -> AnyDictionary {
        let mut colors = VarDictionary::new();
        let Some(text_edit) = self.base().get_text_edit() else {
            return colors.upcast_any_dictionary();
        };
        let source = text_edit.get_text().to_string();
        let mut coloring = self.coloring.borrow_mut();
        if coloring.source != source {
            let class_db = ClassDb::singleton();
            let lines = highlight(&source, |name| class_db.class_exists(name));
            *coloring = Coloring { source, lines };
        }
        let highlights = usize::try_from(line)
            .ok()
            .and_then(|line| coloring.lines.get(line));
        for highlight in highlights.into_iter().flatten() {
            let color = self.palette.color_by_role(highlight.role);
            colors.set(highlight.column as i64, &vdict! { "color" => color });
        }
        colors.upcast_any_dictionary()
    }
}

#[cfg(test)]
mod tests {
    use super::{Highlight, Role, highlight};
    use crate::language::KEYWORDS;

    fn roles(source: &str) -> Vec<Vec<Highlight>> {
        highlight(source, |name| name == "Node2D")
    }

    // The role at a column of a line, as the editor reads it: the last
    // highlight starting at or before the column.
    fn role_at(lines: &[Vec<Highlight>], line: usize, column: usize) -> Role {
        lines[line]
            .iter()
            .take_while(|highlight| highlight.column <= column)
            .last()
            .map_or(Role::Text, |highlight| highlight.role)
    }

    // @behavior RU-002
    #[test]
    fn a_reserved_word_is_coloured_as_a_keyword() {
        let uncoloured: Vec<_> = KEYWORDS
            .iter()
            .filter(|word| {
                !matches!(
                    role_at(&roles(word), 0, 0),
                    Role::Keyword | Role::ControlFlow
                )
            })
            .collect();

        assert!(uncoloured.is_empty(), "not keywords: {uncoloured:?}");
    }

    // @behavior RU-003
    #[test]
    fn a_control_flow_keyword_is_coloured_apart_from_the_other_keywords() {
        let lines = roles("def jump(height)\n  return if height.zero?\nend\n");

        assert_eq!(
            [
                role_at(&lines, 1, 2),
                role_at(&lines, 1, 9),
                role_at(&lines, 0, 0)
            ],
            [Role::ControlFlow, Role::ControlFlow, Role::Keyword]
        );
    }

    // @behavior RU-004
    #[test]
    fn a_heredoc_body_is_coloured_as_a_string() {
        let lines = roles("text = <<~TEXT\n  one\n  two\nTEXT\nspeed = 1\n");

        assert_eq!(
            [
                role_at(&lines, 1, 2),
                role_at(&lines, 2, 2),
                role_at(&lines, 4, 8)
            ],
            [Role::String, Role::String, Role::Number]
        );
    }

    // @behavior RU-005
    #[test]
    fn an_embedded_document_is_coloured_as_a_comment() {
        let lines = roles("speed = 1\n=begin\nnotes\n=end\nheight = 2\n");

        assert_eq!(
            [1, 2, 3].map(|line| role_at(&lines, line, 0)),
            [Role::Comment; 3]
        );
        assert_eq!(
            [role_at(&lines, 0, 8), role_at(&lines, 4, 9)],
            [Role::Number; 2]
        );
    }

    // @behavior RU-006
    #[test]
    fn code_interpolated_into_a_string_is_coloured_as_code() {
        let lines = roles("label = \"hp #{health(10)} left\"\n");

        assert_eq!(
            [
                role_at(&lines, 0, 10),
                role_at(&lines, 0, 21),
                role_at(&lines, 0, 27)
            ],
            [Role::String, Role::Number, Role::String]
        );
    }

    // @behavior RU-007
    #[test]
    fn a_word_list_is_coloured_as_a_string() {
        let lines = roles("names = %w[slime bat]\n");

        assert_eq!(
            [role_at(&lines, 0, 11), role_at(&lines, 0, 17)],
            [Role::String; 2]
        );
    }

    // @behavior RU-008
    #[test]
    fn a_symbol_is_coloured_apart_from_a_string() {
        let lines = roles("enemy = { kind: :slime }\n");

        assert_eq!(
            [role_at(&lines, 0, 10), role_at(&lines, 0, 17)],
            [Role::Symbol; 2]
        );
    }

    // @behavior RU-009
    #[test]
    fn a_method_name_is_coloured_where_it_is_defined() {
        let lines = roles("def self.alive?\nend\n");

        assert_eq!(
            [
                role_at(&lines, 0, 9),
                role_at(&lines, 0, 14),
                role_at(&lines, 0, 4)
            ],
            [Role::Function, Role::Function, Role::Keyword]
        );
    }

    // @behavior RU-010
    #[test]
    fn an_engine_class_is_coloured_apart_from_the_games_constants() {
        let lines = roles("class Player < Godot::Node2D\nend\n");

        assert_eq!(
            [
                role_at(&lines, 0, 22),
                role_at(&lines, 0, 6),
                role_at(&lines, 0, 15)
            ],
            [Role::EngineClass, Role::Constant, Role::Constant]
        );
    }

    // @behavior RU-011
    #[test]
    fn a_column_counts_characters() {
        let lines = roles("name = \"勇者\" # hero\n");

        assert_eq!(role_at(&lines, 0, 12), Role::Comment);
    }

    // @behavior RU-012
    #[test]
    fn source_that_does_not_parse_is_still_coloured() {
        let lines = roles("def jump\n  height = (\n  return\nend\n");

        assert_eq!(role_at(&lines, 2, 2), Role::ControlFlow);
    }
}
