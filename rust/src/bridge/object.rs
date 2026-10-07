//! The engine's objects as Ruby holds them: an object of a class under
//! `Godot` carries the engine object it stands for, and Ruby reaches its
//! methods by their names.

use std::fmt;

use beni::typed_data::{Dup, Obj};
use beni::{
    DataType, Error, ExceptionClass, FromValue, Id, IntoValue, Module, Mrb, Object as _, RArray,
    RClass, RModule, ReprValue, Symbol, TryConvert, TypedData, Value, method, value::qnil,
};
use godot::builtin::StringName;
use godot::builtin::{Variant, VariantType};
use godot::classes::{ClassDb, Engine, Object, ResourceLoader, Script};
use godot::global::type_string;
use godot::meta::ToGodot;
use godot::meta::error::CallError;
use godot::obj::{EngineEnum, Gd, InstanceId, Singleton};
use godot::register::info::PropertyHint;
use godot::sys;
use smallvec::SmallVec;

use super::BridgeData;
use super::bound_method::{Arity, BoundMethod};
use super::value::{self, ToRuby};
use crate::game;
use crate::hint::{Hint, type_name};
use crate::realm::{self, Key};
use crate::snapshot::{Heading, Property, Signal};

/// The engine object a Ruby object of an engine class stands for. Holding
/// it keeps a reference-counted object alive until Ruby lets go of it.
#[derive(Clone)]
pub struct EngineObject(Gd<Object>);

// SAFETY: a realm is entered by one thread at a time, so no two threads
// reach the object together, and the engine's objects are shared across
// threads under gdext's experimental-threads.
unsafe impl Send for EngineObject {}

static ENGINE_OBJECT: DataType<EngineObject> = DataType::new(c"Godot::Object");

// SAFETY: `define` marks Godot::Object, and every engine class under
// Godot descends from it.
unsafe impl TypedData for EngineObject {
    fn class(mrb: &Mrb) -> RClass {
        object_class(mrb, super::data(mrb))
    }

    fn data_type() -> &'static DataType<Self> {
        &ENGINE_OBJECT
    }
}

/// Godot::Object, found once for a realm and kept in its bridge data.
pub(super) fn object_class(mrb: &Mrb, data: &BridgeData) -> RClass {
    super::find_class_once(mrb, &data.object_class, || {
        root(mrb).expect("the Godot gem defines Godot::Object")
    })
}

/// Defines Godot::Object, the class every engine class descends from.
pub fn define(mrb: &Mrb, godot: RModule) -> Result<(), Error> {
    let object = godot.define_class(mrb, c"Object", mrb.object_class())?;
    object.set_instance_data_tt(mrb)?;
    object.undef_default_alloc_func(mrb);
    object.define_singleton_method(mrb, c"__make__", method!(make, 0))?;
    object.define_singleton_method(mrb, c"__make_node__", method!(make_node, 0))?;
    object.define_singleton_method(mrb, c"__allocate__", method!(allocate, 1))?;
    object.define_singleton_method(mrb, c"__singleton__", method!(singleton, 0))?;
    object.define_singleton_method(mrb, c"__declares__", method!(declares, 1))?;
    object.define_singleton_method(mrb, c"__has_static_method__", method!(has_static_method, 1))?;
    object.define_singleton_method(mrb, c"__call_static__", method!(call_static, 2))?;
    object.define_singleton_method(mrb, c"__engine_constant__", method!(engine_constant, 1))?;
    object.define_singleton_method(mrb, c"__declare_signal__", method!(declare_signal, 2))?;
    object.define_singleton_method(mrb, c"__declare_export__", method!(declare_export, 4))?;
    object.define_singleton_method(mrb, c"__declare_heading__", method!(declare_heading, 3))?;
    object.define_private_method(mrb, c"__resolve__", method!(resolve, 1))?;
    object.define_private_method(mrb, c"__call__", method!(call, 2))?;
    object.define_private_method(mrb, c"__call_bound__", method!(call_bound, -1))?;
    object.define_private_method(mrb, c"__apply_bound__", method!(apply_bound, 2))?;
    object.define_singleton_method(mrb, c"__shape__", method!(shape, 1))?;
    object.define_private_method(mrb, c"__instance_id__", method!(instance_id, 0))?;
    object.define_private_method(mrb, c"__label__", method!(label, 0))?;
    object.define_private_method(mrb, c"__clone__", method!(clone, -1))?;
    Ok(())
}

fn root(mrb: &Mrb) -> Result<RClass, Error> {
    mrb.module_get(c"Godot")?.class_get(mrb, c"Object")
}

