//! The engine's objects as Ruby holds them: an object of a class under
//! `Godot` carries the engine object it stands for, and Ruby reaches its
//! methods by their names.

use std::fmt;
use std::mem::ManuallyDrop;
use std::sync::{Mutex, PoisonError};

use beni::typed_data::RTypedData;
use beni::{
    Class as _, DataType, Error, Id, IntoValue, Module, Mrb, Object as _, RArray, RClass, RModule,
    ReprValue, Symbol, TryConvert, TypedData, Value, method, value::qnil,
};
use godot::builtin::StringName;
use godot::builtin::Variant;
use godot::classes::{ClassDb, Engine, Object, ResourceLoader, Script};
use godot::meta::ToGodot;
use godot::obj::{Gd, InstanceId, Singleton};
use godot::sys;
use smallvec::SmallVec;

use super::bound_method::{Arity, BoundMethod, is_static_method};
use super::declaration;
use super::refusal::{
    describe_count, describe_instance_call, make_argument_error, make_call_error,
    make_call_refusal, make_no_method_error, make_not_implemented_error, make_refusal_error,
    make_type_error,
};
use super::value::{self, RubyCrossing};
use super::{BridgeData, read_engine_name};
use crate::realm::{self, Key};

/// The engine object a Ruby object of an engine class stands for. Holding
/// it keeps a reference-counted object alive until Ruby lets go of it, and
/// a reference-counted one Ruby lets go of is let go of at the next frame,
/// since freeing it may call its script, which the collector cannot run.
pub struct EngineObject(ManuallyDrop<Gd<Object>>, Standing);

// Whether a Ruby object is the one its engine object's script made, which
// the realm holds under the engine object's key.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Standing {
    Plain,
    Scripted,
}

// The reference-counted engine objects Ruby let go of, waiting for the next
// frame.
static RELEASED: Mutex<Vec<EngineObject>> = Mutex::new(Vec::new());

impl EngineObject {
    fn new(object: Gd<Object>) -> Self {
        Self(ManuallyDrop::new(object), Standing::Plain)
    }

    // The object a script made, which the realm holds under its key.
    fn scripted(object: Gd<Object>) -> Self {
        Self(ManuallyDrop::new(object), Standing::Scripted)
    }
}

impl Clone for EngineObject {
    fn clone(&self) -> Self {
        Self::new((*self.0).clone())
    }
}

impl Drop for EngineObject {
    fn drop(&mut self) {
        // SAFETY: taken once, as the value goes.
        let object = unsafe { ManuallyDrop::take(&mut self.0) };
        let id = object.instance_id_unchecked();
        if !id.is_ref_counted() {
            return;
        }
        if self.1 == Standing::Scripted {
            realm::note_collected(derive_object_key(id));
        }
        RELEASED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Self::new(object));
    }
}

/// Lets go of the reference-counted engine objects Ruby let go of since the
/// last call; the extension calls it every frame and once the realm closes.
pub fn release_collected() {
    let released = std::mem::take(&mut *RELEASED.lock().unwrap_or_else(PoisonError::into_inner));
    for mut object in released {
        // SAFETY: taken once; the emptied value is forgotten, never dropped.
        drop(unsafe { ManuallyDrop::take(&mut object.0) });
        std::mem::forget(object);
    }
}

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
        mrb.module_get(c"Godot")
            .and_then(|godot| godot.class_get(mrb, c"Object"))
            .expect("the Godot gem defines Godot::Object")
    })
}

