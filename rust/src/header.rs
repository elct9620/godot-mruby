//! Reads a Ruby file's source with Prism and never runs it, so what the file
//! says is known on any thread before its file has run.

use std::collections::BTreeMap;

use godot::builtin::{GString, StringName, VarArray, VarDictionary, Variant, VariantType};
use godot::meta::ToGodot;
use ruby_prism::{CallNode, DefNode, Integer, Node, NodeList, SingletonClassNode, Visit};

use crate::hint::Hint;
use crate::realm::{self, Roots};
use crate::snapshot::{
    self, Export, Heading, Member, MethodCalls, Parameter, Property, Rpc, Signal,
};

// The calls in a class body that define methods the source does not name
// with a `def`.
const METHOD_MAKERS: [&[u8]; 8] = [
    b"attr",
    b"attr_reader",
    b"attr_writer",
    b"attr_accessor",
    b"alias_method",
    b"define_method",
    b"include",
    b"prepend",
];

/// A file's header: the constants its `module` and `class` statements write,
/// the name the class its path names is written with, the superclass written
/// on it, the names of the methods it defines on its instances and on `self`,
/// what each method the file defines calls, the signals, rpcs, properties and
/// headings its body declares, and the `tool`, `abstract` and `icon` its body
/// calls.
#[derive(Debug, Default)]
pub struct Header {
    constant_paths: Vec<Vec<String>>,
    name: String,
    name_offset: usize,
    superclass: Option<Superclass>,
    methods: BTreeMap<String, usize>,
    class_methods: Vec<String>,
    method_calls: Vec<MethodCalls>,
    signal_offsets: BTreeMap<String, usize>,
    export_offsets: BTreeMap<String, usize>,
    signals: Vec<Signal>,
    signal_names: Vec<String>,
    exports: Vec<Export>,
    rpcs: Vec<Rpc>,
    digest: u64,
    is_tool: bool,
    is_abstract: bool,
    abstract_methods: Vec<String>,
    has_methods_without_def: bool,
    // The methods `class << self` or `define_singleton_method` defines,
    // which answer no Godot question but tell `abstract` a name is the
    // class's own.
    singleton_class_methods: Vec<String>,
    // The instance methods a call such as `attr_reader` or `alias` defines,
    // which tell `abstract` a name is the objects' as `def` would.
    methods_by_call: Vec<String>,
    is_parsed: bool,
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
    pub fn from_source(path: &str, source: &str, roots: &Roots) -> Self {
        let result = ruby_prism::parse(source.as_bytes());
        let mut reader = Reader {
            key: roots.key_by_path(path),
            header: None,
            constant_paths: Vec::new(),
        };
        if let Some(program) = result.node().as_program_node() {
            reader.read_body(&program.statements().body(), &[]);
        }
        let header = reader.header.unwrap_or_default();
        let mut collector = MethodCallCollector(Vec::new());
        collector.visit(&result.node());
        Header {
            constant_paths: reader.constant_paths,
            method_calls: collector.0,
            digest: snapshot::digest(source),
            is_parsed: result.errors().next().is_none(),
            icon: header
                .icon
                .as_deref()
                .map(|icon| resolve_icon_path(path, icon)),
            ..header
        }
    }

    /// Each constant a `module` or `class` statement writes, in the order
    /// they are written, as its names from the top level: `module Items` then
    /// `class Potion` inside it write `["Items"]` and `["Items", "Potion"]`.
    pub fn constant_paths(&self) -> &[Vec<String>] {
        &self.constant_paths
    }

    /// The class's own name as its class statement writes it, without the
    /// namespaces around it: `HTTPClient` for `class Net::HTTPClient`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The byte offset in the source of the `module` or `class` statement
    /// writing the class's own name.
    pub fn name_offset(&self) -> usize {
        self.name_offset
    }

    pub fn superclass(&self) -> Option<&Superclass> {
        self.superclass.as_ref()
    }

    pub fn has_method(&self, name: &str) -> bool {
        self.methods.contains_key(name)
    }

    /// The methods the class body defines on `self`, which GDScript calls on
    /// the script as a GDScript's static functions.
    pub fn class_methods(&self) -> &[String] {
        &self.class_methods
    }

    /// What each method the file defines calls, in any class or module of
    /// it, so a method that may wait is known through the methods it calls.
    pub fn method_calls(&self) -> &[MethodCalls] {
        &self.method_calls
    }

    /// The names of the methods the file's class defines, as its `def`
    /// statements and `abstract` write them.
    pub fn methods(&self) -> impl Iterator<Item = &str> {
        self.methods.keys().map(String::as_str)
    }

    /// Each method the file's class defines, with the byte offset in the
    /// source of the `def` or `abstract` writing it.
    pub fn method_offsets(&self) -> impl Iterator<Item = (&str, usize)> {
        self.methods
            .iter()
            .map(|(name, offset)| (name.as_str(), *offset))
    }

    /// Each signal the class declares, with the byte offset of its `signal`
    /// call in the source.
    pub fn signal_offsets(&self) -> impl Iterator<Item = (&str, usize)> {
        self.signal_offsets
            .iter()
            .map(|(name, offset)| (name.as_str(), *offset))
    }