// Godot::Object.__make__: a new engine object of the receiver's engine
// class.
fn make(mrb: &Mrb, class: RClass) -> Result<Value, Error> {
    let path = class.path(mrb).unwrap_or_default();
    let name = path.strip_prefix("Godot::").unwrap_or(&path);
    let class_db = ClassDb::singleton();
    if !class_db.can_instantiate(name) {
        let message = format!("the engine makes no objects of {path}");
        return Err(not_implemented_error(mrb, &message));
    }
    let object = class_db.instantiate(name).to::<Gd<Object>>();
    Ok(mrb.wrap_as(EngineObject(object), class).as_value())
}

// Godot::Object.__make_node__: a new node of the engine class the
// receiver's file extends, carrying that file's script, and the receiver's
// object for it, uninitialized and held under the node's key.
fn make_node(mrb: &Mrb, class: RClass) -> Result<Value, Error> {
    let path = class.path(mrb);
    let script = path
        .as_ref()
        .and_then(|path| {
            let names: Vec<String> = path.split("::").map(str::to_owned).collect();
            realm::file_by_constant(mrb, &names)
        })
        .and_then(|file| ResourceLoader::singleton().load(&file))
        .and_then(|resource| resource.try_cast::<Script>().ok())
        .filter(|script| !script.get_instance_base_type().is_empty());
    let class_name = || {
        path.clone()
            .unwrap_or_else(|| "an unnamed class".to_owned())
    };
    let Some(script) = script else {
        let message = format!(
            "no node script defines {}, so it makes no node",
            class_name()
        );
        return Err(not_implemented_error(mrb, &message));
    };
    let mut node = ClassDb::singleton()
        .instantiate(&script.get_instance_base_type())
        .to::<Gd<Object>>();
    node.set_script(&script);
    if node.get_script().is_none() {
        node.free();
        let message = format!("{}'s script refused its node here", class_name());
        return Err(not_implemented_error(mrb, &message));
    }
    let object = mrb.wrap_as(EngineObject(node.clone()), class).as_value();
    realm::hold(mrb, node_key(node.instance_id()), object)?;
    Ok(object)
}

// Godot::Object.__allocate__(owner): the receiver's object for the engine
// object `owner` stands for, uninitialized, or the TypeError raised when
// that object is not of the engine class the receiver extends, whose
// methods the receiver calls on it.
fn allocate(mrb: &Mrb, class: RClass, owner: &EngineObject) -> Result<Value, Error> {
    check_stands_for(mrb, class, owner)?;
    Ok(mrb.wrap_as(owner.clone(), class).as_value())
}

// Godot::Object#__clone__: a copy made as Ruby's clone makes one, standing
// for the receiver's engine object, or the TypeError a freed one raises.
fn clone(mrb: &Mrb, held: Obj<EngineObject>, args: &[Value]) -> Result<Obj<EngineObject>, Error> {
    check_stands_for(mrb, held.as_value().class(mrb), &held)?;
    <EngineObject as Dup>::clone(mrb, held, args)
}

// The TypeError raised when an object of `class` would stand for `owner`'s
// engine object although that object is freed or not of the engine class
// `class` extends, whose methods `class` calls on it.
fn check_stands_for(mrb: &Mrb, class: RClass, owner: &EngineObject) -> Result<(), Error> {
    let extended = engine_ancestor(mrb, class).unwrap_or_default();
    if owner.0.is_instance_valid() && owner.0.is_class(extended.as_str()) {
        return Ok(());
    }
    let message = format!(
        "{} stands only for a live {extended}",
        class.path(mrb).unwrap_or_default()
    );
    Err(type_error(mrb, &message))
}

/// The key a realm holds a node's Ruby object under: its instance id, which
/// is positive, since only a reference-counted object's id is negative.
pub fn node_key(node: InstanceId) -> Key {
    Key::from(node.to_i64())
}

/// A node's engine object as Ruby is given it, for its class to make the
/// node's Ruby object from.
pub struct Owner(pub InstanceId);

impl IntoValue for Owner {
    fn into_value(self, mrb: &Mrb) -> Value {
        match Gd::<Object>::try_from_instance_id(self.0) {
            Ok(owner) => mrb.wrap(EngineObject(owner)).as_value(),
            Err(_) => qnil().as_value(),
        }
    }
}