/// Defines Godot::Object, the class every engine class descends from.
pub fn define(mrb: &Mrb, godot: RModule) -> Result<(), Error> {
    let object = godot.define_class(mrb, c"Object", mrb.object_class())?;
    object.set_instance_data_tt(mrb)?;
    object.undef_default_alloc_func(mrb);
    object.define_singleton_method(mrb, c"__make__", method!(make, 0))?;
    object.define_singleton_method(mrb, c"__make_scripted__", method!(make_scripted, 0))?;
    object.define_singleton_method(mrb, c"__allocate__", method!(allocate, 1))?;
    object.define_singleton_method(mrb, c"__singleton__", method!(singleton, 0))?;
    object.define_singleton_method(mrb, c"__has_engine_member__", method!(has_engine_member, 1))?;
    object.define_singleton_method(mrb, c"__has_static_method__", method!(has_static_method, 1))?;
    object.define_singleton_method(mrb, c"__call_static__", method!(call_static, 2))?;
    object.define_singleton_method(
        mrb,
        c"__refuse_instance_method__",
        method!(refuse_instance_method, 1),
    )?;
    object.define_singleton_method(
        mrb,
        c"__engine_constant_by_name__",
        method!(engine_constant_by_name, 1),
    )?;
    declaration::define(mrb, object)?;
    object.define_private_method(mrb, c"__resolve__", method!(resolve, 1))?;
    object.define_private_method(mrb, c"__call__", method!(call, 2))?;
    object.define_private_method(mrb, c"__call_bound__", method!(call_bound, -1))?;
    object.define_private_method(mrb, c"__apply_bound__", method!(apply_bound, 2))?;
    object.define_singleton_method(mrb, c"__read_shape__", method!(read_shape, 1))?;
    object.define_private_method(mrb, c"__instance_id__", method!(instance_id, 0))?;
    object.define_private_method(mrb, c"__label__", method!(label, 0))?;
    object.define_private_method(mrb, c"initialize_copy", method!(initialize_copy, 1))?;
    Ok(())
}

// Godot::Object.__make__: a new engine object of the receiver's engine
// class.
fn make(mrb: &Mrb, class: RClass) -> Result<Value, Error> {
    let path = class.path(mrb).unwrap_or_default();
    let name = read_engine_name(&path);
    let class_db = ClassDb::singleton();
    if !class_db.can_instantiate(name) {
        let message = format!("the engine makes no objects of {path}");
        return Err(make_not_implemented_error(mrb, &message));
    }
    let object = class_db.instantiate(name).to::<Gd<Object>>();
    Ok(mrb.wrap_as(EngineObject::new(object), class).as_value())
}

// Godot::Object.__make_scripted__: a new object of the engine class the
// receiver's script extends, carrying that script, and the receiver's object
// for it, uninitialized and held under the object's key: loosely for a
// reference-counted object, which only Ruby holds yet.
fn make_scripted(mrb: &Mrb, class: RClass) -> Result<Value, Error> {
    let path = class.path(mrb);
    let script = path
        .as_ref()
        .and_then(|path| script_by_class_path(mrb, path))
        .filter(|script| !script.get_instance_base_type().is_empty());
    let class_name = || {
        path.clone()
            .unwrap_or_else(|| "an unnamed class".to_owned())
    };
    let Some(script) = script else {
        let message = format!("no script defines {}, so it makes no object", class_name());
        return Err(make_not_implemented_error(mrb, &message));
    };
    let mut object = ClassDb::singleton()
        .instantiate(&script.get_instance_base_type())
        .to::<Gd<Object>>();
    let id = object.instance_id();
    object.set_script(&script);
    if object.get_script().is_none() {
        if !id.is_ref_counted() {
            object.free();
        }
        let message = format!("{}'s script refused its object here", class_name());
        return Err(make_not_implemented_error(mrb, &message));
    }
    let held = mrb
        .wrap_as(EngineObject::scripted(object), class)
        .as_value();
    let key = derive_object_key(id);
    if id.is_ref_counted() {
        realm::hold_loose(mrb, key, held);
    } else {
        realm::hold(mrb, key, held)?;
    }
    Ok(held)
}

// The script of the class the constant path `path` names: the script its
// file loads as, or for a class inside a file's class, the inner script
// named by that file's path followed by the inner names.
fn script_by_class_path(mrb: &Mrb, path: &str) -> Option<Gd<Script>> {
    let names: Vec<String> = path.split("::").map(str::to_owned).collect();
    let script_path = (1..=names.len()).rev().find_map(|outer| {
        let file = realm::file_by_constant(mrb, &names[..outer])?;
        Some(match &names[outer..] {
            [] => file,
            inner => realm::join_inner_path(&file, inner),
        })
    })?;
    ResourceLoader::singleton()
        .load(&script_path)
        .and_then(|resource| resource.try_cast::<Script>().ok())
}

