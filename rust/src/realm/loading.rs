use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use beni::{
    DataType, Error, FromValue, Gem, IntoValue, Module, Mrb, RClass, RModule, Symbol, Value, method,
};
use godot::classes::FileAccess;

use super::index::{self, ClassIndex, Key, Named, Namespace};
use super::{load, refused};

/// How far a file has run in a realm.
#[derive(Clone, Copy)]
enum Run {
    Running,
    Done,
    /// It raised; by name it runs again, by path it does not.
    Failed,
}

/// A file running now, and the constants it has created so far, each as the
/// names of its namespace and its own.
struct Frame {
    path: String,
    created: Vec<(Vec<String>, String)>,
}

/// Loading by name: `Module#const_missing` asks the class index first and
/// `Module#const_added` tells the realm what a running file creates, both from
/// a module prepended to `Module` that leaves the rest to Ruby through
/// `super`. Every realm installs it.
///
/// It is also what it knows — the class index, how far each file has run, the
/// files running now — carried by the `mrb_state` itself, so Ruby's calls back
/// reach it from the state they are given, as an mruby gem keeps its data.
#[derive(Default)]
pub struct ByName {
    index: RefCell<ClassIndex>,
    files: RefCell<HashMap<String, Run>>,
    frames: RefCell<Vec<Frame>>,
    // Set while the realm itself defines a directory's module, which is no
    // file's to take away.
    making_namespace: Cell<bool>,
}

static STATE: DataType<ByName> = DataType::new(c"class index");

// Where the state hangs: an instance variable of Module whose name Ruby
// cannot spell, as mruby keeps its own `__outer__`.
const STATE_NAME: &[u8] = b"__class_index__";

impl Gem for ByName {
    fn init(mrb: &Mrb) -> Result<(), Error> {
        let module = mrb.class_get(c"Module")?;
        let carrier = mrb.class_new(mrb.object_class())?;
        carrier.set_instance_data_tt(mrb)?;
        let state = carrier.data_wrap(mrb, ByName::default(), &STATE)?;
        module
            .to_value(mrb)
            .iv_set(mrb, mrb.intern(STATE_NAME).to_sym(), state)?;
        module.define_private_method(mrb, c"__load_by_name__", method!(load_by_name, 1))?;
        module.define_private_method(mrb, c"__created__", method!(created, 1))?;
        load(mrb, "godot_mruby/by_name.rb", include_str!("by_name.rb"))
    }
}

/// A realm's loading by name: its `mrb_state` and the state installed there.
pub(super) struct Loading<'a> {
    mrb: &'a Mrb,
    state: &'a ByName,
}

// Module#__load_by_name__(name): the constant in a one-element array, or nil
// when the class index names no file for it.
fn load_by_name(mrb: &Mrb, receiver: Value, name: Symbol) -> Result<Value, Error> {
    let (Some(loading), Some(name)) = (Loading::of(mrb), name.name(mrb)) else {
        return Ok(Value::nil());
    };
    Ok(match loading.resolve(receiver, &name)? {
        Some(constant) => mrb.ary_new_from_values(&[constant]).as_value(),
        None => Value::nil(),
    })
}

// Module#__created__(name): the receiver has just been given the constant.
fn created(mrb: &Mrb, receiver: Value, name: Symbol) -> Value {
    if let (Some(loading), Some(name)) = (Loading::of(mrb), name.name(mrb)) {
        loading.record(receiver, name);
    }
    Value::nil()
}

impl<'a> Loading<'a> {
    /// The loading state installed in `mrb`, if `ByName` has been.
    pub(super) fn of(mrb: &'a Mrb) -> Option<Self> {
        let module = mrb.class_get(c"Module").ok()?.to_value(mrb);
        let state = module
            .iv_get(mrb, mrb.intern(STATE_NAME).to_sym())
            .data_get(mrb, &STATE)?;
        Some(Self { mrb, state })
    }

    /// Adds the files at `paths` to the class index, each named from `res://`.
    pub(super) fn index_files(&self, paths: impl IntoIterator<Item = String>) {
        self.state
            .index
            .borrow_mut()
            .add(paths, |key| self.constant_at(key).is_some());
    }

    // The constant `key` spells, if it is already here, matched the way the
    // class index matches it.
    fn constant_at(&self, key: &[String]) -> Option<Value> {
        key.iter().try_fold(self.object(), |scope, segment| {
            self.constant_matching(scope, segment)
        })
    }

    fn object(&self) -> Value {
        self.mrb.object_class().to_value(self.mrb)
    }

    // The constant `scope` holds whose name the class index matches to
    // `segment`.
    fn constant_matching(&self, scope: Value, segment: &str) -> Option<Value> {
        let constants = scope
            .funcall(self.mrb, c"constants", &[])
            .and_then(|constants| constants.ensure_array(self.mrb))
            .ok()?;
        let name = (0..constants.len())
            .map(|index| constants.entry(index as isize))
            .find(|name| index::normalize(&name.to_string(self.mrb)) == segment)?;
        let name = name.to_sym(self.mrb).ok()?.to_sym();
        scope.const_get(self.mrb, name).ok()
    }