// Godot::Object#__resolve__(name): the engine method a call of `name`
// reaches, whether the engine class the receiver's Ruby class extends
// declares it, so every object of the Ruby class answers it, an object of
// a class extending that one or of a class the engine keeps hidden
// included, and the method bound for that class when the engine registered
// it, or nil when it reaches none; a receiver carrying no engine object
// raises TypeError. A name ending in `=` reaches the property's setter, and
// a name no method has reaches its getter; a property read or written by an
// index reaches the engine's `get` or `set`, with the property's name.
fn resolve(mrb: &Mrb, receiver: Value, name: Symbol) -> Result<Value, Error> {
    let held = <&EngineObject>::try_convert(receiver, mrb)?;
    let name = name.name(mrb).unwrap_or_default();
    let object = held.live_object(mrb, &name)?;
    let class = StringName::from(&object.get_class());
    let mut class_db = ClassDb::singleton();
    let (target, property) = if let Some(property) = name.strip_suffix('=') {
        let setter = class_db.class_get_property_setter(&class, property);
        (setter.to_string(), Some((property, "set")))
    } else if object.has_method(name.as_str()) {
        (name.clone(), None)
    } else {
        let getter = class_db.class_get_property_getter(&class, name.as_str());
        (getter.to_string(), Some((name.as_str(), "get")))
    };
    if target.is_empty() {
        return Ok(qnil().as_value());
    }
    // A setter takes the value beside any index, a getter nothing more.
    let takes_index = |accessor: &str| {
        let values = usize::from(accessor == "set");
        Arity::find(&class.to_string(), &target).is_some_and(|arity| arity.required > values)
    };
    if let Some((property, accessor)) = property.filter(|(_, accessor)| takes_index(accessor)) {
        return indexed_property(mrb, accessor, property);
    }
    let extended = receiver
        .funcall(mrb, c"class", &[])
        .ok()
        .and_then(RClass::from_value)
        .and_then(|ruby_class| engine_ancestor(mrb, ruby_class))
        .filter(|extended| class_db.class_has_method(extended.as_str(), target.as_str()));
    let declared = extended.is_some();
    let bound = extended
        .and_then(|extended| BoundMethod::find(&extended, &target))
        .map_or_else(|| qnil().as_value(), |bound| mrb.wrap(bound).as_value());
    let target = Symbol::from(mrb.intern(target.as_bytes())?).as_value();
    Ok(mrb
        .ary_new_from_values(&[target, declared.into_value(mrb), bound])
        .as_value())
}

// Godot::Object.__declares__(name): whether the engine class the receiver
// extends has a method or property a call of `name` reaches, for an object
// that has no live engine object to ask.
fn declares(mrb: &Mrb, class: RClass, name: Symbol) -> bool {
    let Some(extended) = engine_ancestor(mrb, class) else {
        return false;
    };
    let name = name.name(mrb).unwrap_or_default();
    let extended = StringName::from(extended.as_str());
    let mut class_db = ClassDb::singleton();
    if let Some(property) = name.strip_suffix('=') {
        return !class_db
            .class_get_property_setter(&extended, property)
            .is_empty();
    }
    class_db.class_has_method(&extended, name.as_str())
        || !class_db
            .class_get_property_getter(&extended, name.as_str())
            .is_empty()
}

// What `__resolve__` answers for a property whose getter and setter take an
// index the property's name stands for: the engine's `get` or `set`, called
// by name with the property's name ahead of the arguments.
fn indexed_property(mrb: &Mrb, accessor: &str, property: &str) -> Result<Value, Error> {
    let target = Symbol::from(mrb.intern(accessor.as_bytes())?).as_value();
    let property = mrb.str_new(property.as_bytes()).as_value();
    Ok(mrb
        .ary_new_from_values(&[target, true.into_value(mrb), qnil().as_value(), property])
        .as_value())
}

// Godot::Object#__call_bound__(bound, *args): calls the engine method
// `bound` with the arguments given and answers what it returns.
fn call_bound(mrb: &Mrb, held: &EngineObject, args: &[Value]) -> Result<Value, Error> {
    let Some((&bound, args)) = args.split_first() else {
        return Err(argument_error(mrb, &count_message(0, 1)));
    };
    let bound = <&BoundMethod>::try_convert(bound, mrb)?;
    call_bind(mrb, super::data(mrb), held, bound, args.iter().copied())
}

// Godot::Object#__apply_bound__(bound, args): calls the engine method
// `bound` with the arguments in `args`, for a method taking any number.
fn apply_bound(
    mrb: &Mrb,
    held: &EngineObject,
    bound: &BoundMethod,
    args: RArray,
) -> Result<Value, Error> {
    call_bind(mrb, super::data(mrb), held, bound, args.entries(mrb))
}

// Calls the engine method `bound` with `args` on the receiver's engine
// object, asking whether it lives only once the arguments are converted,
// since converting one may run Ruby that frees it.
fn call_bind(
    mrb: &Mrb,
    data: &BridgeData,
    held: &EngineObject,
    bound: &BoundMethod,
    args: impl IntoIterator<Item = Value>,
) -> Result<Value, Error> {
    let args = to_arguments(mrb, data, args)?;
    let object = held.live_object(mrb, bound.name())?;
    let answer = bound.call(object, &args).map_err(|error| {
        let base = object.get_class().to_string();
        refusal_error(mrb, &error, bound.name(), &base, &args, Some(bound.arity()))
    })?;
    ruby_answer(mrb, data, &answer)
}