// Godot::Object.__allocate__(owner): the receiver's object for the engine
// object `owner` stands for, uninitialized, or the TypeError raised when
// that object is not of the engine class the receiver extends, whose
// methods the receiver calls on it.
fn allocate(mrb: &Mrb, class: RClass, owner: &EngineObject) -> Result<Value, Error> {
    check_stands_for(mrb, class, owner)?;
    let object = EngineObject::scripted((*owner.0).clone());
    Ok(mrb.wrap_as(object, class).as_value())
}

// Godot::Object#initialize_copy(original): gives the copy Ruby's dup or
// clone made the engine object `original` stands for, or the TypeError
// raised when that object is freed or the copy cannot take it.
fn initialize_copy(mrb: &Mrb, copy: RTypedData, original: Value) -> Result<RTypedData, Error> {
    super::fill_copy(mrb, copy, original, |class, owner: &EngineObject| {
        check_stands_for(mrb, class, owner).map(|()| owner.clone())
    })
}

// The TypeError raised when an object of `class` would stand for `owner`'s
// engine object although that object is freed or not of the engine class
// `class` extends, whose methods `class` calls on it.
fn check_stands_for(mrb: &Mrb, class: RClass, owner: &EngineObject) -> Result<(), Error> {
    let extended = find_engine_ancestor(mrb, class).unwrap_or_default();
    if owner.0.is_instance_valid() && owner.0.is_class(extended.as_str()) {
        return Ok(());
    }
    let message = format!(
        "{} stands only for a live {extended}",
        class.path(mrb).unwrap_or_default()
    );
    Err(make_type_error(mrb, &message))
}

/// The key a realm holds a scripted object's Ruby object under: its instance
/// id, which no key the realm names itself is: those count down from -1,
/// while a reference-counted object's negative id is far below them.
pub fn derive_object_key(object: InstanceId) -> Key {
    Key::from(object.to_i64())
}

/// A scripted object's engine object as Ruby is given it, for its script's
/// class to make the object's Ruby object from.
pub struct Owner(pub InstanceId);