    /// Runs the file at `path` by path: once, whether it succeeded or not.
    pub(super) fn run(&self, path: &str) -> Result<(), Error> {
        if self.state.files.borrow().contains_key(path) {
            return Ok(());
        }
        self.execute(path)
    }

    // Reads and runs the file at `path`, after the namespaces its path passes
    // through when the class index names it; a test file is not one it names.
    fn load_file(&self, path: &str) -> Result<(), Error> {
        if self.state.index.borrow().names(path) {
            self.ensure_namespaces(path)?;
        }
        if !FileAccess::file_exists(path) {
            return Err(refused(self.mrb, "the file does not exist"));
        }
        let source = FileAccess::get_file_as_string(path).to_string();
        load(self.mrb, path, &source)
    }

    // What `name` names from inside `receiver`, looked for from the innermost
    // namespace outward, as Rails' classic autoloader does: mruby hands
    // const_missing the innermost scope alone.
    fn resolve(&self, receiver: Value, name: &str) -> Result<Option<Value>, Error> {
        let scope = self.scope_of(receiver);
        for depth in (0..=scope.len()).rev() {
            let outer = &scope[..depth];
            let mut key: Key = outer
                .iter()
                .map(|segment| index::normalize(segment))
                .collect();
            key.push(index::normalize(name));
            let named = self.state.index.borrow().named(&key);
            match named {
                Some(Named::File(path)) => return self.constant_from(&path, outer, name).map(Some),
                Some(Named::Namespace(namespace)) => {
                    return self.namespace(outer, name, &namespace).map(Some);
                }
                None => {}
            }
        }
        Ok(None)
    }

    // The names of the namespaces `receiver` sits in, outermost first, then
    // its own; none for Object or a module without a name.
    fn scope_of(&self, receiver: Value) -> Vec<String> {
        self.path_of(receiver).unwrap_or_default()
    }

    // As `scope_of`, with nothing for a module without a name, since no name
    // leads back to it.
    fn path_of(&self, receiver: Value) -> Option<Vec<String>> {
        let path = RClass::from_value(receiver)
            .and_then(|class| class.path(self.mrb))
            .or_else(|| RModule::from_value(receiver).and_then(|module| module.path(self.mrb)))?;
        Some(match path.as_str() {
            "Object" => Vec::new(),
            path => path.split("::").map(str::to_owned).collect(),
        })
    }

    // Runs the file at `path`, which has to have defined `name` in the
    // namespace `outer` spells.
    // A file that raised runs again, as a failed `require` does in Ruby: it
    // took away what it created, so it starts over.
    fn constant_from(&self, path: &str, outer: &[String], name: &str) -> Result<Value, Error> {
        let running = matches!(self.state.files.borrow().get(path), Some(Run::Running));
        if running {
            return Err(self.name_error(&self.cycle(path, outer, name), name));
        }
        self.run_by_name(path)
            .map_err(|error| self.placed(path, error))?;
        let scope = self.named_scope(outer)?;
        let symbol = self.mrb.intern(name.as_bytes()).to_sym();
        if scope.const_defined_at(self.mrb, symbol) {
            scope.const_get(self.mrb, symbol)
        } else {
            let message = format!("{path} ran without defining {}", qualified(outer, name));
            Err(self.name_error(&message, name))
        }
    }

    // A directory's module, which answers only to the name Zeitwerk gives it.
    fn namespace(
        &self,
        outer: &[String],
        name: &str,
        namespace: &Namespace,
    ) -> Result<Value, Error> {
        if name != namespace.name {
            let message = format!(
                "{} is the namespace {}, not {}",
                namespace.directory,
                qualified(outer, &namespace.name),
                qualified(outer, name)
            );
            return Err(self.name_error(&message, name));
        }
        let scope = self.named_scope(outer)?;
        self.define_module(scope, &namespace.name)
    }

    /// Makes sure each namespace the file at `path` sits in exists before it
    /// runs: the namespace's own file runs, or its directory becomes an empty
    /// module. A `module` statement never asks const_missing, so a file that
    /// opened its namespace first would make a module unrelated to it.
    fn ensure_namespaces(&self, path: &str) -> Result<(), Error> {
        let key = index::key_of(path);
        for depth in 1..key.len() {
            if self.constant_at(&key[..depth]).is_some() {
                continue;
            }
            let named = self.state.index.borrow().named(&key[..depth]);
            match named {
                Some(Named::File(file)) => self.run_by_name(&file)?,
                Some(Named::Namespace(namespace)) => {
                    // A namespace file that ran without defining its module
                    // leaves the files inside to open it themselves.
                    let Some(scope) = self.constant_at(&key[..depth - 1]) else {
                        return Ok(());
                    };
                    self.define_module(scope, &namespace.name)?;
                }
                None => {}
            }
        }
        Ok(())
    }