// Godot::Object.__shape__(bound): how many arguments the engine method
// `bound` requires and how many more it takes, or nil when it takes any
// number.
fn shape(mrb: &Mrb, _class: RClass, bound: &BoundMethod) -> Value {
    let arity = bound.arity();
    if arity.is_vararg {
        return qnil().as_value();
    }
    let counts = [arity.required, arity.optional].map(|count| (count as i64).into_value(mrb));
    mrb.ary_new_from_values(&counts).as_value()
}

// Godot::Object#__call__(name, args): calls the engine method `name` with
// `args` and answers what it returns, asking whether the receiver lives once
// the arguments are converted, as `call_bind` does.
fn call(mrb: &Mrb, held: &EngineObject, name: Symbol, args: RArray) -> Result<Value, Error> {
    let data = super::data(mrb);
    let name = name_by_symbol(mrb, name);
    let args = to_arguments(mrb, data, args.entries(mrb))?;
    let mut object = held.live_object(mrb, &name)?.clone();
    let answer = object.try_call(&name, &args).map_err(|error| {
        let base = object.get_class().to_string();
        call_refusal(mrb, &error, &base, &name.to_string(), args.len())
    })?;
    ruby_answer(mrb, data, &answer)
}

/// The engine's name `symbol` spells, made once for a realm, since making
/// one looks the name up in the engine's table of names.
pub(super) fn name_by_symbol(mrb: &Mrb, symbol: Symbol) -> StringName {
    let names = &super::data(mrb).engine_names;
    let id = Id::from(symbol);
    if let Some(name) = names.borrow().get(&id) {
        return name.clone();
    }
    let name = StringName::from(symbol.name(mrb).unwrap_or_default().as_str());
    names.borrow_mut().insert(id, name.clone());
    name
}

// Godot::Object#__label__: the engine object as Godot prints it,
// `<Class#id>`, or `<Freed Object>` once it is freed.
fn label(mrb: &Mrb, held: &EngineObject) -> Value {
    let label = if held.0.is_instance_valid() {
        let id = held.0.instance_id_unchecked().to_i64();
        format!("<{}#{id}>", held.0.get_class())
    } else {
        "<Freed Object>".to_owned()
    };
    mrb.str_new(label.as_bytes()).as_value()
}

// Godot::Object#__instance_id__: the engine object's instance id, which
// two Ruby objects for one engine object share.
fn instance_id(_mrb: &Mrb, held: &EngineObject) -> i64 {
    held.0.instance_id_unchecked().to_i64()
}

// Godot::Object.__singleton__: the engine's singleton of the receiver's
// engine class, or nil when the engine keeps none. The engine is asked
// first, since it reports a singleton it lacks as an error.
fn singleton(mrb: &Mrb, class: RClass) -> Value {
    let name = engine_name(mrb, class);
    let engine = Engine::singleton();
    if !engine.has_singleton(&name) {
        return qnil().as_value();
    }
    engine
        .get_singleton(&name)
        .map(|object| mrb.wrap_as(EngineObject(object), class).as_value())
        .unwrap_or_else(|| qnil().as_value())
}

// Godot::Object.__has_static_method__(name): whether the receiver's engine class
// has a method of that name to call on the class.
fn has_static_method(mrb: &Mrb, class: RClass, name: Symbol) -> bool {
    let name = name.name(mrb).unwrap_or_default();
    ClassDb::singleton().class_has_method(&engine_name(mrb, class), name.as_str())
}

// Godot::Object.__call_static__(name, args): calls the static method `name`
// of the receiver's engine class with `args`.
fn call_static(mrb: &Mrb, class: RClass, name: Symbol, args: RArray) -> Result<Value, Error> {
    let name = name_by_symbol(mrb, name);
    let class = engine_name(mrb, class);
    let data = super::data(mrb);
    let args = to_arguments(mrb, data, args.entries(mrb))?;
    let answer = ClassDb::singleton()
        .try_class_call_static(&class, &name, &args)
        .map_err(|error| call_refusal(mrb, &error, &class, &name.to_string(), args.len()))?;
    ruby_answer(mrb, data, &answer)
}

