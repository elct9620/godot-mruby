//! The script editor's colours for a Ruby file, from the tokens Prism reads
//! from the nearest line lexing can resume at.

use std::cell::RefCell;
use std::collections::BTreeMap;

use godot::classes::{EditorInterface, EditorSyntaxHighlighter, IEditorSyntaxHighlighter};
use godot::prelude::*;
use ruby_prism::{BlockNode, DefNode, LambdaNode, Visit};
use ruby_prism_sys::pm_token_type_t;

use crate::lexer::{Token, lex};
use crate::{game, language};

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

/// A line's highlights, in the order the line reads: a role runs until the
/// next highlight, or to the end of the line.
#[derive(Debug, PartialEq, Eq)]
pub struct Line {
    pub highlights: Vec<Highlight>,
    // Whether lexing from this line's start reads what lexing from the
    // source's start does: it follows the end of a statement, inside no
    // string, heredoc or embedded document, nor any method or block, whose
    // local variables lexing from inside it would not know.
    is_resumable: bool,
}

/// Each line of the source, coloured.
pub fn highlight(source: &str, is_engine_class: impl Fn(&str) -> bool) -> Vec<Line> {
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(source.match_indices('\n').map(|(at, _)| at + 1))
        .collect();
    let tokens = lex(source);
    let resumable_starts = find_resumable_starts(&tokens, &collect_local_scopes(source));
    let mut marks = vec![BTreeMap::new(); line_starts.len()];
    for (start, end, role) in assign_roles(source, &tokens, is_engine_class) {
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
        .zip(line_starts)
        .map(|(line, start)| {
            let mut highlights: Vec<Highlight> = Vec::new();
            for (column, role) in line {
                if highlights.last().is_none_or(|last| last.role != role) {
                    highlights.push(Highlight { column, role });
                }
            }
            Line {
                highlights,
                is_resumable: start == 0 || resumable_starts.contains(&start),
            }
        })
        .collect()
}

// The offsets lexing can resume at: after a newline ending a statement, once
// every token before it has ended, outside every scope in `scopes`. Prism
// hands a heredoc's body over before the rest of the line that opens it, so
// the line after the opener is not one.
fn find_resumable_starts(tokens: &[Token], scopes: &[(usize, usize)]) -> Vec<usize> {
    let mut lexed_to = 0;
    let mut starts = Vec::new();
    for token in tokens {
        lexed_to = lexed_to.max(token.end);
        let is_scoped = scopes
            .iter()
            .any(|&(start, end)| start < token.end && token.end < end);
        if token.kind == ruby_prism_sys::PM_TOKEN_NEWLINE && token.end == lexed_to && !is_scoped {
            starts.push(token.end);
        }
    }
    starts
}

// The byte ranges of the methods, blocks and lambdas in `source`, each a
// scope of local variables of its own; one a cut leaves open runs to the end.
fn collect_local_scopes(source: &str) -> Vec<(usize, usize)> {
    struct Scopes(Vec<(usize, usize)>);
    impl<'pr> Visit<'pr> for Scopes {
        fn visit_def_node(&mut self, node: &DefNode<'pr>) {
            let location = node.location();
            self.0
                .push((location.start_offset(), location.end_offset()));
            ruby_prism::visit_def_node(self, node);
        }

        fn visit_block_node(&mut self, node: &BlockNode<'pr>) {
            let location = node.location();
            self.0
                .push((location.start_offset(), location.end_offset()));
            ruby_prism::visit_block_node(self, node);
        }

        fn visit_lambda_node(&mut self, node: &LambdaNode<'pr>) {
            let location = node.location();
            self.0
                .push((location.start_offset(), location.end_offset()));
            ruby_prism::visit_lambda_node(self, node);
        }
    }

    let mut scopes = Scopes(Vec::new());
    scopes.visit(&ruby_prism::parse(source.as_bytes()).node());
    scopes.0
}

/// A text's highlighted lines from its first down, lexed only as far as they
/// are read. An edit drops the lines from the last one lexing can resume at
/// before it, so lexing again costs what the edit reaches, not the text.
#[derive(Default)]
pub struct HighlightCache {
    lines: Vec<Line>,
}

impl HighlightCache {
    // About a screen of lines, so the lines the editor reads after the first
    // come from the same lex.
    const LINES_AHEAD: usize = 100;

    /// Drops the highlights an edit from `line` down may change. Whether
    /// the edited line itself can be resumed at turns on the newline before
    /// it, which Prism judges by looking into the edited line.
    pub fn drop_from(&mut self, line: usize) {
        let lines_before = &self.lines[..line.min(self.lines.len())];
        let last_resumable = lines_before.iter().rposition(|line| line.is_resumable);
        self.lines.truncate(last_resumable.unwrap_or(0));
    }

    /// The highlights of `line` of a text with `line_count` lines, lexing
    /// from the last kept line on, as `read_line` reads them.
    pub fn read_highlights(
        &mut self,
        line: usize,
        line_count: usize,
        read_line: impl Fn(usize) -> String,
        is_engine_class: impl Fn(&str) -> bool,
    ) -> &[Highlight] {
        if line >= line_count {
            return &[];
        }
        let mut ahead = Self::LINES_AHEAD;
        while self.lines.len() <= line {
            let start = self.lines.len();
            let end = (line + ahead).min(line_count);
            let source: String = (start..end).map(|at| read_line(at) + "\n").collect();
            let mut slice_lines = highlight(&source, &is_engine_class);
            slice_lines.truncate(end - start);
            // A line after the last resumable one may stop inside a string
            // the cut ends, so it is lexed again with what follows it.
            if end < line_count {
                let last_resumable = slice_lines.iter().rposition(|line| line.is_resumable);
                slice_lines.truncate(last_resumable.unwrap_or(0));
            }
            self.lines.extend(slice_lines);
            ahead *= 2;
        }
        &self.lines[line].highlights
    }
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
    // The text's highlights, kept until its lines are edited: the editor
    // asks line by line, and a line's highlights turn on the lines before it.
    cache: RefCell<HighlightCache>,
    // The text edit whose edits drop the kept highlights.
    text_edit: Option<InstanceId>,
    base: Base<EditorSyntaxHighlighter>,
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
        self.cache.take();
        let Some(text_edit) = self.base().get_text_edit() else {
            return;
        };
        if self.text_edit != Some(text_edit.instance_id()) {
            self.text_edit = Some(text_edit.instance_id());
            text_edit
                .signals()
                .lines_edited_from()
                .connect_other(&*self, |this, from, to| {
                    let line = usize::try_from(from.min(to)).unwrap_or(0);
                    this.cache.borrow_mut().drop_from(line);
                });
        }
    }

    fn get_line_syntax_highlighting(&self, line: i32) -> AnyDictionary {
        let mut colors = VarDictionary::new();
        let Some(text_edit) = self.base().get_text_edit() else {
            return colors.upcast_any_dictionary();
        };
        let Ok(line) = usize::try_from(line) else {
            return colors.upcast_any_dictionary();
        };
        let mut cache = self.cache.borrow_mut();
        let highlights = cache.read_highlights(
            line,
            usize::try_from(text_edit.get_line_count()).unwrap_or(0),
            |at| text_edit.get_line(at as i32).to_string(),
            game::is_engine_class,
        );
        for highlight in highlights {
            let color = self.palette.color_by_role(highlight.role);
            colors.set(highlight.column as i64, &vdict! { "color" => color });
        }
        colors.upcast_any_dictionary()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::{Highlight, HighlightCache, Role, highlight};
    use crate::language::KEYWORDS;

    fn highlight_roles(source: &str) -> Vec<Vec<Highlight>> {
        highlight(source, |name| name == "Node2D")
            .into_iter()
            .map(|line| line.highlights)
            .collect()
    }

    // Every line's highlights as the editor reads them, one line at a time.
    fn read_every_line(cache: &mut HighlightCache, lines: &[&str]) -> Vec<Vec<Highlight>> {
        (0..lines.len())
            .map(|line| {
                cache
                    .read_highlights(line, lines.len(), |at| lines[at].to_owned(), |_| false)
                    .to_vec()
            })
            .collect()
    }

    // The role at a column of a line, as the editor reads it: the last
    // highlight starting at or before the column.
    fn read_role(lines: &[Vec<Highlight>], line: usize, column: usize) -> Role {
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
                    read_role(&highlight_roles(word), 0, 0),
                    Role::Keyword | Role::ControlFlow
                )
            })
            .collect();

        assert!(uncoloured.is_empty(), "not keywords: {uncoloured:?}");
    }

    // @behavior RU-003
    #[test]
    fn a_control_flow_keyword_is_coloured_apart_from_the_other_keywords() {
        let lines = highlight_roles("def jump(height)\n  return if height.zero?\nend\n");

        assert_eq!(
            [
                read_role(&lines, 1, 2),
                read_role(&lines, 1, 9),
                read_role(&lines, 0, 0)
            ],
            [Role::ControlFlow, Role::ControlFlow, Role::Keyword]
        );
    }

    // @behavior RU-004
    #[test]
    fn a_heredoc_body_is_coloured_as_a_string() {
        let lines = highlight_roles("text = <<~TEXT\n  one\n  two\nTEXT\nspeed = 1\n");

        assert_eq!(
            [
                read_role(&lines, 1, 2),
                read_role(&lines, 2, 2),
                read_role(&lines, 4, 8)
            ],
            [Role::String, Role::String, Role::Number]
        );
    }

    // @behavior RU-005
    #[test]
    fn an_embedded_document_is_coloured_as_a_comment() {
        let lines = highlight_roles("speed = 1\n=begin\nnotes\n=end\nheight = 2\n");

        assert_eq!(
            [1, 2, 3].map(|line| read_role(&lines, line, 0)),
            [Role::Comment; 3]
        );
        assert_eq!(
            [read_role(&lines, 0, 8), read_role(&lines, 4, 9)],
            [Role::Number; 2]
        );
    }

    // @behavior RU-006
    #[test]
    fn code_interpolated_into_a_string_is_coloured_as_code() {
        let lines = highlight_roles("label = \"hp #{health(10)} left\"\n");

        assert_eq!(
            [
                read_role(&lines, 0, 10),
                read_role(&lines, 0, 21),
                read_role(&lines, 0, 27)
            ],
            [Role::String, Role::Number, Role::String]
        );
    }

    // @behavior RU-007
    #[test]
    fn a_word_list_is_coloured_as_a_string() {
        let lines = highlight_roles("names = %w[slime bat]\n");

        assert_eq!(
            [read_role(&lines, 0, 11), read_role(&lines, 0, 17)],
            [Role::String; 2]
        );
    }

    // @behavior RU-008
    #[test]
    fn a_symbol_is_coloured_apart_from_a_string() {
        let lines = highlight_roles("enemy = { kind: :slime }\n");

        assert_eq!(
            [read_role(&lines, 0, 10), read_role(&lines, 0, 17)],
            [Role::Symbol; 2]
        );
    }

    // @behavior RU-009
    #[test]
    fn a_method_name_is_coloured_where_it_is_defined() {
        let lines = highlight_roles("def self.alive?\nend\n");

        assert_eq!(
            [
                read_role(&lines, 0, 9),
                read_role(&lines, 0, 14),
                read_role(&lines, 0, 4)
            ],
            [Role::Function, Role::Function, Role::Keyword]
        );
    }

    // @behavior RU-010
    #[test]
    fn an_engine_class_is_coloured_apart_from_the_games_constants() {
        let lines = highlight_roles("class Player < Godot::Node2D\nend\n");

        assert_eq!(
            [
                read_role(&lines, 0, 22),
                read_role(&lines, 0, 6),
                read_role(&lines, 0, 15)
            ],
            [Role::EngineClass, Role::Constant, Role::Constant]
        );
    }

    // @behavior RU-011
    #[test]
    fn a_column_counts_characters() {
        let lines = highlight_roles("name = \"勇者\" # hero\n");

        assert_eq!(read_role(&lines, 0, 12), Role::Comment);
    }

    // @behavior RU-012
    #[test]
    fn source_that_does_not_parse_is_still_coloured() {
        let lines = highlight_roles("def jump\n  height = (\n  return\nend\n");

        assert_eq!(read_role(&lines, 2, 2), Role::ControlFlow);
    }

    // @behavior RU-015
    #[test]
    fn a_line_below_an_edit_is_coloured_as_the_whole_file_colours_it() {
        let mut lines = vec!["speed = 1"; 5];
        lines.extend(["text = format(<<~TEXT, speed)", "  fast #{speed}", "TEXT"]);
        lines.extend(["speed = 2", "jump", "  .high", "=begin", "note", "=end"]);
        // Lexing again from the heredoc's opener stops inside this string.
        lines.resize(103, "done = true");
        lines.extend(["name = \"spans", "the lex's", "last line\"", "done = true"]);
        let mut cache = HighlightCache::default();
        read_every_line(&mut cache, &lines);

        lines[8] = "speed = :fast";
        cache.drop_from(8);
        let read = read_every_line(&mut cache, &lines);

        let whole: Vec<_> = highlight(&(lines.join("\n") + "\n"), |_| false)
            .into_iter()
            .take(lines.len())
            .map(|line| line.highlights)
            .collect();
        assert_eq!(read, whole);
    }

    // @behavior RU-017
    #[test]
    fn a_local_assigned_in_a_method_longer_than_a_lex_is_known_below() {
        let mut lines = vec!["def run", "  step = 1"];
        lines.resize(150, "  step");
        lines.extend(["  step -1", "end"]);
        let mut cache = HighlightCache::default();

        let read = read_every_line(&mut cache, &lines);
        lines[140] = "  step = 2";
        cache.drop_from(140);
        let reread = read_every_line(&mut cache, &lines);

        assert_eq!(read_role(&read, 150, 7), Role::Operator);
        assert_eq!(read_role(&reread, 150, 7), Role::Operator);
    }

    // @behavior RU-016
    #[test]
    fn an_edit_lexes_again_from_the_nearest_line_lexing_can_start_from() {
        let mut lines = vec!["speed = 1"; 4000];
        let mut cache = HighlightCache::default();
        cache.read_highlights(3999, lines.len(), |at| lines[at].to_owned(), |_| false);

        lines[3990] = "speed = 2";
        cache.drop_from(3990);
        let reads = Cell::new(0);
        cache.read_highlights(
            3991,
            lines.len(),
            |at| {
                reads.set(reads.get() + 1);
                lines[at].to_owned()
            },
            |_| false,
        );

        assert_eq!(reads.get(), lines.len() - 3989);
    }
}
