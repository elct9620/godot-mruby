//! Assignments to a member of a member, rewritten before mruby compiles a
//! project's file, so they write each value type they change back into what
//! it was read from through the `Godot` gem's helpers, as GDScript's do: `position.y = 5` reads the position, sets `y`
//! on its copy and writes the copy back, while a value never changes.
//! Lines stay where they were; columns inside a rewritten one shift.

use std::borrow::Cow;
use std::ops::Range;

use ruby_prism::{ArgumentsNode, CallNode, Node, Visit};

/// `source` with every assignment to a member of a member rewritten, or as
/// it is when there is none or Prism cannot parse it, so mruby reports the
/// error itself.
pub fn rewrite(source: &str) -> Cow<'_, str> {
    let result = ruby_prism::parse(source.as_bytes());
    if result.errors().next().is_some() {
        return Cow::Borrowed(source);
    }
    let mut finder = WriteFinder(Vec::new());
    finder.visit(&result.node());
    if finder.0.is_empty() {
        return Cow::Borrowed(source);
    }
    let mut writes = finder.0;
    writes.sort_by_key(|write| write.span.start);
    Cow::Owned(render(source, 0..source.len(), &writes))
}

// What a chain of reads starts from.
#[derive(Clone)]
enum Root {
    // The object a receiverless read is called on.
    ImplicitSelf,
    // A variable, which a value type that changed is written back into.
    Variable(Range<usize>),
    // Anything else, which nothing holds to write back into.
    Expression(Range<usize>),
}

// A link of a chain: a member's name, or the source of an index's arguments.
#[derive(Clone)]
enum Step {
    Member(String),
    Index(Range<usize>),
}

// Where an assignment ends up: what its chain starts from, what it reads on
// the way, and what it writes.
#[derive(Clone)]
struct Target {
    root: Root,
    path: Vec<Step>,
    step: Step,
}

enum Form {
    // `target = value`.
    Assign(Range<usize>),
    // `target op= value`, `||=` and `&&=` taken as `or` and `and`.
    Operator(String, Range<usize>),
    // `t1, t2 = values`, each target `None` for a variable it assigns as is.
    Multiple(Vec<(Option<Target>, Range<usize>)>, Range<usize>, bool),
}

struct Write {
    span: Range<usize>,
    target: Option<Target>,
    form: Form,
}

struct WriteFinder(Vec<Write>);

