//! How an export's keyword tells the editor to show a property, as a
//! GDScript variable's `@export_*` annotation does: which keywords name a
//! hint, the types each takes, and what the editor reads it with; the types
//! are named as GDScript names them, which the bridge's messages share.

use godot::builtin::{VarArray, Variant, VariantType};
use godot::global::type_string;
use godot::meta::ToGodot;
use godot::obj::{EngineBitfield, EngineEnum};
use godot::register::info::{PropertyHint, PropertyUsageFlags};

use crate::snapshot::Property;

/// The name GDScript gives a variant type.
pub fn name_type(kind: VariantType) -> String {
    type_string(i64::from(kind.ord())).to_string()
}

/// A hint an export's keyword names, each one the `@export_*` annotation it
/// answers to, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hint {
    None,
    Range,
    Enum,
    Flags,
    File,
    Dir,
    Multiline,
    Placeholder,
    ColorNoAlpha,
    ExpEasing,
    FilePath,
    GlobalFile,
    GlobalDir,
    Layers(PropertyHint),
    NodePath,
    Storage,
    ToolButton,
    Custom,
}

// The layer annotations, by the keyword each answers to.
const LAYERS: [(&str, PropertyHint); 7] = [
    ("flags_2d_render", PropertyHint::LAYERS_2D_RENDER),
    ("flags_2d_physics", PropertyHint::LAYERS_2D_PHYSICS),
    ("flags_2d_navigation", PropertyHint::LAYERS_2D_NAVIGATION),
    ("flags_3d_render", PropertyHint::LAYERS_3D_RENDER),
    ("flags_3d_physics", PropertyHint::LAYERS_3D_PHYSICS),
    ("flags_3d_navigation", PropertyHint::LAYERS_3D_NAVIGATION),
    ("flags_avoidance", PropertyHint::LAYERS_AVOIDANCE),
];

impl Hint {
    /// The hint `keyword` names, `none` naming none; any other word is no
    /// hint a property can be exported with, and is refused.
    pub fn from_keyword(keyword: &str) -> Result<Self, String> {
        Ok(match keyword {
            "none" => Self::None,
            "range" => Self::Range,
            "enum" => Self::Enum,
            "flags" => Self::Flags,
            "file" => Self::File,
            "dir" => Self::Dir,
            "multiline" => Self::Multiline,
            "placeholder" => Self::Placeholder,
            "color_no_alpha" => Self::ColorNoAlpha,
            "exp_easing" => Self::ExpEasing,
            "file_path" => Self::FilePath,
            "global_file" => Self::GlobalFile,
            "global_dir" => Self::GlobalDir,
            "node_path" => Self::NodePath,
            "storage" => Self::Storage,
            "tool_button" => Self::ToolButton,
            "custom" => Self::Custom,
            layer if let Some((_, hint)) = LAYERS.iter().find(|(name, _)| *name == layer) => {
                Self::Layers(*hint)
            }
            _ => {
                return Err(format!(
                    "\"{keyword}:\" is not a hint a property can be exported with."
                ));
            }
        })
    }

    /// The engine's hint for a property of type `kind`, unless the type
    /// cannot be read with it. Each takes the types the annotation it
    /// answers to takes, and a type none of them takes is refused in
    /// GDScript's words.
    pub fn choose_property_hint(self, kind: VariantType) -> Result<PropertyHint, String> {
        let (hint, takes): (PropertyHint, &[VariantType]) = match self {
            // `declare` takes a custom hint from what it is written with.
            Self::None | Self::Storage | Self::Custom => return Ok(PropertyHint::NONE),
            Self::Range => (PropertyHint::RANGE, &[VariantType::INT, VariantType::FLOAT]),
            Self::Enum => (
                PropertyHint::ENUM,
                &[
                    VariantType::INT,
                    VariantType::STRING,
                    VariantType::STRING_NAME,
                ],
            ),
            Self::Flags => (PropertyHint::FLAGS, &[VariantType::INT]),
            Self::File => (PropertyHint::FILE, &[VariantType::STRING]),
            Self::Dir => (PropertyHint::DIR, &[VariantType::STRING]),
            Self::Multiline => (PropertyHint::MULTILINE_TEXT, &[VariantType::STRING]),
            Self::Placeholder => (PropertyHint::PLACEHOLDER_TEXT, &[VariantType::STRING]),
            Self::ColorNoAlpha => (PropertyHint::COLOR_NO_ALPHA, &[VariantType::COLOR]),
            Self::ExpEasing => (PropertyHint::EXP_EASING, &[VariantType::FLOAT]),
            Self::FilePath => (PropertyHint::FILE_PATH, &[VariantType::STRING]),
            Self::GlobalFile => (PropertyHint::GLOBAL_FILE, &[VariantType::STRING]),
            Self::GlobalDir => (PropertyHint::GLOBAL_DIR, &[VariantType::STRING]),
            Self::Layers(hint) => (hint, &[VariantType::INT]),
            Self::NodePath => (
                PropertyHint::NODE_PATH_VALID_TYPES,
                &[VariantType::NODE_PATH],
            ),
            Self::ToolButton => (PropertyHint::TOOL_BUTTON, &[VariantType::CALLABLE]),
        };
        if takes.contains(&kind) {
            return Ok(hint);
        }
        Err(format!(
            "\"{}:\" requires a variable of type {}, but type \"{}\" was given instead.",
            self.keyword(),
            list_kinds(takes),
            name_type(kind)
        ))
    }

