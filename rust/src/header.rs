//! What a Ruby file says about the class its path names, read from its source
//! by Prism without running it, so a script answers Godot on any thread before
//! its file has run.

use std::collections::BTreeSet;

use ruby_prism::{CallNode, Node, NodeList};

use crate::realm;

/// A file's header: the superclass written on the class its path names, the
/// names of the methods that class defines, and the `tool` and `abstract` its
/// body calls.
#[derive(Debug, Default)]
pub struct Header {
    superclass: Option<Superclass>,
    methods: BTreeSet<String>,
    tool: bool,
    is_abstract: bool,
}

/// A superclass as its class statement writes it: a constant path, and the
/// namespaces Ruby looks it up from, innermost last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Superclass {
    scope: Vec<String>,
    names: Vec<String>,
}

impl Superclass {
    /// The namespaces the class statement is written in, which Ruby looks the
    /// superclass up from; none for one written from the top level, as `::A`.
    pub fn scope(&self) -> &[String] {
        &self.scope
    }

    /// The constant path as written, as `["Godot", "Node"]`.
    pub fn names(&self) -> &[String] {
        &self.names
    }
}

impl Header {
    /// Reads the header of the file at `path` from `source`. A file that does
    /// not parse still has one: Prism reads on past a syntax error.
    pub fn read(path: &str, source: &str) -> Self {
        let result = ruby_prism::parse(source.as_bytes());
        let mut reader = Reader {
            key: realm::key_of(path),
            header: None,
        };
        if let Some(program) = result.node().as_program_node() {
            reader.read_body(&program.statements().body(), &[]);
        }
        reader.header.unwrap_or_default()
    }

    pub fn superclass(&self) -> Option<&Superclass> {
        self.superclass.as_ref()
    }

    pub fn has_method(&self, name: &str) -> bool {
        self.methods.contains(name)
    }

    pub fn is_tool(&self) -> bool {
        self.tool
    }

    pub fn is_abstract(&self) -> bool {
        self.is_abstract
    }

    // What the class body's own statements say: the methods it defines on its
    // instances and the calls it makes itself; a class inside it or a method's
    // body says nothing of this class.
    fn read_class_body(&mut self, statements: &NodeList) {
        for node in statements.iter() {
            if let Some(def) = node.as_def_node() {
                if def.receiver().is_none() {
                    self.methods.insert(text(def.name().as_slice()));
                }
            } else if let Some(call) = node.as_call_node() {
                self.read_call(&call);
            }
        }
    }

    fn read_call(&mut self, call: &CallNode) {
        if call.receiver().is_some() || call.arguments().is_some() || call.block().is_some() {
            return;
        }
        match call.name().as_slice() {
            b"tool" => self.tool = true,
            b"abstract" => self.is_abstract = true,
            _ => {}
        }
    }
}

struct Reader {
    key: Vec<String>,
    header: Option<Header>,
}