impl<'pr> Visit<'pr> for WriteFinder {
    fn visit_call_node(&mut self, node: &CallNode<'pr>) {
        if let Some(write) = read_call_write(node) {
            self.0.push(write);
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        if let Some(receiver) = node.receiver().filter(|_| !node.is_safe_navigation()) {
            let member = Step::Member(decode(node.read_name().as_slice()));
            let operator = decode(node.binary_operator().as_slice());
            self.push_update(&node.location(), receiver, member, operator, &node.value());
        }
        ruby_prism::visit_call_operator_write_node(self, node);
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        if let Some(receiver) = node.receiver().filter(|_| !node.is_safe_navigation()) {
            let member = Step::Member(decode(node.read_name().as_slice()));
            self.push_update(
                &node.location(),
                receiver,
                member,
                "or".to_owned(),
                &node.value(),
            );
        }
        ruby_prism::visit_call_or_write_node(self, node);
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        if let Some(receiver) = node.receiver().filter(|_| !node.is_safe_navigation()) {
            let member = Step::Member(decode(node.read_name().as_slice()));
            self.push_update(
                &node.location(),
                receiver,
                member,
                "and".to_owned(),
                &node.value(),
            );
        }
        ruby_prism::visit_call_and_write_node(self, node);
    }

    fn visit_index_operator_write_node(&mut self, node: &ruby_prism::IndexOperatorWriteNode<'pr>) {
        if let (Some(receiver), Some(index), None) = (
            node.receiver(),
            read_arguments(node.arguments()),
            node.block(),
        ) {
            let operator = decode(node.binary_operator().as_slice());
            self.push_update(
                &node.location(),
                receiver,
                Step::Index(index),
                operator,
                &node.value(),
            );
        }
        ruby_prism::visit_index_operator_write_node(self, node);
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        if let Some(write) = read_multiple_write(node) {
            self.0.push(write);
        }
        ruby_prism::visit_multi_write_node(self, node);
    }
}

impl WriteFinder {
    fn push_update(
        &mut self,
        location: &ruby_prism::Location,
        receiver: Node,
        step: Step,
        operator: String,
        value: &Node,
    ) {
        if let Some(target) = read_target(receiver, step) {
            self.0.push(Write {
                span: span(location),
                target: Some(target),
                form: Form::Operator(operator, span(&value.location())),
            });
        }
    }
}

// `receiver.m = value` or `receiver[i] = value`, when the receiver is a
// chain whose links may be value types.
fn read_call_write(node: &CallNode) -> Option<Write> {
    if !node.is_attribute_write() || node.is_safe_navigation() || node.block().is_some() {
        return None;
    }
    let receiver = node.receiver()?;
    let arguments: Vec<Node> = node.arguments()?.arguments().iter().collect();
    let (value, index) = arguments.split_last()?;
    let name = decode(node.name().as_slice());
    let step = if name == "[]=" {
        let (first, last) = (index.first()?, index.last()?);
        Step::Index(first.location().start_offset()..last.location().end_offset())
    } else {
        Step::Member(name.strip_suffix('=')?.to_owned())
    };
    Some(Write {
        span: span(&node.location()),
        target: Some(read_target(receiver, step)?),
        form: Form::Assign(span(&value.location())),
    })
}

// `t1, t2 = values` when some target is a member that may be a value type's
// and every other is a variable.
fn read_multiple_write(node: &ruby_prism::MultiWriteNode) -> Option<Write> {
    if node.rest().is_some() || node.rights().iter().next().is_some() {
        return None;
    }
    let mut targets = Vec::new();
    for target in node.lefts().iter() {
        let range = span(&target.location());
        if let Some(call) = target.as_call_target_node() {
            if call.is_safe_navigation() {
                return None;
            }
            let member = decode(call.name().as_slice());
            let step = Step::Member(member.strip_suffix('=')?.to_owned());
            targets.push((read_target(call.receiver(), step), range));
        } else if target.as_local_variable_target_node().is_some()
            || target.as_instance_variable_target_node().is_some()
        {
            targets.push((None, range));
        } else {
            return None;
        }
    }
    if targets.iter().all(|(target, _)| target.is_none()) {
        return None;
    }
    let value = node.value();
    let is_implicit_array = value
        .as_array_node()
        .is_some_and(|array| array.opening_loc().is_none());
    Some(Write {
        span: span(&node.location()),
        target: None,
        form: Form::Multiple(targets, span(&value.location()), is_implicit_array),
    })
}

// The target writing `step` of what `receiver` reads, when one of its links
// may be a value type's: the chain reads a member, or starts from a variable
// whose member is written. An index written of an index read is a grid's,
// left as Ruby writes it.
fn read_target(receiver: Node, step: Step) -> Option<Target> {
    let (root, path) = read_chain(receiver);
    let qualifies = match &step {
        Step::Member(_) => !path.is_empty() || matches!(root, Root::Variable(_)),
        Step::Index(_) => matches!(path.last(), Some(Step::Member(_))),
    };
    qualifies.then_some(Target { root, path, step })
}

// What `node` reads, as the root it starts from and its links in order.
fn read_chain(node: Node) -> (Root, Vec<Step>) {
    let mut path = Vec::new();
    let mut current = node;
    let root = loop {
        if let Some(call) = current.as_call_node() {
            if let Some(member) = read_member_read(&call) {
                path.push(Step::Member(member));
                match call.receiver() {
                    Some(receiver) => current = receiver,
                    None => break Root::ImplicitSelf,
                }
                continue;
            }
            if let (Some(receiver), Some(index)) = (call.receiver(), read_index_read(&call)) {
                path.push(Step::Index(index));
                current = receiver;
                continue;
            }
        }
        let range = span(&current.location());
        let is_variable = current.as_local_variable_read_node().is_some()
            || current.as_instance_variable_read_node().is_some()
            || current.as_global_variable_read_node().is_some()
            || current.as_class_variable_read_node().is_some();
        break if is_variable {
            Root::Variable(range)
        } else {
            Root::Expression(range)
        };
    };
    path.reverse();
    (root, path)
}

// The member a call reads, when it is a plain read of one: a name taking no
// arguments and no block.
fn read_member_read(call: &CallNode) -> Option<String> {
    if call.arguments().is_some() || call.block().is_some() || call.is_safe_navigation() {
        return None;
    }
    let name = decode(call.name().as_slice());
    let is_identifier = name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    is_identifier.then_some(name)
}

// The source of the arguments an index read takes.
fn read_index_read(call: &CallNode) -> Option<Range<usize>> {
    if decode(call.name().as_slice()) != "[]" || call.block().is_some() || call.is_safe_navigation()
    {
        return None;
    }
    read_arguments(call.arguments())
}

fn read_arguments(arguments: Option<ArgumentsNode>) -> Option<Range<usize>> {
    let arguments: Vec<Node> = arguments?.arguments().iter().collect();
    let (first, last) = (arguments.first()?, arguments.last()?);
    Some(first.location().start_offset()..last.location().end_offset())
}

// `range` of `source` with each write inside it rewritten, the outermost of
// any written inside another taking the inner one with it. The writes are in
// the order they start, so only those starting inside the range are looked
// at.
fn render(source: &str, range: Range<usize>, writes: &[Write]) -> String {
    let mut rendered = String::new();
    let mut at = range.start;
    let first = writes.partition_point(|write| write.span.start < range.start);
    for write in writes[first..]
        .iter()
        .take_while(|write| write.span.start < range.end)
    {
        if write.span.start < at || write.span.end > range.end {
            continue;
        }
        rendered.push_str(&source[at..write.span.start]);
        rendered.push_str(&emit(source, write, writes));
        at = write.span.end;
    }
    rendered.push_str(&source[at..range.end]);
    rendered
}

// The Ruby a write is rewritten to, ending with the newlines the parts of it
// left out held, so every line after it keeps its number.
fn emit(source: &str, write: &Write, writes: &[Write]) -> String {
    let mut used: Vec<Range<usize>> = Vec::new();
    let mut part = |range: &Range<usize>| {
        if !used.contains(range) {
            used.push(range.clone());
        }
        render(source, range.clone(), writes)
    };
    let written = match (&write.form, &write.target) {
        (Form::Assign(value), Some(target)) => {
            let value = format!("({})", part(value));
            emit_assign(target, &value, &mut part)
        }
        (Form::Operator(operator, value), Some(target)) => {
            let value = part(value);
            emit_update(target, operator, &value, &mut part)
        }
        (Form::Multiple(targets, value, is_implicit_array), _) => {
            let values = part(value);
            let values = if *is_implicit_array {
                format!("[{values}]")
            } else {
                values
            };
            let assigns: Vec<String> = targets
                .iter()
                .enumerate()
                .map(|(index, (target, range))| {
                    let value = format!("__godot_values__[{index}]");
                    match target {
                        Some(target) => emit_assign(target, &value, &mut part),
                        None => format!("{} = {value}", part(range)),
                    }
                })
                .collect();
            format!(
                "(__godot_values__ = ::Godot::Value.__splat__({values}); {}; __godot_values__)",
                assigns.join("; ")
            )
        }
        _ => unreachable!("a write other than a multiple one has a target"),
    };
    let newlines = |range: &Range<usize>| source[range.clone()].matches('\n').count();
    let dropped = newlines(&write.span) - used.iter().map(newlines).sum::<usize>();
    // Every form ends with the bracket closing it, before which a newline
    // stands as anywhere inside one.
    let (body, closing) = written.split_at(written.len() - 1);
    format!("{body}{}{closing}", "\n".repeat(dropped))
}

fn emit_assign(
    target: &Target,
    value: &str,
    part: &mut impl FnMut(&Range<usize>) -> String,
) -> String {
    let path = emit_path(&target.path, part);
    let step = emit_step(&target.step, part);
    match &target.root {
        Root::Variable(range) => {
            let variable = part(range);
            if let (true, Step::Member(member)) = (target.path.is_empty(), &target.step) {
                format!(
                    "(__godot_value__ = {value}; ::Godot::Value === {variable} && !(::Godot::Value::Shared === {variable}) ? ({variable} = {variable}.with({member}: __godot_value__); __godot_value__) : ({variable}.{member} = __godot_value__))"
                )
            } else {
                format!(
                    "(__godot_value__ = {value}; {variable} = ::Godot::Value.__write_chain__({variable}, {path}, {step}, __godot_value__); __godot_value__)"
                )
            }
        }
        root => match (target.path.as_slice(), &target.step) {
            ([Step::Member(reader)], Step::Member(member)) => format!(
                "::Godot::Value.__write_member__({}, :{reader}, :{reader}=, :{member}, :{member}=, {value})",
                emit_root(root, part)
            ),
            _ => format!(
                "::Godot::Value.__write_through__({}, {path}, {step}, {value})",
                emit_root(root, part)
            ),
        },
    }
}

fn emit_update(
    target: &Target,
    operator: &str,
    value: &str,
    part: &mut impl FnMut(&Range<usize>) -> String,
) -> String {
    let path = emit_path(&target.path, part);
    let step = emit_step(&target.step, part);
    let (short, block) = match operator {
        "or" => (", :or", format!("({value})")),
        "and" => (", :and", format!("({value})")),
        _ => ("", format!("__godot_current__ {operator} ({value})")),
    };
    let update = |root: &str| {
        format!(
            "::Godot::Value.__update_chain__({root}, {path}, {step}{short}) {{ |__godot_current__| {block} }}"
        )
    };
    match &target.root {
        Root::Variable(range) => {
            let variable = part(range);
            format!("({variable} = {})", update(&variable))
        }
        root => update(&emit_root(root, part)),
    }
}

fn emit_root(root: &Root, part: &mut impl FnMut(&Range<usize>) -> String) -> String {
    match root {
        Root::ImplicitSelf => "self".to_owned(),
        Root::Variable(range) | Root::Expression(range) => format!("({})", part(range)),
    }
}

fn emit_path(path: &[Step], part: &mut impl FnMut(&Range<usize>) -> String) -> String {
    let steps: Vec<String> = path.iter().map(|step| emit_step(step, part)).collect();
    format!("[{}]", steps.join(", "))
}

fn emit_step(step: &Step, part: &mut impl FnMut(&Range<usize>) -> String) -> String {
    match step {
        Step::Member(name) => format!(":{name}"),
        Step::Index(range) => format!("[{}]", part(range)),
    }
}

fn span(location: &ruby_prism::Location) -> Range<usize> {
    location.start_offset()..location.end_offset()
}

fn decode(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::rewrite;
    use crate::compiler;

    // @behavior RR-008
    #[test]
    fn a_line_after_a_member_write_keeps_its_number() {
        let source = "position\n  .y = 5\nnode.position.x += [\n  1,\n].sum\nv.x, w.y =\n  1, 2\nraise \"here\"\n";

        let rewritten = rewrite(source);

        assert_ne!(rewritten, source);
        assert_eq!(rewritten.lines().count(), source.lines().count());
        assert!(
            rewritten
                .lines()
                .nth(7)
                .is_some_and(|line| line.contains("raise"))
        );
        assert!(
            compiler::diagnose(c"res://moved.rb", &rewritten)
                .error
                .is_none()
        );
    }

    #[test]
    fn a_write_no_value_type_can_take_is_left_as_written() {
        let source = "@hp = 3\nself.speed = 2\ngrid[y][x] = 1\nlist = []\nlist[0] = 4\nnode&.position.x = 1\n";

        assert_eq!(rewrite(source), source);
    }

    #[test]
    fn a_file_that_does_not_parse_is_left_for_mruby_to_report() {
        let source = "position.y = (\n";

        assert_eq!(rewrite(source), source);
    }
}