    /// What the editor reads the hint with, from the value written with its
    /// keyword: a range's bounds and step, the names a value may take, the
    /// files to choose among, or the text an empty field shows; `dir:`,
    /// `multiline:` and `file: true` are read with nothing. None when the
    /// value is not the list a range, `enum:` or `flags:` is read from.
    pub fn write_hint_string(self, written: &Variant) -> Option<String> {
        match self {
            Self::Range
            | Self::Enum
            | Self::Flags
            | Self::ExpEasing
            | Self::NodePath
            | Self::ToolButton
                if written.get_type() == VariantType::ARRAY =>
            {
                let values = written.try_to::<VarArray>().ok()?;
                Some(
                    values
                        .iter_shared()
                        .map(|value| format_value(self, &value))
                        .collect::<Vec<_>>()
                        .join(","),
                )
            }
            Self::Range | Self::Enum | Self::Flags => None,
            Self::File | Self::FilePath | Self::GlobalFile | Self::ExpEasing | Self::NodePath
                if *written == true.to_variant() =>
            {
                Some(String::new())
            }
            Self::File
            | Self::FilePath
            | Self::GlobalFile
            | Self::Placeholder
            | Self::ToolButton => Some(written.to_string()),
            Self::Custom => written
                .try_to::<VarArray>()
                .ok()
                .and_then(|custom| custom.get(1))
                .map(|text| text.to_string()),
            Self::ExpEasing | Self::NodePath => None,
            Self::None
            | Self::Dir
            | Self::Multiline
            | Self::ColorNoAlpha
            | Self::GlobalDir
            | Self::Layers(_)
            | Self::Storage => Some(String::new()),
        }
    }

    /// The property `name` an export declares with `default`, shown as the
    /// keyword says from `written`: its type is the declared value's, so a
    /// value naming none is refused, except for a tool button, a Callable
    /// each object makes of its own method.
    pub fn declare(
        self,
        name: String,
        default: &Variant,
        written: &Variant,
    ) -> Result<Property, String> {
        let property = match self {
            Self::ToolButton => {
                Property::new(name, &Variant::nil()).with_kind(VariantType::CALLABLE)
            }
            _ if default.get_type() == VariantType::NIL => {
                return Err(
                    "Cannot use \"export\" because the type of the initialized value can't be inferred."
                        .to_owned(),
                );
            }
            _ => Property::new(name, default),
        };
        // The gem refuses a value of another shape where it is written.
        let hint_string = self.write_hint_string(written).unwrap_or_default();
        if let Some((custom, usage)) = self.read_custom(written) {
            return Ok(property.with_hint(custom, hint_string).with_usage(usage));
        }
        let property_hint = self.choose_property_hint(property.kind)?;
        Ok(property
            .with_hint(property_hint, hint_string)
            .with_usage(self.usage()))
    }

    /// The hint a custom keyword names, with what the editor reads it with
    /// and the usage it is declared with besides a script variable's, as
    /// `@export_custom` takes them; none for any other keyword.
    fn read_custom(self, written: &Variant) -> Option<(PropertyHint, PropertyUsageFlags)> {
        if self != Self::Custom {
            return None;
        }
        let custom = written.try_to::<VarArray>().ok()?;
        let hint = PropertyHint::try_from_ord(custom.get(0)?.try_to::<i32>().ok()?)?;
        let usage = match custom.get(2) {
            Some(usage) => {
                PropertyUsageFlags::from_ord(usage.try_to::<i64>().ok()?.try_into().ok()?)
            }
            None => PropertyUsageFlags::DEFAULT,
        };
        Some((hint, usage))
    }

    /// What the property is used for besides being a script variable: kept
    /// in a saved scene and not shown for `storage:`, shown and not kept for
    /// `tool_button:`, and both for any other.
    fn usage(self) -> PropertyUsageFlags {
        match self {
            Self::Storage => PropertyUsageFlags::STORAGE,
            Self::ToolButton => PropertyUsageFlags::EDITOR,
            _ => PropertyUsageFlags::DEFAULT,
        }
    }

    fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Range => "range",
            Self::Enum => "enum",
            Self::Flags => "flags",
            Self::File => "file",
            Self::Dir => "dir",
            Self::Multiline => "multiline",
            Self::Placeholder => "placeholder",
            Self::ColorNoAlpha => "color_no_alpha",
            Self::ExpEasing => "exp_easing",
            Self::FilePath => "file_path",
            Self::GlobalFile => "global_file",
            Self::GlobalDir => "global_dir",
            Self::Layers(hint) => LAYERS
                .iter()
                .find(|(_, layer)| *layer == hint)
                .map_or("flags", |(name, _)| name),
            Self::NodePath => "node_path",
            Self::Storage => "storage",
            Self::ToolButton => "tool_button",
            Self::Custom => "custom",
        }
    }
}

// One value of a list the editor reads a hint with: a range's bound as
// GDScript writes it, where a whole number carries no fraction, or a name.
fn format_value(hint: Hint, value: &Variant) -> String {
    match value.try_to::<f64>() {
        Ok(number) if hint == Hint::Range && number.fract() == 0.0 => format!("{}", number as i64),
        _ => value.to_string(),
    }
}

// The types a hint takes, as GDScript lists them in the same refusal: the
// last is reached through "or", and three or more are separated by commas.
fn list_kinds(kinds: &[VariantType]) -> String {
    let names: Vec<String> = kinds
        .iter()
        .map(|kind| format!("\"{}\"", name_type(*kind)))
        .collect();
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, [first])) => format!("{first} or {last}"),
        Some((last, rest)) => format!("{}, or {last}", rest.join(", ")),
    }
}