    /// Each property the class exports, with the byte offset of its `export`
    /// call in the source.
    pub fn export_offsets(&self) -> impl Iterator<Item = (&str, usize)> {
        self.export_offsets
            .iter()
            .map(|(name, offset)| (name.as_str(), *offset))
    }

    /// The signals the class declares, in the order it declares them, as the
    /// `signal` calls of its body write them; a call writing a name it does
    /// not spell out declares none the header can read.
    /// The rpcs the class body declares with literals, as the file would
    /// declare them as it runs.
    pub fn rpcs(&self) -> &[Rpc] {
        &self.rpcs
    }

    pub fn signals(&self) -> &[Signal] {
        &self.signals
    }

    /// The names the class body's `signal` calls spell out, in the order
    /// they are written, whether or not the rest of each call is.
    pub fn signal_names(&self) -> &[String] {
        &self.signal_names
    }

    /// What the class declares for the editor, in the order it declares it,
    /// as its body writes it: the properties its `export` calls export, each
    /// with the hint its keyword spells out, and the headings its
    /// `export_group`, `export_subgroup` and `export_category` calls write.
    /// A value or a hint the call does not write out is the file's to work
    /// out as it runs, and so is a type `type:` names; a call not spelling
    /// out the name it exports declares nothing here.
    pub fn exports(&self) -> &[Export] {
        &self.exports
    }

    /// The names the class body's `export` calls spell out, in the order
    /// they are written, whether or not the rest of each call is.
    pub fn export_names(&self) -> impl Iterator<Item = &str> {
        self.exports.iter().filter_map(|export| match export {
            Export::Member(Member::Property(property)) => Some(property.name.as_str()),
            Export::Member(Member::Heading(_)) => None,
            Export::Name { name, .. } => Some(name.as_str()),
        })
    }

    /// The digest of the source the header was read from.
    pub fn digest(&self) -> u64 {
        self.digest
    }

    pub fn is_tool(&self) -> bool {
        self.is_tool
    }

    /// Whether the source parses, with no syntax error to stop the file as
    /// it runs.
    pub fn is_parsed(&self) -> bool {
        self.is_parsed
    }

    pub fn is_abstract(&self) -> bool {
        self.is_abstract
    }

    /// The methods `abstract` marks in the class body, as `abstract def hurt`
    /// or `abstract :hurt` writes them, in the order it marks them.
    pub fn abstract_methods(&self) -> &[String] {
        &self.abstract_methods
    }

    /// Whether the class body defines methods other than by `def`, as
    /// `attr_reader`, `alias_method` or `include` do, so the source does not
    /// name every method the class has.
    pub fn has_methods_without_def(&self) -> bool {
        self.has_methods_without_def
    }

    /// The icon's path from the project's root, as the string `icon` is
    /// called with names it.
    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    // What the class body's own statements say: the methods it defines on its
    // instances and the calls it makes itself; a class inside it or a method's
    // body says nothing of this class.
    fn read_class_body(&mut self, statements: &NodeList) {
        for node in statements.iter() {
            if let Some(def) = node.as_def_node() {
                self.read_method(&def);
            } else if let Some(call) = node.as_call_node() {
                self.read_call(&call);
            } else if let Some(alias) = node.as_alias_method_node() {
                self.has_methods_without_def = true;
                self.methods_by_call.extend(read_name(&alias.new_name()));
            } else if let Some(singleton) = node.as_singleton_class_node() {
                self.read_singleton_class(&singleton);
            }
        }
    }

    // The public methods a `class << self` written in the class body defines
    // on the class, by `def` or by a call naming them, as Ruby's
    // `singleton_methods` lists them.
    fn read_singleton_class(&mut self, singleton: &SingletonClassNode) {
        if singleton.expression().as_self_node().is_none() {
            return;
        }
        let Some(body) = singleton.body().and_then(|body| body.as_statements_node()) else {
            return;
        };
        for node in body.body().iter() {
            if let Some(def) = node.as_def_node() {
                // A `def self.` here defines on the singleton class's own
                // singleton class, so not on the class.
                if def.receiver().is_none() {
                    self.singleton_class_methods
                        .push(decode_text(def.name().as_slice()));
                }
            } else if let Some(call) = node.as_call_node() {
                self.read_singleton_class_call(&call);
            }
        }
    }

    // What a call inside `class << self` defines on the class: the names a
    // method maker on the singleton class gives, less those `private` or
    // `protected` hides; one on the class itself, written by its name, makes
    // its objects' methods or, by `define_singleton_method`, the class's own.
    fn read_singleton_class_call(&mut self, call: &CallNode) {
        let name = call.name().as_slice();
        match call.receiver() {
            None => match name {
                b"private" | b"protected" => {
                    let private_names: Vec<String> = call
                        .arguments()
                        .map(|arguments| {
                            arguments
                                .arguments()
                                .iter()
                                .filter_map(|node| read_name(&node))
                                .collect()
                        })
                        .unwrap_or_default();
                    self.singleton_class_methods
                        .retain(|method| !private_names.contains(method));
                }
                b"public" => {
                    for node in call
                        .arguments()
                        .iter()
                        .flat_map(|arguments| arguments.arguments().iter())
                    {
                        if let Some(def) = node.as_def_node().filter(|def| def.receiver().is_none())
                        {
                            self.singleton_class_methods
                                .push(decode_text(def.name().as_slice()));
                        } else if let Some(inner) = node.as_call_node() {
                            self.read_singleton_class_call(&inner);
                        }
                    }
                }
                // Here `define_singleton_method` defines on the singleton
                // class's own singleton class, so not on the class.
                b"define_singleton_method" => {}
                _ => self.singleton_class_methods.extend(read_method_names(call)),
            },
            Some(receiver) if receiver.as_self_node().is_some() => {
                if name != b"define_singleton_method" {
                    self.singleton_class_methods.extend(read_method_names(call));
                }
            }
            Some(receiver) if self.is_own_name(&receiver) => {
                if name == b"define_singleton_method" {
                    self.singleton_class_methods.extend(read_method_names(call));
                } else {
                    self.methods_by_call.extend(read_method_names(call));
                }
            }
            Some(_) => {}
        }
    }

