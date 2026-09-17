//! Ruby's side of loading by name. A module prepended to `Module` asks the
//! class index from `const_missing` and tells the realm from `const_added`
//! what a running file creates; every other miss and addition is left to
//! Ruby through `super`.

use beni::{Error, FromValue, IntoValue, Module, Mrb, RClass, RModule, Symbol, Value, method};

use super::index::{self, Named, Namespace};
use super::{bookkeeping, compile, executor};

pub(super) fn define(mrb: &Mrb) -> Result<(), Error> {
    let module = mrb.class_get(c"Module")?;
    module.define_private_method(mrb, c"__load_by_name__", method!(load_by_name, 1))?;
    module.define_private_method(mrb, c"__created__", method!(created, 1))?;
    compile(
        mrb,
        c"godot_mruby/constants.rb",
        include_str!("constants.rb"),
    )
}

// Module#__load_by_name__(name): the constant in a one-element array, or nil
// when the class index names no file for it.
fn load_by_name(mrb: &Mrb, receiver: Value, name: Symbol) -> Result<Value, Error> {
    let Some(name) = name.name(mrb) else {
        return Ok(Value::nil());
    };
    Ok(match resolve(mrb, receiver, &name)? {
        Some(constant) => mrb.ary_new_from_values(&[constant]).as_value(),
        None => Value::nil(),
    })
}

// Module#__created__(name): the receiver has just been given the constant.
// A directory's module the realm defines is no file's to take away.
fn created(mrb: &Mrb, receiver: Value, name: Symbol) -> Value {
    if !bookkeeping(mrb).defining_namespace.get()
        && let (Some(scope), Some(name)) = (path_of(mrb, receiver), name.name(mrb))
    {
        executor::record(mrb, scope, name);
    }
    Value::nil()
}

/// Makes sure what the file at `path` opens exists before it runs, as CRuby
/// loads a constant still to be loaded at the statement opening it: the
/// namespaces its path sits in, then each other constant its statements write
/// that the class index names. mruby's statements never ask const_missing, so
/// a file opening one first would make a module or class unrelated to it.
pub(super) fn ensure_opened(mrb: &Mrb, path: &str) -> Result<(), Error> {
    ensure_namespaces(mrb, path)?;
    let own = index::key_of(path);
    for names in bookkeeping(mrb).files.declared(path).writes {
        let key: Vec<String> = names.iter().map(|name| index::normalize(name)).collect();
        // Its own class and the namespaces around it are the file's to define,
        // and nothing inside its class exists before the class does.
        if own.starts_with(&key) || key.starts_with(&own) || constant_at(mrb, &key).is_some() {
            continue;
        }
        let Some((name, outer)) = names.split_last() else {
            continue;
        };
        let named = bookkeeping(mrb).index.borrow().named(&key);
        match named {
            Some(Named::File(file)) => {
                constant_from(mrb, &file, outer, name)?;
            }
            Some(Named::Namespace(namespace)) => {
                namespace_module(mrb, outer, name, &namespace)?;
            }
            None => {}
        }
    }
    Ok(())
}

// Makes sure each namespace the file at `path` sits in exists, when the class
// index names the file: the namespace's own file runs, or its directory
// becomes an empty module.
fn ensure_namespaces(mrb: &Mrb, path: &str) -> Result<(), Error> {
    if !bookkeeping(mrb).index.borrow().names(path) {
        return Ok(());
    }
    let key = index::key_of(path);
    for depth in 1..key.len() {
        if constant_at(mrb, &key[..depth]).is_some() {
            continue;
        }
        let named = bookkeeping(mrb).index.borrow().named(&key[..depth]);
        match named {
            Some(Named::File(file)) => run_by_name(mrb, &file)?,
            Some(Named::Namespace(namespace)) => {
                // A namespace file that ran without defining its module
                // leaves the files inside to open it themselves.
                let Some(scope) = constant_at(mrb, &key[..depth - 1]) else {
                    return Ok(());
                };
                define_module(mrb, scope, &namespace.name)?;
            }
            None => {}
        }
    }
    Ok(())
}

/// The constant `key` spells, if it is already here, matched the way the
/// class index matches it.
pub(super) fn constant_at(mrb: &Mrb, key: &[String]) -> Option<Value> {
    key.iter().try_fold(object(mrb), |scope, segment| {
        constant_matching(mrb, scope, segment)
    })
}

fn object(mrb: &Mrb) -> Value {
    mrb.object_class().to_value(mrb)
}