    // The module the names in `outer` spell exactly, from Object.
    fn named_scope(&self, outer: &[String]) -> Result<Value, Error> {
        outer.iter().try_fold(self.object(), |scope, name| {
            scope.const_get(self.mrb, self.mrb.intern(name.as_bytes()).to_sym())
        })
    }

    fn define_module(&self, scope: Value, name: &str) -> Result<Value, Error> {
        let module = self.mrb.module_new().to_value(self.mrb);
        self.state.making_namespace.set(true);
        let defined = scope.const_set(self.mrb, self.mrb.intern(name.as_bytes()).to_sym(), module);
        self.state.making_namespace.set(false);
        defined.map(|()| module)
    }

    // A file needed by name runs unless it has run cleanly or is running now.
    fn run_by_name(&self, path: &str) -> Result<(), Error> {
        match self.state.files.borrow().get(path) {
            Some(Run::Done | Run::Running) => return Ok(()),
            Some(Run::Failed) | None => {}
        }
        self.execute(path)
    }

    /// Runs the file at `path` whatever it did before, keeping what it
    /// creates when it succeeds and taking all of it away when it raises.
    /// What a file it loads by name creates is that file's own.
    fn execute(&self, path: &str) -> Result<(), Error> {
        self.state
            .files
            .borrow_mut()
            .insert(path.to_owned(), Run::Running);
        self.state.frames.borrow_mut().push(Frame {
            path: path.to_owned(),
            created: Vec::new(),
        });
        let outcome = self.load_file(path);
        let frame = self.state.frames.borrow_mut().pop();
        let run = match (&outcome, frame) {
            (Ok(()), _) => Run::Done,
            (Err(_), frame) => {
                frame.iter().for_each(|frame| self.take_away(frame));
                Run::Failed
            }
        };
        self.state.files.borrow_mut().insert(path.to_owned(), run);
        outcome
    }

    fn record(&self, receiver: Value, name: String) {
        if self.state.making_namespace.get() {
            return;
        }
        let Some(scope) = self.path_of(receiver) else {
            return;
        };
        if let Some(frame) = self.state.frames.borrow_mut().last_mut() {
            frame.created.push((scope, name));
        }
    }

    // Removes what `frame` created, the last first, so a constant inside a
    // class goes before the class.
    fn take_away(&self, frame: &Frame) {
        for (scope, name) in frame.created.iter().rev() {
            if let Some(scope) = self.defined_scope(scope) {
                scope
                    .const_remove(self.mrb, self.mrb.intern(name.as_bytes()).to_sym())
                    .ok();
            }
        }
    }

    // The module the names in `scope` spell exactly, if each is still defined;
    // asking for a missing one would load it by name.
    fn defined_scope(&self, scope: &[String]) -> Option<Value> {
        scope.iter().try_fold(self.object(), |outer, name| {
            let name = self.mrb.intern(name.as_bytes()).to_sym();
            if outer.const_defined_at(self.mrb, name) {
                outer.const_get(self.mrb, name).ok()
            } else {
                None
            }
        })
    }

    // The files that led back to the one defining `name`, which is still
    // running.
    fn cycle(&self, path: &str, outer: &[String], name: &str) -> String {
        let frames = self.state.frames.borrow();
        let start = frames
            .iter()
            .position(|frame| frame.path == path)
            .unwrap_or(0);
        let chain: Vec<&str> = frames[start..]
            .iter()
            .map(|frame| frame.path.as_str())
            .chain([path])
            .collect();
        format!(
            "{} is needed while its own file is still running: {}",
            qualified(outer, name),
            chain.join(" -> ")
        )
    }

    fn name_error(&self, message: &str, name: &str) -> Error {
        let error = self.mrb.exc_get(c"NameError").and_then(|class| {
            let arguments = [
                self.mrb.str_new(message.as_bytes()).as_value(),
                self.mrb.intern(name.as_bytes()).as_value(),
            ];
            class
                .into_value(self.mrb)
                .funcall(self.mrb, c"new", &arguments)
        });
        match error {
            Ok(error) => Error::Exception(error),
            Err(error) => error,
        }
    }

    // A file that does not parse, raised where it was used by name, names the
    // file as well as the line.
    fn placed(&self, path: &str, error: Error) -> Error {
        let Error::Syntax(parse) = &error else {
            return error;
        };
        let message = format!("{path}:{}: {}", parse.line(), parse.message());
        match self.mrb.exc_get(c"SyntaxError") {
            Ok(class) => Error::new(self.mrb, class, &message),
            Err(error) => error,
        }
    }
}

fn qualified(outer: &[String], name: &str) -> String {
    outer
        .iter()
        .map(String::as_str)
        .chain([name])
        .collect::<Vec<_>>()
        .join("::")
}