// What the engine answered, as Ruby is given it, or the Godot::CallError an
// answer that cannot reach Ruby raises.
fn ruby_answer(mrb: &Mrb, data: &BridgeData, answer: &Variant) -> Result<Value, Error> {
    ToRuby::try_new(answer)
        .map(|answer| answer.into_ruby(mrb, data))
        .map_err(|reason| call_error(mrb, &reason))
}

/// The Ruby object for an engine object: the one the realm holds for the
/// node, or an object of its engine class under Godot. Only a node is held
/// under its instance id, and a reference-counted object, never a node, is
/// not looked for: its id is negative, as the keys the realm names itself.
pub(super) fn ruby_object(mrb: &Mrb, data: &BridgeData, object: Gd<Object>) -> Value {
    let id = object.instance_id();
    if !id.is_ref_counted()
        && let Some(held) = realm::object(mrb, node_key(id))
    {
        return held;
    }
    let class = class_by_name(mrb, data, exposed_class(&object)).or_else(|| root(mrb).ok());
    match class {
        Some(class) => mrb.wrap_as(EngineObject(object), class).as_value(),
        None => qnil().as_value(),
    }
}

// The class under Godot of the engine class `name`, found once for a realm
// and kept for it, rooted for the collector so the class lives while it is
// kept.
fn class_by_name(mrb: &Mrb, data: &BridgeData, name: StringName) -> Option<RClass> {
    let classes = &data.engine_classes;
    if let Some(class) = classes.borrow().get(&name).copied() {
        return Some(class);
    }
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| godot.const_get::<_, RClass>(mrb, name.to_string().as_str()))
        .ok()?;
    mrb.gc_register_forever(class.as_value());
    classes.borrow_mut().insert(name, class);
    Some(class)
}

// The name of the nearest class of `object` the engine exposes to
// extensions, which is the object's own class unless the engine keeps that
// one hidden.
fn exposed_class(object: &Gd<Object>) -> StringName {
    let mut name = StringName::default();
    // SAFETY: the interface is initialized while the extension runs and
    // `object` is alive; the name it writes over is empty, so it holds
    // nothing to release.
    unsafe {
        sys::interface_fn!(object_get_class_name)(
            object.obj_sys().cast_const(),
            sys::get_library(),
            name.string_sys_mut().cast(),
        );
    }
    name
}

// Godot::Object.__declare_signal__(name, parameters): takes the signal the
// class declares as its body runs, for the realm to publish once the file
// has run.
fn declare_signal(
    mrb: &Mrb,
    class: RClass,
    name: String,
    parameters: RArray,
) -> Result<Value, Error> {
    let parameters = parameters
        .entries(mrb)
        .filter_map(String::from_value)
        .collect();
    realm::declare_signal(mrb, class, Signal { name, parameters })?;
    Ok(qnil().as_value())
}

// Godot::Object.__declare_export__(name, default, hint, written): takes the
// property the class exports as its body runs, its type read from the value
// it is declared with and its hint from the keyword naming it, read with
// the value written with that keyword, for the realm to publish once the
// file has run. A declaration GDScript would refuse is refused in
// GDScript's own words, so the two languages read the same when the same
// mistake is made.
fn declare_export(
    mrb: &Mrb,
    class: RClass,
    name: String,
    default: Value,
    hint: String,
    written: Value,
) -> Result<Value, Error> {
    let keyword = match hint.as_str() {
        "type" => None,
        keyword => {
            Some(Hint::from_keyword(keyword).map_err(|reason| argument_error(mrb, &reason))?)
        }
    };
    let data = super::data(mrb);
    let default =
        value::to_engine(mrb, data, default, 1).map_err(|reason| argument_error(mrb, &reason))?;
    let written =
        value::to_engine(mrb, data, written, 1).map_err(|reason| argument_error(mrb, &reason))?;
    if let Some(engine_class) = engine_member(mrb, class, &name) {
        let message =
            format!("Member \"{name}\" redefined (original in native class '{engine_class}')");
        return Err(argument_error(mrb, &message));
    }
    let property = match keyword {
        None => property_of_class(mrb, name, &default, &written.to_string()),
        Some(hint) => property_of_value(name, &default, hint, &written),
    }
    .map_err(|reason| argument_error(mrb, &reason))?;
    realm::declare_export(mrb, class, property)?;
    Ok(qnil().as_value())
}

// Godot::Object.__declare_heading__(name, prefix, kind): takes the heading
// the class writes as its body runs, which the properties written after it
// are shown under, for the realm to publish once the file has run.
fn declare_heading(mrb: &Mrb, _class: RClass, name: String, prefix: String, kind: String) -> Value {
    let heading = match kind.as_str() {
        // A category the body writes is headed by no file.
        "category" => Heading::Category {
            name,
            path: String::new(),
        },
        "subgroup" => Heading::Subgroup { name, prefix },
        _ => Heading::Group { name, prefix },
    };
    realm::declare_heading(mrb, heading);
    qnil().as_value()
}

