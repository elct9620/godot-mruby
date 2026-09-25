//! How an export's keyword tells the editor to show a property, as a
//! GDScript variable's `@export_*` annotation does: which keywords name a
//! hint, the types each takes, and what the editor reads it with.

use godot::builtin::{VarArray, Variant, VariantType};
use godot::meta::ToGodot;
use godot::register::info::PropertyHint;

use crate::bridge::type_name;

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
}

impl Hint {
    /// The hint `keyword` names, `none` naming none; any other word is no
    /// hint a property can be exported with, and is refused.
    pub fn by_keyword(keyword: &str) -> Result<Self, String> {
        Ok(match keyword {
            "none" => Self::None,
            "range" => Self::Range,
            "enum" => Self::Enum,
            "flags" => Self::Flags,
            "file" => Self::File,
            "dir" => Self::Dir,
            "multiline" => Self::Multiline,
            "placeholder" => Self::Placeholder,
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
    pub fn property_hint(self, kind: VariantType) -> Result<PropertyHint, String> {
        let (hint, takes): (PropertyHint, &[VariantType]) = match self {
            Self::None => return Ok(PropertyHint::NONE),
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
        };
        if takes.contains(&kind) {
            return Ok(hint);
        }
        Err(format!(
            "\"{}:\" requires a variable of type {}, but type \"{}\" was given instead.",
            self.keyword(),
            kind_list(takes),
            type_name(kind)
        ))
    }

    /// What the editor reads the hint with, from the value written with its
    /// keyword: a range's bounds and step, the names a value may take, the
    /// files to choose among, or the text an empty field shows; `dir:`,
    /// `multiline:` and `file: true` are read with nothing. None when the
    /// value is not the list a range, `enum:` or `flags:` is read from.
    pub fn hint_string(self, written: &Variant) -> Option<String> {
        match self {
            Self::Range | Self::Enum | Self::Flags => {
                let values = written.try_to::<VarArray>().ok()?;
                Some(
                    values
                        .iter_shared()
                        .map(|value| text(self, &value))
                        .collect::<Vec<_>>()
                        .join(","),
                )
            }
            Self::File if *written == true.to_variant() => Some(String::new()),
            Self::File | Self::Placeholder => Some(written.to_string()),
            Self::None | Self::Dir | Self::Multiline => Some(String::new()),
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
        }
    }
}

// One value of a list the editor reads a hint with: a range's bound as
// GDScript writes it, where a whole number carries no fraction, or a name.
fn text(hint: Hint, value: &Variant) -> String {
    match value.try_to::<f64>() {
        Ok(number) if hint == Hint::Range && number.fract() == 0.0 => format!("{}", number as i64),
        _ => value.to_string(),
    }
}

// The types a hint takes, as GDScript lists them in the same refusal: the
// last is reached through "or", and three or more are separated by commas.
fn kind_list(kinds: &[VariantType]) -> String {
    let names: Vec<String> = kinds
        .iter()
        .map(|kind| format!("\"{}\"", type_name(*kind)))
        .collect();
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, [first])) => format!("{first} or {last}"),
        Some((last, rest)) => format!("{}, or {last}", rest.join(", ")),
    }
}
