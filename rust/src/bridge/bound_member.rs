//! A value type's member found once through the engine's getter for it, so
//! a later read skips the engine's lookup by name. The engine still reads
//! every member: its getter runs the same code its variant call does, and
//! the member's type is the one the engine gave the first time, which the
//! value's type fixes. A Vector2 Ruby holds as its components answers its
//! `x` and `y` from them, the memory the engine's getter reads.

use std::ptr;

use beni::{DataType, Mrb, RClass, TypedData};
use godot::builtin::{StringName, Variant, VariantType, Vector2, Vector2Axis, real};
use godot::sys;

type Getter = unsafe extern "C" fn(sys::GDExtensionConstTypePtr, sys::GDExtensionTypePtr);

/// A member of a value type, read through the engine's getter for it.
pub struct BoundMember {
    getter: Getter,
    kind: VariantType,
    answer: VariantType,
    axis: Option<Vector2Axis>,
}

// SAFETY: the getter is the engine's, unchanged while the engine runs, and a
// realm is entered by one thread at a time.
unsafe impl Send for BoundMember {}

static BOUND_MEMBER: DataType<BoundMember> = DataType::new(c"Godot::BoundMember");

// SAFETY: `class` marks the class it makes, the only one a bound member is
// wrapped as.
unsafe impl TypedData for BoundMember {
    fn class(mrb: &Mrb) -> RClass {
        super::find_class_once(mrb, &super::data(mrb).bound_member_class, || {
            let class = mrb
                .class_new(mrb.object_class())
                .expect("mruby makes a class");
            class
                .set_instance_data_tt(mrb)
                .expect("mruby marks a class's data");
            class
        })
    }

    fn data_type() -> &'static DataType<Self> {
        &BOUND_MEMBER
    }
}

impl BoundMember {
    /// The member `name` of `value`'s type, bound to the type of the value
    /// `read` is, or none when the engine has no getter for it.
    pub fn find(value: &Variant, name: &StringName, read: &Variant) -> Option<Self> {
        let kind = value.get_type();
        // SAFETY: the interface is initialized while the extension runs, and
        // the name lives for the call.
        let getter = unsafe {
            sys::interface_fn!(variant_get_ptr_getter)(kind_sys(kind), name.string_sys())
        }?;
        let axis = match (kind, name.to_string().as_str()) {
            (VariantType::VECTOR2, "x") => Some(Vector2Axis::X),
            (VariantType::VECTOR2, "y") => Some(Vector2Axis::Y),
            _ => None,
        };
        Some(Self {
            getter,
            kind,
            answer: read.get_type(),
            axis,
        })
    }

    /// The component of `vector` the member is, or none when it is a member
    /// of another type.
    pub fn read_component(&self, vector: Vector2) -> Option<real> {
        self.axis.map(|axis| vector[axis])
    }

    /// The member of `value`, which is of the type the member was found
    /// for, or none when it is of another.
    pub fn read(&self, value: &Variant) -> Option<Variant> {
        if value.get_type() != self.kind {
            return None;
        }
        let mut answer = new_default(self.answer);
        // SAFETY: `value` is of the getter's type and `answer` of the type
        // the getter writes, so each internal pointer is of the type the
        // getter takes.
        unsafe {
            (self.getter)(internal(value), internal_mut(&mut answer));
        }
        Some(answer)
    }
}

fn kind_sys(kind: VariantType) -> sys::GDExtensionVariantType {
    kind.ord as sys::GDExtensionVariantType
}

// A value of type `kind` as the engine's default constructor builds it, for
// a typed function to write over.
fn new_default(kind: VariantType) -> Variant {
    // SAFETY: the interface is initialized while the extension runs, and
    // every value type has a constructor taking nothing.
    unsafe {
        Variant::new_with_var_uninit(|answer| {
            let mut error = sys::default_call_error();
            sys::interface_fn!(variant_construct)(
                kind_sys(kind),
                answer,
                ptr::null(),
                0,
                ptr::addr_of_mut!(error),
            );
        })
    }
}

// The engine's pointer to the data `variant` holds, for reading.
fn internal(variant: &Variant) -> sys::GDExtensionConstTypePtr {
    // SAFETY: the engine only computes where the data lies, writing nothing.
    unsafe { internal_getter(variant.get_type())(variant.var_sys().cast_mut()) }
        .cast_const()
        .cast()
}

// The engine's pointer to the data `variant` holds, for writing.
fn internal_mut(variant: &mut Variant) -> sys::GDExtensionTypePtr {
    let kind = variant.get_type();
    // SAFETY: the variant is borrowed mutably for the pointer's use.
    unsafe { internal_getter(kind)(variant.var_sys_mut()) }.cast()
}

// The engine's function finding the data a variant of type `kind` holds.
fn internal_getter(
    kind: VariantType,
) -> unsafe extern "C" fn(sys::GDExtensionVariantPtr) -> *mut std::ffi::c_void {
    // SAFETY: the interface is initialized while the extension runs.
    unsafe { sys::interface_fn!(variant_get_ptr_internal_getter)(kind_sys(kind)) }
        .expect("the engine has an internal getter for every type")
}