// The property an export naming its type declares: an object of the class
// it names, which Godot fills in from the scene or the project's files. The
// value it is declared with is the class's own to hold, so a value of
// another type is refused as GDScript refuses a mismatched one.
fn property_of_class(
    mrb: &Mrb,
    name: String,
    default: &Variant,
    class: &str,
) -> Result<Property, String> {
    let (hint, class_name) = hint_by_class(mrb, class)?;
    if let Some(given) = mismatch(mrb, default, class) {
        return Err(format!(
            "Cannot assign a value of type {given} to variable \"{name}\" with specified type {class_name}."
        ));
    }
    Ok(Property::new(name, default).with_class(hint, class_name))
}

// What the value an export was declared with is, unless the class it names
// takes it: nothing at all, which is what Godot fills in, or an object of
// that class, a class descending from it included.
fn mismatch(mrb: &Mrb, default: &Variant, class: &str) -> Option<String> {
    let kind = default.get_type();
    if kind == VariantType::NIL {
        return None;
    }
    let Ok(object) = default.try_to::<Gd<Object>>() else {
        return Some(type_name(kind));
    };
    (!is_of_class(mrb, &object, class)).then(|| resolve_class_name(&object))
}

// Whether `object` is of the class `class` names: the engine's class
// database answers for an engine class, and a node is of a Ruby class when
// its script is that class's file or inherits from it.
fn is_of_class(mrb: &Mrb, object: &Gd<Object>, class: &str) -> bool {
    if let Some(engine_class) = class.strip_prefix("Godot::") {
        let object_class = StringName::from(&object.get_class());
        return ClassDb::singleton().is_parent_class(&object_class, engine_class);
    }
    let names: Vec<String> = class.split("::").map(str::to_owned).collect();
    let Some(file) = realm::file_by_constant(mrb, &names) else {
        return false;
    };
    script_files(object).any(|path| path == file)
}

// The name `object`'s own class is known by: the one its node script is
// announced under, or the engine class it is of.
fn resolve_class_name(object: &Gd<Object>) -> String {
    script_files(object)
        .next()
        .and_then(|path| editor_name_by_path(&path))
        .unwrap_or_else(|| object.get_class().to_string())
}

// The files `object`'s script is written in, nearest first: its own and the
// ones it inherits from, as Godot answers for them.
fn script_files(object: &Gd<Object>) -> impl Iterator<Item = String> {
    let mut script = object.get_script().map(Gd::upcast::<Script>);
    std::iter::from_fn(move || {
        let written = script.take()?;
        script = written.get_base_script();
        Some(written.get_path().to_string())
    })
}

// The property an export naming no type declares: its type is the declared
// value's, so a value naming none is refused, and the keyword naming a hint
// tells the editor how to show it.
fn property_of_value(
    name: String,
    default: &Variant,
    hint: Hint,
    written: &Variant,
) -> Result<Property, String> {
    if default.get_type() == VariantType::NIL {
        return Err(
            "Cannot use \"export\" because the type of the initialized value can't be inferred."
                .to_owned(),
        );
    }
    let property_hint = hint.property_hint(default.get_type())?;
    // The gem refuses a value of another shape where it is written.
    let hint_string = hint.hint_string(written).unwrap_or_default();
    Ok(Property::new(name, default).with_hint(property_hint, hint_string))
}

// The hint an exported object takes from the class it names, and the name
// the editor reads that class by: a resource is chosen among the project's
// files and a node among the scene's, while a class of neither kind is no
// type an export can take. A Ruby class is named by the announcement the
// editor lists it under, so one no announcement names is out of reach.
fn hint_by_class(mrb: &Mrb, class: &str) -> Result<(PropertyHint, String), String> {
    if let Some(engine_class) = class.strip_prefix("Godot::") {
        let class_db = ClassDb::singleton();
        if !class_db.class_exists(engine_class) {
            return Err(unknown_type(class));
        }
        if class_db.is_parent_class(engine_class, "Resource") {
            return Ok((PropertyHint::RESOURCE_TYPE, engine_class.to_owned()));
        }
        if class_db.is_parent_class(engine_class, "Node") {
            return Ok((PropertyHint::NODE_TYPE, engine_class.to_owned()));
        }
        return Err("Export type can only be built-in, a resource, a node, or an enum.".to_owned());
    }
    editor_name(mrb, class)
        .map(|name| (PropertyHint::NODE_TYPE, name))
        .ok_or_else(|| unknown_type(class))
}