    // Whether `node` is the class's own name written as a constant.
    fn is_own_name(&self, node: &Node) -> bool {
        node.as_constant_read_node()
            .is_some_and(|constant| decode_text(constant.name().as_slice()) == self.name)
    }

    // A method the class body defines, on its instances or on `self`.
    fn read_method(&mut self, def: &DefNode) {
        let name = decode_text(def.name().as_slice());
        match def.receiver() {
            None => {
                let offset = def.location().start_offset();
                self.methods.entry(name).or_insert(offset);
            }
            Some(receiver) if receiver.as_self_node().is_some() => {
                if !self.class_methods.contains(&name) {
                    self.class_methods.push(name);
                }
            }
            Some(_) => {}
        }
    }

    // Marks `name` abstract and takes it among the class's methods, as the
    // class has it once `abstract` defines it at `offset`; a name only the
    // class itself defines, by `def self.`, in `class << self` or by
    // `define_singleton_method`, stays the class's own, as `abstract` leaves
    // it.
    fn mark_abstract(&mut self, name: String, offset: usize) {
        let is_on_class =
            self.class_methods.contains(&name) || self.singleton_class_methods.contains(&name);
        if is_on_class && !self.methods.contains_key(&name) && !self.methods_by_call.contains(&name)
        {
            return;
        }
        if !self.abstract_methods.contains(&name) {
            self.abstract_methods.push(name.clone());
        }
        self.methods.entry(name).or_insert(offset);
    }

    fn read_call(&mut self, call: &CallNode) {
        let is_on_class = call.receiver().is_none_or(|receiver| {
            receiver.as_self_node().is_some() || self.is_own_name(&receiver)
        });
        if is_on_class && METHOD_MAKERS.contains(&call.name().as_slice()) {
            self.has_methods_without_def = true;
            self.methods_by_call.extend(read_method_names(call));
        }
        if is_on_class && call.name().as_slice() == b"define_singleton_method" {
            self.singleton_class_methods.extend(read_method_names(call));
        }
        if call.receiver().is_some() || call.block().is_some() {
            return;
        }
        let offset = call.location().start_offset();
        let arguments: Vec<Node> = call
            .arguments()
            .map(|arguments| arguments.arguments().iter().collect())
            .unwrap_or_default();
        match (call.name().as_slice(), arguments.as_slice()) {
            (b"private" | b"protected" | b"public", targets) => {
                for node in targets {
                    if let Some(def) = node.as_def_node() {
                        self.read_method(&def);
                    } else if let Some(call) = node.as_call_node() {
                        self.read_call(&call);
                    }
                }
            }
            (b"tool", []) => self.is_tool = true,
            (b"abstract", []) => self.is_abstract = true,
            // Ruby runs every argument before `abstract` is called, and a
            // `def` answers its name as a symbol would, so each `def` is
            // read before any name is marked.
            (b"abstract", methods) => {
                for def in methods.iter().filter_map(Node::as_def_node) {
                    self.read_method(&def);
                }
                for method in methods {
                    let name = match method.as_def_node() {
                        Some(def) => Some(decode_text(def.name().as_slice())),
                        None => read_name(method),
                    };
                    if let Some(name) = name {
                        self.mark_abstract(name, offset);
                    }
                }
            }
            (b"icon", [path]) => {
                if let Some(path) = path.as_string_node() {
                    self.icon = Some(decode_text(path.unescaped()));
                }
            }
            (b"signal", [name, parameters @ ..]) => {
                if let Some(name) = read_name(name) {
                    self.signal_offsets.entry(name.clone()).or_insert(offset);
                    self.signal_names.push(name);
                }
                if let Some(signal) = read_signal(name, parameters) {
                    self.signals.push(signal);
                }
            }
            (b"rpc", [name, keywords @ ..]) => {
                if let Some(rpc) = read_rpc(name, keywords) {
                    self.rpcs.push(rpc);
                }
            }
            (b"export", [name, default, keywords @ ..]) => {
                if let Some(name) = read_name(name) {
                    self.export_offsets.entry(name).or_insert(offset);
                }
                if let Some(export) = read_export(name, default, keywords) {
                    self.exports.push(export);
                }
            }
            (b"export_group" | b"export_subgroup", [name, prefix @ ..]) => {
                if let Some(heading) = read_group(call.name().as_slice(), name, prefix) {
                    self.exports.push(Export::Member(Member::Heading(heading)));
                }
            }
            (b"export_category", [name]) => {
                if let Some(name) = read_name(name) {
                    self.exports
                        .push(Export::Member(Member::Heading(Heading::Category {
                            name,
                            path: String::new(),
                        })));
                }
            }
            _ => {}
        }
    }
}