impl Reader {
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
        let statements = body
            .and_then(|body| body.as_statements_node())
            .map(|statements| statements.body());
        if self.header.is_none() && self.names_file(&names) {
            let mut header = Header {
                superclass: class
                    .and_then(|class| class.superclass())
                    .and_then(|superclass| written(&superclass, scope)),
                ..Header::default()
            };
            if let Some(statements) = &statements {
                header.read_class_body(statements);
            }
            self.header = Some(header);
        }
        if let Some(statements) = &statements {
            self.read_body(statements, &names);
        }
    }

    fn names_file(&self, names: &[String]) -> bool {
        names.len() == self.key.len()
            && names
                .iter()
                .zip(&self.key)
                .all(|(name, segment)| realm::normalize(name) == *segment)
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// A superclass the class statement inside `scope` writes, when it is a
// constant path.
fn written(superclass: &Node, scope: &[String]) -> Option<Superclass> {
    let (names, from_top) = constant_path(superclass)?;
    Some(Superclass {
        scope: if from_top { Vec::new() } else { scope.to_vec() },
        names,
    })
}

// The whole constant path a `module` or `class` statement inside `scope`
// defines: `class A::B` in `module Outer` is `Outer::A::B`, `class ::A` is `A`.
// None for a path whose parent is not a constant.
fn constant_names(path: &Node, scope: &[String]) -> Option<Vec<String>> {
    let (names, from_top) = constant_path(path)?;
    Some(if from_top {
        names
    } else {
        [scope, &names].concat()
    })
}

// A constant path as written, and whether it starts from the top level as
// `::A` does. None for a path whose parent is not a constant.
fn constant_path(node: &Node) -> Option<(Vec<String>, bool)> {
    if let Some(read) = node.as_constant_read_node() {
        return Some((vec![text(read.name().as_slice())], false));
    }
    let path = node.as_constant_path_node()?;
    let (mut names, from_top) = match path.parent() {
        Some(parent) => constant_path(&parent)?,
        None => (Vec::new(), true),
    };
    names.push(text(path.name()?.as_slice()));
    Some((names, from_top))
}

#[cfg(test)]
mod tests {
    use super::Header;

    // The superclass's constant path, joined as it is written.
    fn superclass(header: &Header) -> Option<String> {
        header
            .superclass()
            .map(|superclass| superclass.names().join("::"))
    }

    fn scope(header: &Header) -> Option<Vec<String>> {
        header
            .superclass()
            .map(|superclass| superclass.scope().to_vec())
    }

    // @behavior RH-001
    #[test]
    fn a_class_written_inside_its_paths_namespaces_is_the_files_class() {
        let source = "module Enemies\n  class Boss < Godot::Node2D\n  end\nend\n";

        let header = Header::read("res://enemies/boss.rb", source);

        assert_eq!(superclass(&header).as_deref(), Some("Godot::Node2D"));
    }

    // @behavior RH-002
    #[test]
    fn a_class_written_with_its_whole_constant_path_is_the_files_class() {
        let source = "class Enemies::Boss < Godot::Node2D\nend\n";

        let header = Header::read("res://enemies/boss.rb", source);

        assert_eq!(superclass(&header).as_deref(), Some("Godot::Node2D"));
    }

    // @behavior RH-003
    #[test]
    fn a_class_matching_its_path_apart_from_underscores_and_case_is_the_files_class() {
        let source = "class HTTPClient < Godot::Node\nend\n";

        let header = Header::read("res://http_client.rb", source);

        assert_eq!(superclass(&header).as_deref(), Some("Godot::Node"));
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

        assert_eq!(superclass(&header).as_deref(), Some("Godot::Node"));
        assert!(header.has_method("_ready"));
    }

    // @behavior RH-006
    #[test]
    fn a_module_file_has_no_superclass() {
        let source = "module Items\nend\n";

        let header = Header::read("res://items.rb", source);

        assert_eq!(superclass(&header), None);
    }

    // @behavior RH-007
    #[test]
    fn a_superclass_that_is_not_a_constant_is_not_carried() {
        let source = "class Point < Struct.new(:x, :y)\nend\n";

        let header = Header::read("res://point.rb", source);

        assert_eq!(superclass(&header), None);
    }

    // @behavior RH-008
    #[test]
    fn tool_called_in_the_class_body_makes_the_header_a_tools() {
        let source = "class Player < Godot::Node\n  tool\nend\n";

        let header = Header::read("res://player.rb", source);

        assert!(header.is_tool());
    }

    // @behavior RH-009
    #[test]
    fn abstract_called_in_the_class_body_makes_the_header_an_abstract_classs() {
        let source = "class Enemy < Godot::Node2D\n  abstract\nend\n";

        let header = Header::read("res://enemy.rb", source);

        assert!(header.is_abstract());
    }

    // @behavior RH-010
    #[test]
    fn a_call_inside_a_method_of_the_class_is_not_the_class_bodys() {
        let source = "class Player < Godot::Node\n  def setup\n    tool\n  end\nend\n";

        let header = Header::read("res://player.rb", source);

        assert!(!header.is_tool());
    }

    // @behavior RH-011
    #[test]
    fn a_superclass_is_looked_up_from_the_namespaces_its_class_is_written_in() {
        let source = "module Enemies\n  class Boss < Enemy\n  end\nend\n";

        let header = Header::read("res://enemies/boss.rb", source);

        assert_eq!(scope(&header), Some(vec!["Enemies".to_owned()]));
    }

    // @behavior RH-012
    #[test]
    fn a_superclass_on_a_class_written_with_its_whole_path_is_looked_up_from_the_top_level() {
        let source = "class Enemies::Boss < Enemy\nend\n";

        let header = Header::read("res://enemies/boss.rb", source);

        assert_eq!(scope(&header), Some(Vec::new()));
    }
}