// The name the editor lists the Ruby class `class` under, if a node script
// of the project defines it and nothing else is announced by that name.
fn editor_name(mrb: &Mrb, class: &str) -> Option<String> {
    let names: Vec<String> = class.split("::").map(str::to_owned).collect();
    editor_name_by_path(&realm::file_by_constant(mrb, &names)?)
}

// The name the editor lists the file at `path` under, if it is announced.
fn editor_name_by_path(path: &str) -> Option<String> {
    game::announcement_by_path(path)
        .ok()
        .map(|announcement| announcement.name)
}

// GDScript's words for a type nothing in the project is known by.
fn unknown_type(class: &str) -> String {
    format!("The class \"{class}\" was not found in the global scope.")
}

// The engine class `class` extends that has a member of that name, if one
// has: a signal, a property or an integer constant, which is what GDScript
// refuses a member for redefining.
fn engine_member(mrb: &Mrb, class: RClass, name: &str) -> Option<String> {
    let engine_class = engine_ancestor(mrb, class)?;
    let mut class_db = ClassDb::singleton();
    let has = class_db.class_has_signal(engine_class.as_str(), name)
        || class_db.class_has_integer_constant(engine_class.as_str(), name)
        || !class_db
            .class_get_property_setter(engine_class.as_str(), name)
            .is_empty()
        || !class_db
            .class_get_property_getter(engine_class.as_str(), name)
            .is_empty();
    has.then_some(engine_class)
}

// The engine class `class` extends, itself or through its superclasses, as
// the engine names it.
fn engine_ancestor(mrb: &Mrb, class: RClass) -> Option<String> {
    let mut current = class.as_value();
    loop {
        let path = RClass::from_value(current)?.path(mrb)?;
        if let Some(name) = path.strip_prefix("Godot::") {
            return Some(name.to_owned());
        }
        current = current.funcall(mrb, c"superclass", &[]).ok()?;
    }
}

// Godot::Object.__engine_constant__(name): the integer constant or enum
// value of that name the receiver's engine class has, or nil.
fn engine_constant(mrb: &Mrb, class: RClass, name: Symbol) -> Value {
    let name = name.name(mrb).unwrap_or_default();
    let class = engine_name(mrb, class);
    let class_db = ClassDb::singleton();
    if !class_db.class_has_integer_constant(&class, name.as_str()) {
        return qnil().as_value();
    }
    class_db
        .class_get_integer_constant(&class, name.as_str())
        .into_value(mrb)
}

// The engine class an engine class under Godot stands for.
fn engine_name(mrb: &Mrb, class: RClass) -> String {
    let path = class.path(mrb).unwrap_or_default();
    path.strip_prefix("Godot::").unwrap_or(&path).to_owned()
}

/// The arguments of one engine call, which a game passes few of, so they stay
/// off the heap.
pub(super) type Arguments = SmallVec<[Variant; 4]>;

/// `args` as the engine takes them, or the ArgumentError one that cannot
/// reach it raises, before the engine is given any.
pub(super) fn to_arguments(
    mrb: &Mrb,
    data: &BridgeData,
    args: impl IntoIterator<Item = Value>,
) -> Result<Arguments, Error> {
    args.into_iter()
        .map(|arg| {
            value::to_engine(mrb, data, arg, 1).map_err(|reason| argument_error(mrb, &reason))
        })
        .collect()
}

impl EngineObject {
    /// The engine object as the engine takes it, unless it was freed.
    pub fn variant(&self) -> Result<Variant, String> {
        if self.0.is_instance_valid() {
            Ok(self.0.to_variant())
        } else {
            Err("a freed engine object cannot reach the engine".to_owned())
        }
    }

    // The engine object, lent rather than copied since a copy checks it
    // again, or the Godot::CallError calling `method` on a freed one raises,
    // worded as GDScript words it.
    fn live_object(
        &self,
        mrb: &Mrb,
        method: &(impl fmt::Display + ?Sized),
    ) -> Result<&Gd<Object>, Error> {
        if self.0.is_instance_valid() {
            Ok(&self.0)
        } else {
            let message = format!(
                "Attempt to call function '{method}' in base 'previously freed' on a null instance."
            );
            Err(call_error(mrb, &message))
        }
    }
}

// The error a call the engine refused raises, as `refusal_error` sorts it.
// gdext hands the engine's reason only as text, so the expected count, the
// argument and the types are read back from it; the count names those the
// method requires, as the class database lists them.
fn call_refusal(mrb: &Mrb, error: &CallError, base: &str, method: &str, given: usize) -> Error {
    let reason = error.message(false);
    let reason = reason.rsplit("Reason: ").next().unwrap_or_default();
    if let Some(expected) = parse_parameter_count(reason) {
        let message = Arity::find(base, method).map_or_else(
            || count_message(given, expected),
            |arity| count_message(given, arity.required),
        );
        argument_error(mrb, &message)
    } else if let Some((argument, from, to)) = conversion(reason) {
        type_error(mrb, &type_message(method, base, argument, &from, &to))
    } else {
        call_error(
            mrb,
            &format!("Invalid call to function '{method}' in base '{base}': {reason}"),
        )
    }
}