struct Reader {
    key: Vec<String>,
    header: Option<Header>,
    constant_paths: Vec<Vec<String>>,
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
        let Some(names) = resolve_constant_names(&path, scope) else {
            return;
        };
        self.constant_paths.push(names.clone());
        let statements = body
            .and_then(|body| body.as_statements_node())
            .map(|statements| statements.body());
        if self.header.is_none() && self.is_own_constant(&names) {
            let mut header = Header {
                name: names.last().cloned().unwrap_or_default(),
                name_offset: node.location().start_offset(),
                superclass: class
                    .and_then(|class| class.superclass())
                    .and_then(|superclass| read_superclass(&superclass, scope)),
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

    fn is_own_constant(&self, names: &[String]) -> bool {
        names.len() == self.key.len()
            && names
                .iter()
                .zip(&self.key)
                .all(|(name, segment)| realm::normalize(name) == *segment)
    }
}

// The names of the methods `call` makes without `def`, as its arguments
// write them: every name `attr` and its kin are given, the first for
// `alias_method`, `define_method` and `define_singleton_method`.
fn read_method_names(call: &CallNode) -> Vec<String> {
    let names: Vec<String> = call
        .arguments()
        .map(|arguments| {
            arguments
                .arguments()
                .iter()
                .filter_map(|argument| read_name(&argument))
                .collect()
        })
        .unwrap_or_default();
    match call.name().as_slice() {
        b"attr" | b"attr_reader" => names,
        b"attr_writer" => names.into_iter().map(|name| format!("{name}=")).collect(),
        b"attr_accessor" => names
            .into_iter()
            .flat_map(|name| [format!("{name}="), name])
            .collect(),
        b"alias_method" | b"define_method" | b"define_singleton_method" => {
            names.into_iter().take(1).collect()
        }
        _ => Vec::new(),
    }
}

/// The line, counted from 1, that the byte at `offset` of `source` is on.
pub fn locate_line(source: &str, offset: usize) -> u32 {
    let before = source.get(..offset).unwrap_or(source);
    u32::try_from(before.matches('\n').count() + 1).unwrap_or(u32::MAX)
}

// The signal a `signal` call declares, as it is written: the name it is
// emitted by and the keyword naming each value it carries, of any type until
// the file runs, since only a run knows what class a constant is. A name
// written as anything but a symbol or a string, or keywords spread from a
// value, are the file's to work out as it runs, so the header carries none of
// that declaration.
fn read_signal(name: &Node, parameters: &[Node]) -> Option<Signal> {
    let parameters = match parameters {
        [] => Vec::new(),
        [keywords] => keywords
            .as_keyword_hash_node()?
            .elements()
            .iter()
            .map(|element| {
                let pair = element.as_assoc_node()?;
                let keyword = pair.key().as_symbol_node()?;
                Some(Parameter::new(
                    decode_text(keyword.unescaped()),
                    VariantType::NIL,
                ))
            })
            .collect::<Option<_>>()?,
        _ => return None,
    };
    Some(Signal {
        name: read_name(name)?,
        parameters,
    })
}

// What an `export` call declares, as it is written: the property of the
// name Godot reads it by, the value it is declared with, which gives it its
// type, and the hint its keywords spell out. A value or a hint written as
// anything but a literal is the file's to work out as it runs, and a name
// written so is none the header knows.
fn read_export(name: &Node, default: &Node, keywords: &[Node]) -> Option<Export> {
    let name = read_name(name)?;
    let Some(value) = read_literal(default) else {
        return Some(Export::Name { name, bare: None });
    };
    let property = Property::new(name.clone(), &value);
    let Some(keywords) = keywords.first() else {
        return Some(Export::Member(Member::Property(property)));
    };
    Some(match with_keyword_hint(&property, keywords) {
        Some(hinted) => Export::Member(Member::Property(hinted)),
        None => Export::Name {
            name,
            bare: Some(property),
        },
    })
}

// `property` as the hint `keywords` spell out declares it, if they spell out
// one the property's type takes.
fn with_keyword_hint(property: &Property, keywords: &Node) -> Option<Property> {
    let (hint, written) = read_keyword_hint(keywords)?;
    hint.write_hint_string(&written)?;
    hint.declare(property.name.clone(), &property.default_value(), &written)
        .ok()
}

// What each `def` of a file calls, its blocks' calls included and a `def`
// written inside it apart.
struct MethodCallCollector(Vec<MethodCalls>);

impl<'pr> Visit<'pr> for MethodCallCollector {
    fn visit_def_node(&mut self, node: &DefNode<'pr>) {
        struct Calls<'a>(&'a mut MethodCalls);
        impl<'pr> Visit<'pr> for Calls<'_> {
            fn visit_call_node(&mut self, node: &CallNode<'pr>) {
                let name = decode_text(node.name().as_slice());
                if node.receiver().is_none() && name == "await" {
                    self.0.has_await = true;
                } else if !self.0.callees.contains(&name) {
                    self.0.callees.push(name);
                }
                ruby_prism::visit_call_node(self, node);
            }

            fn visit_def_node(&mut self, _node: &DefNode<'pr>) {}
        }

        let mut calls = MethodCalls {
            method: decode_text(node.name().as_slice()),
            ..MethodCalls::default()
        };
        if let Some(body) = node.body() {
            Calls(&mut calls).visit(&body);
        }
        self.0.push(calls);
        ruby_prism::visit_def_node(self, node);
    }
}

// The rpc an `rpc` call declares with a method's name and literal keywords
// for what `@rpc` takes, the ones it leaves out at `@rpc`'s defaults; none
// for words the file would refuse or keywords written as anything else.
fn read_rpc(name: &Node, keywords: &[Node]) -> Option<Rpc> {
    let method = read_name(name)?;
    let (mut mode, mut sync, mut transfer_mode, mut channel) = (
        "authority".to_owned(),
        "call_remote".to_owned(),
        "reliable".to_owned(),
        0,
    );
    if let Some(keywords) = keywords.first() {
        for element in keywords.as_keyword_hash_node()?.elements().iter() {
            let pair = element.as_assoc_node()?;
            let keyword = decode_text(pair.key().as_symbol_node()?.unescaped());
            let value = pair.value();
            match keyword.as_str() {
                "mode" => mode = read_name(&value)?,
                "sync" => sync = read_name(&value)?,
                "transfer_mode" => transfer_mode = read_name(&value)?,
                "transfer_channel" => channel = read_literal(&value)?.try_to::<i64>().ok()?,
                _ => return None,
            }
        }
    }
    Rpc::read(method, &mode, &sync, &transfer_mode, channel).ok()
}

// The one hint an export's keywords name and the value it is read with, as
// the running file would take them: a range's bounds and its step, or the
// literal the keyword is written with. Keywords the file would refuse, or
// write as anything but literals, spell out none.
fn read_keyword_hint(keywords: &Node) -> Option<(Hint, Variant)> {
    let mut step = None;
    let mut extra = None;
    let mut named = Vec::new();
    for element in keywords.as_keyword_hash_node()?.elements().iter() {
        let pair = element.as_assoc_node()?;
        let keyword = decode_text(pair.key().as_symbol_node()?.unescaped());
        if keyword == "step" {
            step = Some(read_literal(&pair.value())?);
        } else if keyword == "extra_hints" {
            extra = Some(read_literal(&pair.value())?);
        } else {
            named.push((keyword, pair.value()));
        }
    }
    let [(keyword, value)] = named.as_slice() else {
        return None;
    };
    let hint = Hint::from_keyword(keyword).ok()?;
    let written = match (hint, step, extra) {
        (Hint::Range, step, extra) => read_bounds(value, step, extra)?,
        (_, None, None) => read_literal(value)?,
        _ => return None,
    };
    Some((hint, written))
}

// A range's bounds, its step after them when one is written or extra hints
// follow, and the extra hints, as a range literal including its end writes
// them.
fn read_bounds(range: &Node, step: Option<Variant>, extra: Option<Variant>) -> Option<Variant> {
    let range = range.as_range_node()?;
    if range.is_exclude_end() {
        return None;
    }
    let mut bounds = VarArray::new();
    bounds.push(&read_literal(&range.left()?)?);
    bounds.push(&read_literal(&range.right()?)?);
    if let Some(step) = step
        .clone()
        .or_else(|| extra.as_ref().map(|_| 1.to_variant()))
    {
        bounds.push(&step);
    }
    match extra.map(|extra| extra.try_to::<VarArray>()) {
        Some(Ok(hints)) => hints
            .iter_shared()
            .for_each(|hint| bounds.push(&hint.to_string().to_variant())),
        Some(Err(_)) => return None,
        None => {}
    }
    Some(bounds.to_variant())
}

// The group or subgroup an `export_group` or `export_subgroup` call writes,
// with the prefix its properties are taken by when one is written.
fn read_group(call: &[u8], name: &Node, prefix: &[Node]) -> Option<Heading> {
    let name = read_name(name)?;
    let prefix = match prefix {
        [] => String::new(),
        [prefix] => read_name(prefix)?,
        _ => return None,
    };
    Some(match call {
        b"export_subgroup" => Heading::Subgroup { name, prefix },
        _ => Heading::Group { name, prefix },
    })
}

// The value a literal writes, as the engine takes it, following the Ruby
// value's own class as a value crossing from a running file does. A literal
// holding anything else, such as a constant or a call, writes none.
fn read_literal(node: &Node) -> Option<Variant> {
    if node.as_true_node().is_some() {
        return Some(true.to_variant());
    }
    if node.as_false_node().is_some() {
        return Some(false.to_variant());
    }
    if let Some(number) = node.as_integer_node() {
        return read_integer(&number.value()).map(|number| number.to_variant());
    }
    if let Some(number) = node.as_float_node() {
        return Some(number.value().to_variant());
    }
    if let Some(string) = node.as_string_node() {
        return Some(GString::from(&decode_text(string.unescaped())).to_variant());
    }
    if let Some(symbol) = node.as_symbol_node() {
        return Some(StringName::from(&decode_text(symbol.unescaped())).to_variant());
    }
    if let Some(array) = node.as_array_node() {
        let mut copied = VarArray::new();
        for element in array.elements().iter() {
            copied.push(&read_literal(&element)?);
        }
        return Some(copied.to_variant());
    }
    if let Some(hash) = node.as_hash_node() {
        let mut copied = VarDictionary::new();
        for element in hash.elements().iter() {
            let entry = element.as_assoc_node()?;
            copied.set(&read_literal(&entry.key())?, &read_literal(&entry.value())?);
        }
        return Some(copied.to_variant());
    }
    None
}

// The number an integer literal writes, unless it is larger than the
// engine's own integers hold.
fn read_integer(number: &Integer) -> Option<i64> {
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
fn read_name(node: &Node) -> Option<String> {
    if let Some(symbol) = node.as_symbol_node() {
        return Some(decode_text(symbol.unescaped()));
    }
    node.as_string_node()
        .map(|string| decode_text(string.unescaped()))
}

fn decode_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// An icon's path from the project's root: one written relative to the file's
// directory is taken from there, as GDScript's `@icon` is.
fn resolve_icon_path(file: &str, icon: &str) -> String {
    if is_absolute_path(icon) {
        simplify_path(icon)
    } else {
        let directory = file.rsplit_once('/').map_or("", |(directory, _)| directory);
        simplify_path(&format!("{directory}/{icon}"))
    }
}

// Whether `path` is absolute as Godot's `String::is_absolute_path` reads it:
// from the root, or under a scheme such as `res://` or a drive.
fn is_absolute_path(path: &str) -> bool {
    path.starts_with(['/', '\\']) || path.contains(":/") || path.contains(":\\")
}

// `path` as Godot's `String::simplify_path` writes it: its prefix kept, `.`
// and empty steps dropped, and each `..` taking the step before it.
fn simplify_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    let (prefix, rest) = match path.find(":/") {
        Some(at) if path[at..].starts_with("://") => path.split_at(at + 3),
        Some(at) => path.split_at(at + 2),
        None if path.starts_with('/') => path.split_at(1),
        None => path.split_at(0),
    };
    let mut steps: Vec<&str> = Vec::new();
    for step in rest.split('/') {
        match step {
            "" | "." => {}
            ".." => {
                steps.pop();
            }
            _ => steps.push(step),
        }
    }
    format!("{prefix}{}", steps.join("/"))
}

// A superclass the class statement inside `scope` writes, when it is a
// constant path.
fn read_superclass(superclass: &Node, scope: &[String]) -> Option<Superclass> {
    let (names, from_top) = read_constant_path(superclass)?;
    Some(Superclass {
        scope: if from_top { Vec::new() } else { scope.to_vec() },
        names,
    })
}

// The whole constant path a `module` or `class` statement inside `scope`
// defines: `class A::B` in `module Outer` is `Outer::A::B`, `class ::A` is `A`.
// None for a path whose parent is not a constant.
fn resolve_constant_names(path: &Node, scope: &[String]) -> Option<Vec<String>> {
    let (names, from_top) = read_constant_path(path)?;
    Some(if from_top {
        names
    } else {
        [scope, &names].concat()
    })
}

// A constant path as written, and whether it starts from the top level as
// `::A` does. None for a path whose parent is not a constant.
fn read_constant_path(node: &Node) -> Option<(Vec<String>, bool)> {
    if let Some(read) = node.as_constant_read_node() {
        return Some((vec![decode_text(read.name().as_slice())], false));
    }
    let path = node.as_constant_path_node()?;
    let (mut names, from_top) = match path.parent() {
        Some(parent) => read_constant_path(&parent)?,
        None => (Vec::new(), true),
    };
    names.push(decode_text(path.name()?.as_slice()));
    Some((names, from_top))
}

#[cfg(test)]
mod tests {
    use super::{Header, Parameter, Roots, Signal, VariantType};