// The constant `scope` holds whose name the class index matches to
// `segment`.
fn constant_matching(mrb: &Mrb, scope: Value, segment: &str) -> Option<Value> {
    let constants = scope
        .funcall(mrb, c"constants", &[])
        .and_then(|constants| constants.ensure_array(mrb))
        .ok()?;
    let name = (0..constants.len())
        .map(|index| constants.entry(index as isize))
        .find(|name| index::normalize(&name.to_string(mrb)) == segment)?;
    let name = name.to_sym(mrb).ok()?.to_sym();
    scope.const_get(mrb, name).ok()
}

// What `name` names from inside `receiver`: mruby hands const_missing the
// innermost scope alone, so the index looks outward from it.
fn resolve(mrb: &Mrb, receiver: Value, name: &str) -> Result<Option<Value>, Error> {
    let scope = path_of(mrb, receiver).unwrap_or_default();
    let named = bookkeeping(mrb).index.borrow().lookup(&scope, name);
    match named {
        Some((depth, Named::File(path))) => {
            constant_from(mrb, &path, &scope[..depth], name).map(Some)
        }
        Some((depth, Named::Namespace(namespace))) => {
            namespace_module(mrb, &scope[..depth], name, &namespace).map(Some)
        }
        None => Ok(None),
    }
}

// The names of the namespaces `receiver` sits in, outermost first, then its
// own; none for Object, and nothing for a module without a name, since no
// name leads back to it.
fn path_of(mrb: &Mrb, receiver: Value) -> Option<Vec<String>> {
    let path = RClass::from_value(receiver)
        .and_then(|class| class.path(mrb))
        .or_else(|| RModule::from_value(receiver).and_then(|module| module.path(mrb)))?;
    Some(match path.as_str() {
        "Object" => Vec::new(),
        path => path.split("::").map(str::to_owned).collect(),
    })
}

// Runs the file at `path`, which has to have defined `name` in the namespace
// `outer` spells.
fn constant_from(mrb: &Mrb, path: &str, outer: &[String], name: &str) -> Result<Value, Error> {
    if let Some(chain) = executor::cycle(mrb, path) {
        let message = format!(
            "{} is needed while its own file is still running: {}",
            qualified(outer, name),
            chain.join(" -> ")
        );
        return Err(name_error(mrb, &message, name));
    }
    run_by_name(mrb, path).map_err(|error| placed(mrb, path, error))?;
    let scope = named_scope(mrb, outer)?;
    let symbol = mrb.intern(name.as_bytes())?.to_sym();
    if scope.const_defined_at(mrb, symbol) {
        scope.const_get(mrb, symbol)
    } else {
        let message = format!("{path} ran without defining {}", qualified(outer, name));
        Err(name_error(mrb, &message, name))
    }
}

fn run_by_name(mrb: &Mrb, path: &str) -> Result<(), Error> {
    executor::run_by_name(mrb, path, || ensure_opened(mrb, path))
}

// A directory's module, which answers only to the name Zeitwerk gives it.
fn namespace_module(
    mrb: &Mrb,
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
        return Err(name_error(mrb, &message, name));
    }
    let scope = named_scope(mrb, outer)?;
    define_module(mrb, scope, &namespace.name)
}

// The module the names in `outer` spell exactly, from Object.
fn named_scope(mrb: &Mrb, outer: &[String]) -> Result<Value, Error> {
    outer.iter().try_fold(object(mrb), |scope, name| {
        scope.const_get(mrb, mrb.intern(name.as_bytes())?.to_sym())
    })
}

fn define_module(mrb: &Mrb, scope: Value, name: &str) -> Result<Value, Error> {
    let name = mrb.intern(name.as_bytes())?.to_sym();
    let module = mrb.module_new().to_value(mrb);
    let defining = &bookkeeping(mrb).defining_namespace;
    defining.set(true);
    let defined = scope.const_set(mrb, name, module);
    defining.set(false);
    defined.map(|()| module)
}

fn name_error(mrb: &Mrb, message: &str, name: &str) -> Error {
    let error = mrb.exc_get(c"NameError").and_then(|class| {
        let arguments = [
            mrb.str_new(message.as_bytes()).as_value(),
            mrb.intern(name.as_bytes())?.as_value(),
        ];
        class.into_value(mrb).funcall(mrb, c"new", &arguments)
    });
    match error {
        Ok(error) => Error::Exception(error),
        Err(error) => error,
    }
}

// A file that does not parse, raised where it was used by name, names the
// file as well as the line.
fn placed(mrb: &Mrb, path: &str, error: Error) -> Error {
    let Error::Syntax(parse) = &error else {
        return error;
    };
    let message = format!("{path}:{}: {}", parse.line(), parse.message());
    match mrb.exc_get(c"SyntaxError") {
        Ok(class) => Error::new(mrb, class, &message),
        Err(error) => error,
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