/// The error a call of `method` on `base` with `args` the engine refused
/// with `error` raises: ArgumentError for the number of arguments, worded as
/// mruby words it, naming those the method's `arity` requires when it is
/// known; TypeError for an argument's type, worded as GDScript's
/// untyped call; Godot::CallError for what Ruby has no error for.
pub(super) fn refusal_error(
    mrb: &Mrb,
    error: &sys::GDExtensionCallError,
    method: &str,
    base: &str,
    args: &[Variant],
    arity: Option<Arity>,
) -> Error {
    match error.error {
        sys::GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT => {
            let index = error.argument as usize;
            let from = args
                .get(index)
                .map_or_else(String::new, |arg| type_name(arg.get_type()));
            let to = type_name(<VariantType as EngineEnum>::from_ord(error.expected));
            type_error(
                mrb,
                &type_message(method, base, &(index + 1).to_string(), &from, &to),
            )
        }
        sys::GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS
        | sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS => {
            let message = match arity {
                Some(arity) if arity.is_vararg && args.len() >= arity.required => {
                    forwarded_count_message(error, base, args, arity)
                }
                Some(arity) => count_message(args.len(), arity.required),
                None => count_message(args.len(), error.expected),
            };
            argument_error(mrb, &message)
        }
        _ => call_error(
            mrb,
            &format!("Invalid call to function '{method}' in base '{base}'."),
        ),
    }
}

// Ruby's words for a count the engine refused past a method taking any
// number, which `Arity::check` let through: the method forwarded the
// arguments past those it requires to the method its first one names, as
// `call` does, so the words name that method's count, as Ruby's `send` does.
fn forwarded_count_message(
    error: &sys::GDExtensionCallError,
    base: &str,
    args: &[Variant],
    arity: Arity,
) -> String {
    let forwarded = args.len() - arity.required;
    let required = args
        .first()
        .and_then(|name| name.try_to::<StringName>().ok())
        .and_then(|name| Arity::find(base, &name.to_string()))
        .map_or(error.expected as usize, |method| method.required);
    count_message(forwarded, required)
}

/// Ruby's words for a call given `given` arguments where `expected` are taken.
pub(super) fn count_message(given: usize, expected: impl fmt::Display) -> String {
    format!("wrong number of arguments (given {given}, expected {expected})")
}

// GDScript's words for a call given an argument of a type it cannot take.
fn type_message(method: &str, base: &str, argument: &str, from: &str, to: &str) -> String {
    format!(
        "Invalid type in function '{method}' in base '{base}'. \
         Cannot convert argument {argument} from {from} to {to}."
    )
}

// "function has N parameters, but received M arguments": N.
fn parse_parameter_count(reason: &str) -> Option<&str> {
    reason
        .strip_prefix("function has ")?
        .split_once(" parameter")
        .map(|(count, _)| count)
}

// "parameter #I -- cannot convert from FROM to TO": I and the two types as
// GDScript names them.
fn conversion(reason: &str) -> Option<(&str, String, String)> {
    let (argument, types) = reason
        .strip_prefix("parameter #")?
        .split_once(" -- cannot convert from ")?;
    let (from, to) = types.split_once(" to ")?;
    Some((argument, type_name_by_debug(from)?, type_name_by_debug(to)?))
}

// The name GDScript gives the variant type gdext debug-prints as `debug`.
fn type_name_by_debug(debug: &str) -> Option<String> {
    (0..VariantType::MAX.ord)
        .map(<VariantType as EngineEnum>::from_ord)
        .find(|kind| format!("{kind:?}") == debug)
        .map(|kind| type_string(i64::from(kind.ord)).to_string())
}

pub(super) fn type_error(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"TypeError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}

pub(super) fn zero_division_error(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"ZeroDivisionError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}

pub(super) fn argument_error(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"ArgumentError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}

fn not_implemented_error(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"NotImplementedError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}

pub(super) fn call_error(mrb: &Mrb, message: &str) -> Error {
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| godot.class_get(mrb, c"CallError"));
    match class.map(|class| ExceptionClass::from_value(class.as_value())) {
        Ok(Some(class)) => Error::new(mrb, class, message),
        Ok(None) => unreachable!("Godot::CallError is an exception class"),
        Err(error) => error,
    }
}
