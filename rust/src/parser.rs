//! Reads a Ruby file's source with Prism and never runs it, so what the file
//! says is known on any thread before its file has run.

use std::collections::BTreeSet;

use ruby_prism::{CallNode, Node, NodeList};

use crate::realm;

/// A file's header: the constants its `module` and `class` statements write,
/// the name the class its path names is written with, the superclass written
/// on it, the names of the methods it defines, and the `tool`, `abstract` and
/// `icon` its body calls.
#[derive(Debug, Default)]
pub struct Header {
    writes: Vec<Vec<String>>,
    name: String,
    superclass: Option<Superclass>,
    methods: BTreeSet<String>,
    tool: bool,
    is_abstract: bool,
    icon: Option<String>,
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
            writes: Vec::new(),
        };
        if let Some(program) = result.node().as_program_node() {
            reader.read_body(&program.statements().body(), &[]);
        }
        Header {
            writes: reader.writes,
            ..reader.header.unwrap_or_default()
        }
    }

    /// Each constant a `module` or `class` statement writes, in the order
    /// they are written, as its names from the top level: `module Items` then
    /// `class Potion` inside it write `["Items"]` and `["Items", "Potion"]`.
    pub fn writes(&self) -> &[Vec<String>] {
        &self.writes
    }

    /// The class's own name as its class statement writes it, without the
    /// namespaces around it: `HTTPClient` for `class Net::HTTPClient`.
    pub fn name(&self) -> &str {
        &self.name
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

    /// The icon's path, as the string `icon` is called with.
    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
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
        if call.receiver().is_some() || call.block().is_some() {
            return;
        }
        let arguments: Vec<Node> = call
            .arguments()
            .map(|arguments| arguments.arguments().iter().collect())
            .unwrap_or_default();
        match (call.name().as_slice(), arguments.as_slice()) {
            (b"tool", []) => self.tool = true,
            (b"abstract", []) => self.is_abstract = true,
            (b"icon", [path]) => {
                if let Some(path) = path.as_string_node() {
                    self.icon = Some(text(path.unescaped()));
                }
            }
            _ => {}
        }
    }
}

struct Reader {
    key: Vec<String>,
    header: Option<Header>,
    writes: Vec<Vec<String>>,
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
        self.writes.push(names.clone());
        let statements = body
            .and_then(|body| body.as_statements_node())
            .map(|statements| statements.body());
        if self.header.is_none() && self.names_file(&names) {
            let mut header = Header {
                name: names.last().cloned().unwrap_or_default(),
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

    // @behavior RH-013
    #[test]
    fn icon_called_in_the_class_body_with_a_string_carries_that_path() {
        let source = "class Enemy < Godot::Node2D\n  icon \"icons/enemy.svg\"\nend\n";

        let header = Header::read("res://enemy.rb", source);

        assert_eq!(header.icon(), Some("icons/enemy.svg"));
    }

    // @behavior RH-014
    #[test]
    fn the_classs_name_is_the_last_name_its_class_statement_writes() {
        let source = "class Net::HTTPClient < Godot::Node\nend\n";

        let header = Header::read("res://net/http_client.rb", source);

        assert_eq!(header.name(), "HTTPClient");
    }

    // @behavior RH-015
    #[test]
    fn every_constant_a_module_or_class_statement_writes_is_carried() {
        let source = "module Enemies\n  class Boss < Enemy\n    class Loot\n    end\n  end\n\n  class ::Lamp\n  end\nend\n";

        let header = Header::read("res://enemies/boss.rb", source);

        assert_eq!(
            header.writes(),
            [
                vec!["Enemies"],
                vec!["Enemies", "Boss"],
                vec!["Enemies", "Boss", "Loot"],
                vec!["Lamp"],
            ]
        );
    }

    // @behavior RH-016
    #[test]
    fn a_class_statement_inside_a_method_writes_nothing_the_header_carries() {
        let source =
            "class Boss < Godot::Node\n  def spawn\n    class Minion\n    end\n  end\nend\n";

        let header = Header::read("res://boss.rb", source);

        assert_eq!(header.writes(), [vec!["Boss"]]);
    }
}