    // The superclass's constant path, joined as it is written.
    fn join_superclass(header: &Header) -> Option<String> {
        header
            .superclass()
            .map(|superclass| superclass.names().join("::"))
    }

    fn read_scope(header: &Header) -> Option<Vec<String>> {
        header
            .superclass()
            .map(|superclass| superclass.scope().to_vec())
    }

    // @behavior RH-001
    #[test]
    fn a_class_written_inside_its_paths_namespaces_is_the_files_class() {
        let source = "module Enemies\n  class Boss < Godot::Node2D\n  end\nend\n";

        let header = Header::from_source("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(join_superclass(&header).as_deref(), Some("Godot::Node2D"));
    }

    // @behavior RH-002
    #[test]
    fn a_class_written_with_its_whole_constant_path_is_the_files_class() {
        let source = "class Enemies::Boss < Godot::Node2D\nend\n";

        let header = Header::from_source("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(join_superclass(&header).as_deref(), Some("Godot::Node2D"));
    }

    // @behavior RH-003
    #[test]
    fn a_class_matching_its_path_apart_from_underscores_and_case_is_the_files_class() {
        let source = "class HTTPClient < Godot::Node\nend\n";

        let header = Header::from_source("res://http_client.rb", source, &Roots::default());

        assert_eq!(join_superclass(&header).as_deref(), Some("Godot::Node"));
    }

    // @behavior RH-004
    #[test]
    fn a_method_of_a_class_nested_in_the_files_class_is_not_the_files() {
        let source = "class Player < Godot::Node\n  class Stats\n    def _ready\n    end\n  end\n\n  def _process(delta)\n  end\nend\n";

        let header = Header::from_source("res://player.rb", source, &Roots::default());

        assert!(!header.has_method("_ready"));
        assert!(header.has_method("_process"));
    }

    // @behavior RH-005
    #[test]
    fn a_file_that_does_not_parse_still_has_a_header() {
        let source = "class Player < Godot::Node\n  def _ready\n  end\nend\nend\n";

        let header = Header::from_source("res://player.rb", source, &Roots::default());

        assert_eq!(join_superclass(&header).as_deref(), Some("Godot::Node"));
        assert!(header.has_method("_ready"));
    }

    // @behavior RH-006
    #[test]
    fn a_module_file_has_no_superclass() {
        let source = "module Items\nend\n";

        let header = Header::from_source("res://items.rb", source, &Roots::default());

        assert_eq!(join_superclass(&header), None);
    }

    // @behavior RH-007
    #[test]
    fn a_superclass_that_is_not_a_constant_is_not_carried() {
        let source = "class Point < Struct.new(:x, :y)\nend\n";

        let header = Header::from_source("res://point.rb", source, &Roots::default());

        assert_eq!(join_superclass(&header), None);
    }

    // @behavior RH-008
    #[test]
    fn tool_called_in_the_class_body_makes_the_header_a_tools() {
        let source = "class Player < Godot::Node\n  tool\nend\n";

        let header = Header::from_source("res://player.rb", source, &Roots::default());

        assert!(header.is_tool());
    }

    // @behavior RH-009
    #[test]
    fn abstract_called_in_the_class_body_makes_the_header_an_abstract_classs() {
        let source = "class Enemy < Godot::Node2D\n  abstract\nend\n";

        let header = Header::from_source("res://enemy.rb", source, &Roots::default());

        assert!(header.is_abstract());
    }

    // @behavior RH-021
    #[test]
    fn abstract_given_methods_marks_them_and_the_header_holds_them() {
        let source = "class Shape < Godot::Node2D\n  abstract\n\n  abstract def draw\n  end\n\n  abstract :area, \"edges\"\n  abstract def self.count\n  end\n\n  def self.total\n  end\n  abstract :total\n\n  def self.sum\n  end\n\n  def sum\n  end\n  abstract :sum\nend\n";

        let header = Header::from_source("res://shape.rb", source, &Roots::default());

        assert_eq!(
            (
                header.methods().collect::<Vec<_>>(),
                header.abstract_methods(),
                header.class_methods()
            ),
            (
                vec!["area", "draw", "edges", "sum"],
                [
                    "draw".to_owned(),
                    "area".to_owned(),
                    "edges".to_owned(),
                    "sum".to_owned()
                ]
                .as_slice(),
                ["count".to_owned(), "total".to_owned(), "sum".to_owned()].as_slice()
            )
        );
    }

    // @behavior RH-022
    #[test]
    fn a_body_writing_methods_without_def_has_methods_the_header_cannot_name() {
        let sources = [
            "class Circle < Shape\n  attr_reader :area\nend\n",
            "class Circle < Shape\n  attr :area\nend\n",
            "class Circle < Shape\n  alias area size\nend\n",
            "class Circle < Shape\n  define_method(:area) { 1 }\nend\n",
            "class Circle < Shape\n  include Round\nend\n",
        ];

        let answers = sources.map(|source| {
            Header::from_source("res://circle.rb", source, &Roots::default())
                .has_methods_without_def()
        });

        assert_eq!(answers, [true; 5]);
    }

    // @behavior RH-023
    #[test]
    fn a_method_written_inside_a_visibility_call_is_the_class_bodys_own() {
        let source =
            "class Circle < Shape\n  private def area\n  end\n\n  private attr_reader :size\nend\n";

        let header = Header::from_source("res://circle.rb", source, &Roots::default());

        assert_eq!(
            (header.has_method("area"), header.has_methods_without_def()),
            (true, true)
        );
    }

    // @behavior RH-024
    #[test]
    fn abstract_judges_each_name_as_ruby_evaluates_the_call() {
        let source = "class Shape < Godot::Node2D\n  abstract\n\n  def x\n  end\n  abstract def self.x\n  end\n\n  abstract :y, def self.y\n  end\n\n  class << self\n    def z\n    end\n  end\n  abstract :z\nend\n";

        let header = Header::from_source("res://shape.rb", source, &Roots::default());

        assert_eq!(
            (header.abstract_methods(), header.class_methods()),
            (
                ["x".to_owned()].as_slice(),
                ["x".to_owned(), "y".to_owned()].as_slice()
            )
        );
    }

    // @behavior RH-025
    #[test]
    fn abstract_leaves_names_the_class_itself_makes_without_def() {
        let source = "class Shape < Godot::Node2D\n  abstract\n\n  class << self\n    attr_reader :w\n    attr_accessor :a\n  end\n  define_singleton_method(:s) { 1 }\n  self.define_singleton_method(:t) { 2 }\n  abstract :w, :a, :a=, :s, :t\nend\n";

        let header = Header::from_source("res://shape.rb", source, &Roots::default());

        assert!(header.abstract_methods().is_empty());
    }

    // @behavior RH-026
    #[test]
    fn a_method_define_singleton_method_makes_in_class_self_is_not_the_classs_own() {
        let source = "class Shape < Godot::Node2D\n  abstract\n\n  class << self\n    define_singleton_method(:x) { 1 }\n  end\n  abstract :x\nend\n";

        let header = Header::from_source("res://shape.rb", source, &Roots::default());

        assert_eq!(header.abstract_methods(), ["x".to_owned()].as_slice());
    }

    // @behavior RH-027
    #[test]
    fn abstract_judges_a_name_by_the_class_methods_ruby_lists() {
        let source = "class Seal < Godot::Node\n  abstract\n\n  class << self\n    def v\n    end\n    private :v\n\n    def self.r\n    end\n\n    Seal.define_singleton_method(:q) { 1 }\n  end\n\n  attr_reader :t\n\n  def self.t\n  end\n\n  abstract :v, :r, :t, :q\nend\n";

        let header = Header::from_source("res://seal.rb", source, &Roots::default());

        assert_eq!(
            header.abstract_methods(),
            ["v".to_owned(), "r".to_owned(), "t".to_owned()].as_slice()
        );
    }

    // @behavior RH-010
    #[test]
    fn a_call_inside_a_method_of_the_class_is_not_the_class_bodys() {
        let source = "class Player < Godot::Node\n  def setup\n    tool\n  end\nend\n";

        let header = Header::from_source("res://player.rb", source, &Roots::default());

        assert!(!header.is_tool());
    }

    // @behavior RH-011
    #[test]
    fn a_superclass_is_looked_up_from_the_namespaces_its_class_is_written_in() {
        let source = "module Enemies\n  class Boss < Enemy\n  end\nend\n";

        let header = Header::from_source("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(read_scope(&header), Some(vec!["Enemies".to_owned()]));
    }

    // @behavior RH-012
    #[test]
    fn a_superclass_on_a_class_written_with_its_whole_path_is_looked_up_from_the_top_level() {
        let source = "class Enemies::Boss < Enemy\nend\n";

        let header = Header::from_source("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(read_scope(&header), Some(Vec::new()));
    }

    // @behavior RH-013
    #[test]
    fn icon_called_in_the_class_body_with_a_string_carries_that_path() {
        let source = "class Enemy < Godot::Node2D\n  icon \"icons/enemy.svg\"\nend\n";

        let header = Header::from_source("res://enemy.rb", source, &Roots::default());

        assert_eq!(header.icon(), Some("res://icons/enemy.svg"));
    }

    // @behavior RH-019
    #[test]
    fn an_icon_written_relative_to_the_file_is_taken_from_the_files_directory() {
        let source = "class Enemies::Boss < Godot::Node2D\n  icon \"../icons/./boss.svg\"\nend\n";

        let header = Header::from_source("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(header.icon(), Some("res://icons/boss.svg"));
    }

    // @behavior RH-020
    #[test]
    fn an_icon_written_from_the_projects_root_keeps_its_path() {
        let source = "class Enemies::Boss < Godot::Node2D\n  icon \"res://icons//boss.svg\"\nend\n";

        let header = Header::from_source("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(header.icon(), Some("res://icons/boss.svg"));
    }

    // @behavior RH-014
    #[test]
    fn the_classs_name_is_the_last_name_its_class_statement_writes() {
        let source = "class Net::HTTPClient < Godot::Node\nend\n";

        let header = Header::from_source("res://net/http_client.rb", source, &Roots::default());

        assert_eq!(header.name(), "HTTPClient");
    }

    // @behavior RH-015
    #[test]
    fn every_constant_a_module_or_class_statement_writes_is_carried() {
        let source = "module Enemies\n  class Boss < Enemy\n    class Loot\n    end\n  end\n\n  class ::Lamp\n  end\nend\n";

        let header = Header::from_source("res://enemies/boss.rb", source, &Roots::default());

        assert_eq!(
            header.constant_paths(),
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

        let header = Header::from_source("res://boss.rb", source, &Roots::default());

        assert_eq!(header.constant_paths(), [vec!["Boss"]]);
    }

    // @behavior RH-017
    #[test]
    fn signal_called_in_the_class_body_carries_the_signal_it_declares() {
        let source = "class Bell < Godot::Node2D\n  signal :rung, times: Integer\nend\n";

        let header = Header::from_source("res://bell.rb", source, &Roots::default());

        assert_eq!(
            header.signals(),
            [Signal {
                name: "rung".to_owned(),
                parameters: vec![Parameter::new("times".to_owned(), VariantType::NIL)],
            }]
        );
    }

    // @behavior RH-018
    #[test]
    fn a_signal_declared_with_a_name_that_is_not_written_out_is_not_carried() {
        let source = "class Bell < Godot::Node2D\n  name = :rung\n  signal name\nend\n";

        let header = Header::from_source("res://bell.rb", source, &Roots::default());

        assert!(header.signals().is_empty());
    }
}
