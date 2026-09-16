//! What a Ruby file says about the class its path names, read from its source
//! by Prism without running it, so a script answers Godot on any thread before
//! its file has run.

use std::collections::BTreeSet;

use ruby_prism::{Node, NodeList};

use crate::realm;

/// A file's header: the superclass written on the class its path names, and
/// the names of the methods that class defines.
#[derive(Debug, Default)]
pub struct Header {
    superclass: Option<String>,
    methods: BTreeSet<String>,
}

impl Header {
    /// Reads the header of the file at `path` from `source`. A file that does
    /// not parse still has one: Prism reads on past a syntax error.
    pub fn read(path: &str, source: &str) -> Self {
        let result = ruby_prism::parse(source.as_bytes());
        let mut reader = Reader {
            source: source.as_bytes(),
            key: realm::key_of(path),
            header: None,
        };
        if let Some(program) = result.node().as_program_node() {
            reader.read_body(&program.statements().body(), &[]);
        }
        reader.header.unwrap_or_default()
    }

    /// The superclass as written on the class statement, as `Godot::Node`.
    pub fn superclass(&self) -> Option<&str> {
        self.superclass.as_deref()
    }

    pub fn has_method(&self, name: &str) -> bool {
        self.methods.contains(name)
    }
}

struct Reader<'a> {
    source: &'a [u8],
    key: Vec<String>,
    header: Option<Header>,
}

impl Reader<'_> {
    // The `module` and `class` statements of a body inside the namespaces
    // `scope` names, looking for the one the file's path names.
    fn read_body(&mut self, body: &NodeList, scope: &[String]) {
        for node in body.iter() {
            self.read_statement(&node, scope);
        }
    }

    fn read_statement(&mut self, node: &Node, scope: &[String]) {
        let (path, body, class) = if let Some(module) = node.as_module_node() {
            (module.constant_path(), module.body(), None)
        } else if let Some(class) = node.as_class_node() {
            (class.constant_path(), class.body(), Some(class))
        } else {
            return;
        };
        let Some(names) = constant_names(&path, scope) else {
            return;
        };
        if self.header.is_none() && self.names_file(&names) {
            self.header = Some(Header {
                superclass: class
                    .and_then(|class| class.superclass())
                    .map(|superclass| self.text(&superclass).trim_start_matches("::").to_owned()),
                methods: body.as_ref().map(methods).unwrap_or_default(),
            });
        }
        if let Some(statements) = body.and_then(|body| body.as_statements_node()) {
            self.read_body(&statements.body(), &names);
        }
    }

    fn names_file(&self, names: &[String]) -> bool {
        names.len() == self.key.len()
            && names
                .iter()
                .zip(&self.key)
                .all(|(name, segment)| realm::normalize(name) == *segment)
    }

    fn text(&self, node: &Node) -> String {
        let location = node.location();
        String::from_utf8_lossy(&self.source[location.start_offset()..location.end_offset()])
            .into_owned()
    }
}

// The methods a class body defines on its instances; what a class inside it
// defines is that class's own.
fn methods(body: &Node) -> BTreeSet<String> {
    let Some(statements) = body.as_statements_node() else {
        return BTreeSet::new();
    };
    statements
        .body()
        .iter()
        .filter_map(|node| node.as_def_node())
        .filter(|def| def.receiver().is_none())
        .map(|def| String::from_utf8_lossy(def.name().as_slice()).into_owned())
        .collect()
}

// The whole constant path a `module` or `class` statement inside `scope`
// defines: `class A::B` in `module Outer` is `Outer::A::B`, `class ::A` is `A`.
// None for a path whose parent is not a constant.
fn constant_names(path: &Node, scope: &[String]) -> Option<Vec<String>> {
    let name = |id: &[u8]| String::from_utf8_lossy(id).into_owned();
    if let Some(read) = path.as_constant_read_node() {
        let mut names = scope.to_vec();
        names.push(name(read.name().as_slice()));
        return Some(names);
    }
    let constant_path = path.as_constant_path_node()?;
    let mut names = match constant_path.parent() {
        Some(parent) => constant_names(&parent, scope)?,
        None => Vec::new(),
    };
    names.push(name(constant_path.name()?.as_slice()));
    Some(names)
}

#[cfg(test)]
mod tests {
    use super::Header;

    // @behavior RH-001
    #[test]
    fn a_class_written_inside_its_paths_namespaces_is_the_files_class() {
        let source = "module Enemies\n  class Boss < Godot::Node2D\n  end\nend\n";

        let header = Header::read("res://enemies/boss.rb", source);

        assert_eq!(header.superclass(), Some("Godot::Node2D"));
    }

    // @behavior RH-002
    #[test]
    fn a_class_written_with_its_whole_constant_path_is_the_files_class() {
        let source = "class Enemies::Boss < Godot::Node2D\nend\n";

        let header = Header::read("res://enemies/boss.rb", source);

        assert_eq!(header.superclass(), Some("Godot::Node2D"));
    }

    // @behavior RH-003
    #[test]
    fn a_class_matching_its_path_apart_from_underscores_and_case_is_the_files_class() {
        let source = "class HTTPClient < Godot::Node\nend\n";

        let header = Header::read("res://http_client.rb", source);

        assert_eq!(header.superclass(), Some("Godot::Node"));
    }

    // @behavior RH-004
    #[test]
    fn a_method_of_a_class_nested_in_the_files_class_is_not_the_files() {
        let source = "class Player < Godot::Node\n  class Stats\n    def _ready\n    end\n  end\n\n  def _process(delta)\n  end\nend\n";

        let header = Header::read("res://player.rb", source);

        assert!(!header.has_method("_ready"));
        assert!(header.has_method("_process"));
    }

    // @behavior RH-005
    #[test]
    fn a_file_that_does_not_parse_still_has_a_header() {
        let source = "class Player < Godot::Node\n  def _ready\n  end\nend\nend\n";

        let header = Header::read("res://player.rb", source);

        assert_eq!(header.superclass(), Some("Godot::Node"));
        assert!(header.has_method("_ready"));
    }

    // @behavior RH-006
    #[test]
    fn a_module_file_has_no_superclass() {
        let source = "module Items\nend\n";

        let header = Header::read("res://items.rb", source);

        assert_eq!(header.superclass(), None);
    }
}