impl IntoValue for Owner {
    fn into_value(self, mrb: &Mrb) -> Value {
        match Gd::<Object>::try_from_instance_id(self.0) {
            Ok(owner) => mrb.wrap(EngineObject::new(owner)).as_value(),
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
    let object = held.borrow_live_object(mrb, &name)?;
    let class = StringName::from(&object.get_class());
    let mut class_db = ClassDb::singleton();
    let property = if let Some(property) = name.strip_suffix('=') {
        Some((property, "set"))
    } else if object.has_method(name.as_str()) {
        None
    } else {
        Some((name.as_str(), "get"))
    };
    let target = match property {
        Some((property, accessor)) => {
            accessor_by_property(&mut class_db, &class, property, accessor).to_string()
        }
        None => name.clone(),
    };
    if target.is_empty() {
        return Ok(qnil().as_value());
    }
    let extended = find_engine_ancestor(mrb, receiver.class(mrb));
    // A setter takes the value beside any index, a getter nothing more.
    let takes_index = |accessor: &str| {
        let values = usize::from(accessor == "set");
        Arity::find(&class.to_string(), &target).is_some_and(|arity| arity.required > values)
    };
    if let Some((property, accessor)) = property.filter(|(_, accessor)| takes_index(accessor)) {
        let declared = extended.is_some_and(|extended| {
            let extended = StringName::from(extended.as_str());
            accessor_by_property(&mut class_db, &extended, property, accessor).to_string() == target
        });
        return resolve_indexed_property(mrb, accessor, property, declared);
    }
    let extended =
        extended.filter(|extended| class_db.class_has_method(extended.as_str(), target.as_str()));
    let declared = extended.is_some();
    let bound = extended
        .and_then(|extended| BoundMethod::find(&extended, &target))
        .map_or_else(|| qnil().as_value(), |bound| mrb.wrap(bound).as_value());
    let target = Symbol::from(mrb.intern(target.as_bytes())?).as_value();
    Ok(mrb
        .ary_new_from_values(&[target, declared.into_value(mrb), bound])
        .as_value())
}

// The engine method the engine class `class` reads (`get`) or writes (`set`)
// its property `property` with, empty when it has no such property.
fn accessor_by_property(
    class_db: &mut Gd<ClassDb>,
    class: &StringName,
    property: &str,
    accessor: &str,
) -> StringName {
    if accessor == "set" {
        class_db.class_get_property_setter(class, property)
    } else {
        class_db.class_get_property_getter(class, property)
    }
}

// Godot::Object.__has_engine_member__(name): whether the engine class the receiver
// extends has a method or property a call of `name` reaches, for an object
// that has no live engine object to ask.
fn has_engine_member(mrb: &Mrb, class: RClass, name: Symbol) -> bool {
    let Some(extended) = find_engine_ancestor(mrb, class) else {
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
// by name with the property's name ahead of the arguments, and whether the
// engine class the receiver's Ruby class extends declares the property.
fn resolve_indexed_property(
    mrb: &Mrb,
    accessor: &str,
    property: &str,
    declared: bool,
) -> Result<Value, Error> {
    let target = Symbol::from(mrb.intern(accessor.as_bytes())?).as_value();
    let property = mrb.str_new(property.as_bytes()).as_value();
    Ok(mrb
        .ary_new_from_values(&[
            target,
            declared.into_value(mrb),
            qnil().as_value(),
            property,
        ])
        .as_value())
}

// Godot::Object#__call_bound__(bound, *args): calls the engine method
// `bound` with the arguments given and answers what it returns.
fn call_bound(mrb: &Mrb, held: &EngineObject, args: &[Value]) -> Result<Value, Error> {
    let Some((&bound, args)) = args.split_first() else {
        return Err(make_argument_error(mrb, &describe_count(0, 1)));
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
    let object = held.borrow_live_object(mrb, bound.name())?;
    let answer = bound.call(object, &args).map_err(|error| {
        let base = object.get_class().to_string();
        make_refusal_error(mrb, &error, bound.name(), &base, &args, Some(bound.arity()))
    })?;
    Ok(RubyCrossing(&answer).into_ruby(mrb, data))
}

// Godot::Object.__read_shape__(bound): how many arguments the engine method
// `bound` requires and how many more it takes, or nil when it takes any
// number.
fn read_shape(mrb: &Mrb, _class: RClass, bound: &BoundMethod) -> Value {
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
    let mut object = held.borrow_live_object(mrb, &name)?.clone();
    let answer = object.try_call(&name, &args).map_err(|error| {
        let base = object.get_class().to_string();
        make_call_refusal(mrb, &error, &base, &name.to_string(), args.len())
    })?;
    Ok(RubyCrossing(&answer).into_ruby(mrb, data))
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
    let path = class.path(mrb).unwrap_or_default();
    let name = read_engine_name(&path);
    let engine = Engine::singleton();
    if !engine.has_singleton(name) {
        return qnil().as_value();
    }
    engine
        .get_singleton(name)
        .map(|object| mrb.wrap_as(EngineObject::new(object), class).as_value())
        .unwrap_or_else(|| qnil().as_value())
}

// Godot::Object.__has_static_method__(name): whether the receiver's engine class
// has a static method of that name to call on the class.
fn has_static_method(mrb: &Mrb, class: RClass, name: Symbol) -> bool {
    let name = name.name(mrb).unwrap_or_default();
    let path = class.path(mrb).unwrap_or_default();
    is_static_method(read_engine_name(&path), &name)
}

// Godot::Object.__refuse_instance_method__(name): raises NoMethodError in
// GDScript's words when the receiver's engine class has a method of that
// name its objects answer rather than the class.
fn refuse_instance_method(mrb: &Mrb, class: RClass, name: Symbol) -> Result<Value, Error> {
    let name = name.name(mrb).unwrap_or_default();
    let path = class.path(mrb).unwrap_or_default();
    let class = read_engine_name(&path);
    if ClassDb::singleton().class_has_method(class, name.as_str()) {
        return Err(make_no_method_error(
            mrb,
            &describe_instance_call(&name, class),
        ));
    }
    Ok(qnil().as_value())
}

// Godot::Object.__call_static__(name, args): calls the static method `name`
// of the receiver's engine class with `args`.
fn call_static(mrb: &Mrb, class: RClass, name: Symbol, args: RArray) -> Result<Value, Error> {
    let name = name_by_symbol(mrb, name);
    let path = class.path(mrb).unwrap_or_default();
    let class = read_engine_name(&path);
    let data = super::data(mrb);
    let args = to_arguments(mrb, data, args.entries(mrb))?;
    let answer = ClassDb::singleton()
        .try_class_call_static(class, &name, &args)
        .map_err(|error| make_call_refusal(mrb, &error, class, &name.to_string(), args.len()))?;
    Ok(RubyCrossing(&answer).into_ruby(mrb, data))
}

/// The Ruby object for an engine object: the one the realm holds for its
/// script, or an object of its engine class under Godot, or of Godot::Object
/// when that name holds no engine class.
pub(super) fn wrap_object(mrb: &Mrb, data: &BridgeData, object: Gd<Object>) -> Value {
    if let Some(held) = realm::object_by_key(mrb, derive_object_key(object.instance_id())) {
        return held;
    }
    let class = class_by_name(mrb, data, find_exposed_class(&object))
        .unwrap_or_else(|| object_class(mrb, data));
    mrb.wrap_as(EngineObject::new(object), class).as_value()
}

// The class under Godot of the engine class `name`, found once for a realm
// and kept for it, rooted for the collector so the class lives while it is
// kept. A class Ruby put under that name that is no engine class is not one.
fn class_by_name(mrb: &Mrb, data: &BridgeData, name: StringName) -> Option<RClass> {
    let classes = &data.engine_classes;
    if let Some(class) = classes.borrow().get(&name).copied() {
        return Some(class);
    }
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| godot.const_get::<_, RClass>(mrb, name.to_string().as_str()))
        .ok()
        .filter(|class| super::has_ancestor(mrb, *class, object_class(mrb, data)))?;
    mrb.gc_register_forever(class.as_value());
    classes.borrow_mut().insert(name, class);
    Some(class)
}

// The name of the nearest class of `object` the engine exposes to
// extensions, which is the object's own class unless the engine keeps that
// one hidden.
fn find_exposed_class(object: &Gd<Object>) -> StringName {
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

// The engine class `class` extends, itself or through its superclasses, as
// the engine names it.
pub(super) fn find_engine_ancestor(mrb: &Mrb, class: RClass) -> Option<String> {
    let mut current = class;
    loop {
        let path = current.path(mrb)?;
        if let Some(name) = path.strip_prefix("Godot::") {
            return Some(name.to_owned());
        }
        current = current.superclass(mrb)?;
    }
}

// Godot::Object.__engine_constant_by_name__(name): the integer constant or enum
// value of that name the receiver's engine class has, or nil.
fn engine_constant_by_name(mrb: &Mrb, class: RClass, name: Symbol) -> Value {
    let name = name.name(mrb).unwrap_or_default();
    let path = class.path(mrb).unwrap_or_default();
    let class = read_engine_name(&path);
    let class_db = ClassDb::singleton();
    if !class_db.class_has_integer_constant(class, name.as_str()) {
        return qnil().as_value();
    }
    class_db
        .class_get_integer_constant(class, name.as_str())
        .into_value(mrb)
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
            value::to_engine(mrb, data, arg, 1).map_err(|reason| make_argument_error(mrb, &reason))
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
    fn borrow_live_object(
        &self,
        mrb: &Mrb,
        method: &(impl fmt::Display + ?Sized),
    ) -> Result<&Gd<Object>, Error> {
        if self.0.is_instance_valid() {
            Ok(&*self.0)
        } else {
            let message = format!(
                "Attempt to call function '{method}' in base 'previously freed' on a null instance."
            );
            Err(make_call_error(mrb, &message))
        }
    }
}
