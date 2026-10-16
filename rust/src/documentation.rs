//! What a Ruby file's comments say of its class and members, read as RDoc
//! reads them: the last block of comment lines above a definition, with no
//! statement between them.

use crate::header::Header;

/// A file's documentation: its class's, then its members'.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Documentation {
    /// The first paragraph of the class's comment.
    pub brief: String,
    /// The paragraphs after it.
    pub description: String,
    pub methods: Vec<Entry>,
    pub properties: Vec<Entry>,
    pub signals: Vec<Entry>,
}

/// A member and what its comment says of it.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub description: String,
}

// Comments Ruby reads as the file's settings rather than as words about it.
const MAGIC_COMMENTS: [&str; 5] = [
    "frozen_string_literal:",
    "encoding:",
    "coding:",
    "warn_indent:",
    "shareable_constant_value:",
];

/// What the comments of `source`, whose header is `header`, say of the
/// file's class and of each method, export and signal it declares; a method
/// whose `def` line says `:nodoc:` is left out, as RDoc leaves it.
pub fn read_documentation(source: &str, header: &Header) -> Documentation {
    let class = read_comment(source, header.name_offset());
    let (brief, description) = class
        .split_once("\n\n")
        .map_or((class.as_str(), ""), |(brief, rest)| (brief, rest));
    let entries = |offsets: Vec<(&str, usize)>| {
        offsets
            .into_iter()
            .map(|(name, offset)| Entry {
                name: name.to_owned(),
                description: read_comment(source, offset),
            })
            .collect()
    };
    let methods = header
        .method_offsets()
        .filter(|(_, offset)| !is_hidden(source, *offset))
        .collect();
    Documentation {
        brief: brief.to_owned(),
        description: description.to_owned(),
        methods: entries(methods),
        properties: entries(header.export_offsets().collect()),
        signals: entries(header.signal_offsets().collect()),
    }
}

// The comment block above the line the byte at `offset` is on: blank lines
// right above it skipped, then every comment line up to the first line that
// is not one, each without its `#` and the space after it.
fn read_comment(source: &str, offset: usize) -> String {
    let line_start = source[..offset.min(source.len())]
        .rfind('\n')
        .map_or(0, |at| at + 1);
    let mut lines: Vec<&str> = source[..line_start]
        .lines()
        .rev()
        .skip_while(|line| line.trim().is_empty())
        .map_while(|line| line.trim_start().strip_prefix('#'))
        .collect();
    lines.reverse();
    lines
        .into_iter()
        .map(|line| line.strip_prefix(' ').unwrap_or(line))
        .filter(|line| !MAGIC_COMMENTS.iter().any(|magic| line.starts_with(magic)))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

// Whether the line the byte at `offset` is on asks RDoc to leave its
// definition out.
fn is_hidden(source: &str, offset: usize) -> bool {
    source[offset.min(source.len())..]
        .lines()
        .next()
        .and_then(|line| line.split_once('#'))
        .is_some_and(|(_, comment)| comment.trim() == ":nodoc:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realm::Roots;

    const BOSS: &str = "\
# frozen_string_literal: true

# res://enemies/boss.rb

# A boss that guards the gate.
#
# It has two phases.
class Boss < Godot::Node2D
  # Emitted as it falls.
  signal :fell

  # Separated by a blank line.

  def gap; end

  # Two.
  # Lines.

  # Last.
  def last; end

  x = 1
  def after_statement; end

  # Not for docs.
  def hidden # :nodoc:
  end
end
";

    fn read_boss() -> Documentation {
        let header = Header::from_source("res://boss.rb", BOSS, &Roots::default());
        read_documentation(BOSS, &header)
    }

    fn read_method_descriptions(documentation: &Documentation) -> Vec<(&str, &str)> {
        documentation
            .methods
            .iter()
            .map(|entry| (entry.name.as_str(), entry.description.as_str()))
            .collect()
    }

    // @behavior RZ-001
    #[test]
    fn the_class_comments_first_paragraph_is_its_brief_and_the_rest_its_description() {
        let documentation = read_boss();
        assert_eq!(
            (
                documentation.brief.as_str(),
                documentation.description.as_str()
            ),
            ("A boss that guards the gate.", "It has two phases.")
        );
    }

    // @behavior RZ-002
    #[test]
    fn a_method_takes_the_last_comment_block_above_it_with_no_statement_between() {
        assert_eq!(
            read_method_descriptions(&read_boss()),
            [
                ("after_statement", ""),
                ("gap", "Separated by a blank line."),
                ("last", "Last.")
            ]
        );
    }

    // @behavior RZ-003
    #[test]
    fn a_signal_takes_the_comment_above_its_call() {
        let documentation = read_boss();
        assert_eq!(
            documentation.signals,
            [Entry {
                name: "fell".to_owned(),
                description: "Emitted as it falls.".to_owned()
            }]
        );
    }

    // @behavior RZ-004
    #[test]
    fn a_magic_comment_is_no_documentation() {
        let source = "# frozen_string_literal: true\nclass Magic\nend\n";
        let header = Header::from_source("res://magic.rb", source, &Roots::default());
        assert_eq!(
            read_documentation(source, &header),
            Documentation::default()
        );
    }
}
