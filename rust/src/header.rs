//! Reads a Ruby file's source with Prism and never runs it, so what the file
//! says is known on any thread before its file has run.

use std::collections::BTreeSet;

use godot::builtin::{GString, StringName, VarArray, VarDictionary, Variant};
use godot::meta::ToGodot;
use ruby_prism::{CallNode, Integer, Node, NodeList};

use crate::realm::{self, Roots};
use crate::snapshot::{Property, Signal};

/// A file's header: the constants its `module` and `class` statements write,
/// the name the class its path names is written with, the superclass written
/// on it, the names of the methods it defines, the signals and properties
/// its body declares, and the `tool`, `abstract` and `icon` its body calls.
#[derive(Debug, Default)]
pub struct Header {
    writes: Vec<Vec<String>>,
    name: String,
    superclass: Option<Superclass>,
    methods: BTreeSet<String>,
    signals: Vec<Signal>,
    exports: Vec<Property>,
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
    /// Reads the header of the file at `path`, named from `roots`, from
    /// `source`. A file that does not parse still has one: Prism reads on
    /// past a syntax error.
    pub fn read(path: &str, source: &str, roots: &Roots) -> Self {
        let result = ruby_prism::parse(source.as_bytes());
        let mut reader = Reader {
            key: roots.key_of(path),
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

    /// The names of the methods the file's class defines, as its `def`
    /// statements write them.
    pub fn methods(&self) -> impl Iterator<Item = &str> {
        self.methods.iter().map(String::as_str)
    }

    /// The signals the class declares, in the order it declares them, as the
    /// `signal` calls of its body write them; a call writing a name it does
    /// not spell out declares none the header can read.
    pub fn signals(&self) -> &[Signal] {
        &self.signals
    }

    /// The properties the class exports, in the order it declares them, as
    /// the `export` calls of its body write them. A value the call does not
    /// write out is the file's to work out as it runs, and the hint and the
    /// type a keyword names are read only then, so a property here carries
    /// the name and the value alone.
    pub fn exports(&self) -> &[Property] {
        &self.exports
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
            (b"signal", [name, parameters @ ..]) => {
                if let Some(signal) = declared(name, parameters) {
                    self.signals.push(signal);
                }
            }
            (b"export", [name, default, ..]) => {
                if let Some(property) = exported(name, default) {
                    self.exports.push(property);
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

// The signal a `signal` call declares, as it is written: the name it is
// emitted by and a name for each value it carries. A name written as
// anything but a symbol or a string is the file's to work out as it runs, so
// the header carries none of that declaration.
fn declared(name: &Node, parameters: &[Node]) -> Option<Signal> {
    Some(Signal {
        name: name_of(name)?,
        parameters: parameters.iter().map(name_of).collect::<Option<_>>()?,
    })
}

// The property an `export` call declares, as it is written: the name Godot
// reads it by and the value it is declared with, which gives it its type. A
// value written as anything but a literal is the file's to work out as it
// runs, so the header carries none of that declaration.
fn exported(name: &Node, default: &Node) -> Option<Property> {
    Some(Property::new(name_of(name)?, &literal(default)?))
}

// The value a literal writes, as the engine takes it, following the Ruby
// value's own class as a value crossing from a running file does. A literal
// holding anything else, such as a constant or a call, writes none.
fn literal(node: &Node) -> Option<Variant> {
    if node.as_true_node().is_some() {
        return Some(true.to_variant());
    }
    if node.as_false_node().is_some() {
        return Some(false.to_variant());
    }
    if let Some(number) = node.as_integer_node() {
        return whole(&number.value()).map(|number| number.to_variant());
    }
    if let Some(number) = node.as_float_node() {
        return Some(number.value().to_variant());
    }
    if let Some(string) = node.as_string_node() {
        return Some(GString::from(&text(string.unescaped())).to_variant());
    }
    if let Some(symbol) = node.as_symbol_node() {
        return Some(StringName::from(&text(symbol.unescaped())).to_variant());
    }
    if let Some(array) = node.as_array_node() {
        let mut copied = VarArray::new();
        for element in array.elements().iter() {
            copied.push(&literal(&element)?);
        }
        return Some(copied.to_variant());
    }
    if let Some(hash) = node.as_hash_node() {
        let mut copied = VarDictionary::new();
        for element in hash.elements().iter() {
            let entry = element.as_assoc_node()?;
            copied.set(&literal(&entry.key())?, &literal(&entry.value())?);
        }
        return Some(copied.to_variant());
    }
    None
}

// The number an integer literal writes, unless it is larger than the
// engine's own integers hold.
fn whole(number: &Integer) -> Option<i64> {
    let (negative, digits) = number.to_u32_digits();
    let mut whole: i64 = 0;
    for digit in digits.iter().rev() {
        whole = whole
            .checked_mul(i64::from(u32::MAX) + 1)?
            .checked_add(i64::from(*digit))?;
    }
    Some(if negative { -whole } else { whole })
}

// A name as a call writes it, which is a symbol or a string.
fn name_of(node: &Node) -> Option<String> {
    if let Some(symbol) = node.as_symbol_node() {
        return Some(text(symbol.unescaped()));
    }
    node.as_string_node().map(|string| text(string.unescaped()))
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
    use super::{Header, Roots, Signal};

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

        let header = Header::read("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(superclass(&header).as_deref(), Some("Godot::Node2D"));
    }

    // @behavior RH-002
    #[test]
    fn a_class_written_with_its_whole_constant_path_is_the_files_class() {
        let source = "class Enemies::Boss < Godot::Node2D\nend\n";

        let header = Header::read("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(superclass(&header).as_deref(), Some("Godot::Node2D"));
    }

    // @behavior RH-003
    #[test]
    fn a_class_matching_its_path_apart_from_underscores_and_case_is_the_files_class() {
        let source = "class HTTPClient < Godot::Node\nend\n";

        let header = Header::read("res://http_client.rb", source, &Roots::default());

        assert_eq!(superclass(&header).as_deref(), Some("Godot::Node"));
    }

    // @behavior RH-004
    #[test]
    fn a_method_of_a_class_nested_in_the_files_class_is_not_the_files() {
        let source = "class Player < Godot::Node\n  class Stats\n    def _ready\n    end\n  end\n\n  def _process(delta)\n  end\nend\n";

        let header = Header::read("res://player.rb", source, &Roots::default());

        assert!(!header.has_method("_ready"));
        assert!(header.has_method("_process"));
    }

    // @behavior RH-005
    #[test]
    fn a_file_that_does_not_parse_still_has_a_header() {
        let source = "class Player < Godot::Node\n  def _ready\n  end\nend\nend\n";

        let header = Header::read("res://player.rb", source, &Roots::default());

        assert_eq!(superclass(&header).as_deref(), Some("Godot::Node"));
        assert!(header.has_method("_ready"));
    }

    // @behavior RH-006
    #[test]
    fn a_module_file_has_no_superclass() {
        let source = "module Items\nend\n";

        let header = Header::read("res://items.rb", source, &Roots::default());

        assert_eq!(superclass(&header), None);
    }

    // @behavior RH-007
    #[test]
    fn a_superclass_that_is_not_a_constant_is_not_carried() {
        let source = "class Point < Struct.new(:x, :y)\nend\n";

        let header = Header::read("res://point.rb", source, &Roots::default());

        assert_eq!(superclass(&header), None);
    }

    // @behavior RH-008
    #[test]
    fn tool_called_in_the_class_body_makes_the_header_a_tools() {
        let source = "class Player < Godot::Node\n  tool\nend\n";

        let header = Header::read("res://player.rb", source, &Roots::default());

        assert!(header.is_tool());
    }

    // @behavior RH-009
    #[test]
    fn abstract_called_in_the_class_body_makes_the_header_an_abstract_classs() {
        let source = "class Enemy < Godot::Node2D\n  abstract\nend\n";

        let header = Header::read("res://enemy.rb", source, &Roots::default());

        assert!(header.is_abstract());
    }

    // @behavior RH-010
    #[test]
    fn a_call_inside_a_method_of_the_class_is_not_the_class_bodys() {
        let source = "class Player < Godot::Node\n  def setup\n    tool\n  end\nend\n";

        let header = Header::read("res://player.rb", source, &Roots::default());

        assert!(!header.is_tool());
    }

    // @behavior RH-011
    #[test]
    fn a_superclass_is_looked_up_from_the_namespaces_its_class_is_written_in() {
        let source = "module Enemies\n  class Boss < Enemy\n  end\nend\n";

        let header = Header::read("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(scope(&header), Some(vec!["Enemies".to_owned()]));
    }

    // @behavior RH-012
    #[test]
    fn a_superclass_on_a_class_written_with_its_whole_path_is_looked_up_from_the_top_level() {
        let source = "class Enemies::Boss < Enemy\nend\n";

        let header = Header::read("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(scope(&header), Some(Vec::new()));
    }

    // @behavior RH-013
    #[test]
    fn icon_called_in_the_class_body_with_a_string_carries_that_path() {
        let source = "class Enemy < Godot::Node2D\n  icon \"icons/enemy.svg\"\nend\n";

        let header = Header::read("res://enemy.rb", source, &Roots::default());

        assert_eq!(header.icon(), Some("icons/enemy.svg"));
    }

    // @behavior RH-014
    #[test]
    fn the_classs_name_is_the_last_name_its_class_statement_writes() {
        let source = "class Net::HTTPClient < Godot::Node\nend\n";

        let header = Header::read("res://net/http_client.rb", source, &Roots::default());

        assert_eq!(header.name(), "HTTPClient");
    }

    // @behavior RH-015
    #[test]
    fn every_constant_a_module_or_class_statement_writes_is_carried() {
        let source = "module Enemies\n  class Boss < Enemy\n    class Loot\n    end\n  end\n\n  class ::Lamp\n  end\nend\n";

        let header = Header::read("res://enemies/boss.rb", source, &Roots::default());

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

        let header = Header::read("res://boss.rb", source, &Roots::default());

        assert_eq!(header.writes(), [vec!["Boss"]]);
    }

    // @behavior RH-017
    #[test]
    fn signal_called_in_the_class_body_carries_the_signal_it_declares() {
        let source = "class Bell < Godot::Node2D\n  signal :rung, :times\nend\n";

        let header = Header::read("res://bell.rb", source, &Roots::default());

        assert_eq!(
            header.signals(),
            [Signal {
                name: "rung".to_owned(),
                parameters: vec!["times".to_owned()],
            }]
        );
    }

    // @behavior RH-018
    #[test]
    fn a_signal_declared_with_a_name_that_is_not_written_out_is_not_carried() {
        let source = "class Bell < Godot::Node2D\n  name = :rung\n  signal name\nend\n";

        let header = Header::read("res://bell.rb", source, &Roots::default());

        assert!(header.signals().is_empty());
    }
}
