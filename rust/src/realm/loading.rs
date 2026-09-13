use std::cell::Cell;
use std::ptr;

use beni::{Error, FromValue, Gem, IntoValue, Module, Mrb, RClass, RModule, Symbol, Value, method};

use super::index::{self, Key, Named, Namespace};
use super::{Realm, load};

thread_local! {
    // The realm this thread is inside, for Ruby's calls back into it.
    static CURRENT: Cell<*const Realm> = const { Cell::new(ptr::null()) };
}

/// Marks this thread as inside a realm for as long as it lives.
pub(super) struct Inside(*const Realm);

impl Inside {
    pub(super) fn new(realm: &Realm) -> Self {
        Self(CURRENT.replace(realm))
    }
}

impl Drop for Inside {
    fn drop(&mut self) {
        CURRENT.set(self.0);
    }
}

fn current<'a>() -> Option<&'a Realm> {
    // SAFETY: CURRENT holds a realm only while an `Inside` for it lives, which
    // `enter` keeps for as long as it lends the realm out; Ruby calls back
    // into the realm only within that span, on this thread.
    unsafe { CURRENT.get().as_ref() }
}

/// Loading by name: `Module#const_missing` asks the class index first, from a
/// module prepended to `Module`, and leaves every other miss to Ruby through
/// `super`. Every realm installs it.
pub struct ByName;

impl Gem for ByName {
    fn init(mrb: &Mrb) -> Result<(), Error> {
        mrb.class_get(c"Module")?.define_private_method(
            mrb,
            c"__load_by_name__",
            method!(load_by_name, 1),
        )?;
        load(
            mrb,
            "godot_mruby/const_missing.rb",
            include_str!("const_missing.rb"),
        )
    }
}

// Module#__load_by_name__(name): the constant in a one-element array, or nil
// when the class index names no file for it.
fn load_by_name(mrb: &Mrb, receiver: Value, name: Symbol) -> Result<Value, Error> {
    let (Some(realm), Some(name)) = (current(), name.name(mrb)) else {
        return Ok(Value::nil());
    };
    Ok(match realm.resolve(receiver, &name)? {
        Some(constant) => mrb.ary_new_from_values(&[constant]).as_value(),
        None => Value::nil(),
    })
}

impl Realm {
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
            let named = self.index.borrow().named(&key);
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
        let path = RClass::from_value(receiver)
            .and_then(|class| class.path(&self.mrb))
            .or_else(|| RModule::from_value(receiver).and_then(|module| module.path(&self.mrb)));
        match path.as_deref() {
            None | Some("Object") => Vec::new(),
            Some(path) => path.split("::").map(str::to_owned).collect(),
        }
    }

    // Runs the file at `path`, which has to have defined `name` in the
    // namespace `outer` spells.
    fn constant_from(&self, path: &str, outer: &[String], name: &str) -> Result<Value, Error> {
        self.run(path).map_err(|error| self.placed(path, error))?;
        let scope = self.named_scope(outer)?;
        let symbol = self.mrb.intern(name.as_bytes()).to_sym();
        if scope.const_defined_at(&self.mrb, symbol) {
            scope.const_get(&self.mrb, symbol)
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
    pub(super) fn ensure_namespaces(&self, path: &str) -> Result<(), Error> {
        let key = index::key_of(path);
        for depth in 1..key.len() {
            if self.constant_at(&key[..depth]).is_some() {
                continue;
            }
            let named = self.index.borrow().named(&key[..depth]);
            match named {
                Some(Named::File(file)) => self.run(&file)?,
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
            scope.const_get(&self.mrb, self.mrb.intern(name.as_bytes()).to_sym())
        })
    }

    fn define_module(&self, scope: Value, name: &str) -> Result<Value, Error> {
        let module = self.mrb.module_new().to_value(&self.mrb);
        scope.const_set(&self.mrb, self.mrb.intern(name.as_bytes()).to_sym(), module)?;
        Ok(module)
    }

    fn name_error(&self, message: &str, name: &str) -> Error {
        let error = self.mrb.exc_get(c"NameError").and_then(|class| {
            let arguments = [
                self.mrb.str_new(message.as_bytes()).as_value(),
                self.mrb.intern(name.as_bytes()).as_value(),
            ];
            class
                .into_value(&self.mrb)
                .funcall(&self.mrb, c"new", &arguments)
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
            Ok(class) => Error::new(&self.mrb, class, &message),
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
