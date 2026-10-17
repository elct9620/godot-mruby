//! What a realm has published of the classes its files define. Godot asks a
//! script its shape on any thread, so the answers are kept here, outside the
//! realm: asking never waits for the thread inside it, and what is read is
//! whole, since a realm publishes a class only once its file has run.

use std::collections::{BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, LazyLock, OnceLock, PoisonError, RwLock};

use godot::builtin::{PackedByteArray, VarArray, VarDictionary, Variant, VariantType};
use godot::classes::{Os, Resource, ResourceLoader};
use godot::global::{bytes_to_var, var_to_bytes};
use godot::meta::ToGodot;
use godot::obj::{Gd, Singleton};
use godot::register::info::{PropertyHint, PropertyUsageFlags};

/// A method a class lets peers call over the network, as a GDScript's
/// `@rpc` does: who may call it, whether it runs on the caller too, how it
/// travels and on which channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rpc {
    pub method: String,
    mode: i64,
    is_call_local: bool,
    transfer_mode: i64,
    channel: i64,
}

impl Rpc {
    /// The rpc of `method` the words `mode`, `sync` and `transfer_mode` and
    /// the `channel` describe, as `@rpc` takes them, or GDScript's refusal of
    /// a word it does not take.
    pub fn read(
        method: String,
        mode: &str,
        sync: &str,
        transfer_mode: &str,
        channel: i64,
    ) -> Result<Self, String> {
        // MultiplayerAPI.RPCMode and MultiplayerPeer.TransferMode.
        let mode = match mode {
            "any_peer" => 1,
            "authority" => 2,
            _ => return Err(describe_rpc_refusal(mode)),
        };
        let is_call_local = match sync {
            "call_local" => true,
            "call_remote" => false,
            _ => return Err(describe_rpc_refusal(sync)),
        };
        let transfer_mode = match transfer_mode {
            "unreliable" => 0,
            "unreliable_ordered" => 1,
            "reliable" => 2,
            _ => return Err(describe_rpc_refusal(transfer_mode)),
        };
        Ok(Self {
            method,
            mode,
            is_call_local,
            transfer_mode,
            channel,
        })
    }

    /// The rpc as Godot reads it from a script's rpc config: its mode, and
    /// whatever it sets apart from the defaults, as GDScript's config has it.
    pub fn describe(&self) -> VarDictionary {
        let mut described = VarDictionary::new();
        described.set("rpc_mode", self.mode);
        if self.is_call_local {
            described.set("call_local", true);
        }
        if self.transfer_mode != 2 {
            described.set("transfer_mode", self.transfer_mode);
        }
        if self.channel != 0 {
            described.set("channel", self.channel);
        }
        described
    }
}

// GDScript's words for an `@rpc` argument it does not take.
fn describe_rpc_refusal(word: &str) -> String {
    format!(
        "Invalid RPC argument \"{word}\". Must be one of: \"call_local\"/\"call_remote\" (sync), \"any_peer\"/\"authority\" (mode), \"reliable\"/\"unreliable\"/\"unreliable_ordered\" (transfer mode)."
    )
}

/// A signal a class declared, with the parameters it was declared with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signal {
    pub name: String,
    pub parameters: Vec<Parameter>,
}

impl Signal {
    // Whether `other` names the same parameters in the same order, whatever
    // their types.
    fn has_parameters_named_as(&self, other: &Self) -> bool {
        self.parameters
            .iter()
            .map(|parameter| &parameter.name)
            .eq(other.parameters.iter().map(|parameter| &parameter.name))
    }
}

/// A value a signal carries, as Godot reads it: its name, and the type it
/// was declared with, nil for any type and an object's with the name of its
/// class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub kind: VariantType,
    pub class_name: String,
}

impl Parameter {
    /// The parameter `name` of the type `kind`, nil being any type, as
    /// GDScript's untyped parameter is.
    pub fn new(name: String, kind: VariantType) -> Self {
        Self {
            name,
            kind,
            class_name: String::new(),
        }
    }

    /// The parameter taking objects of the class Godot lists as
    /// `class_name`.
    pub fn with_class(self, class_name: String) -> Self {
        Self {
            kind: VariantType::OBJECT,
            class_name,
            ..self
        }
    }

    /// How Godot reads the parameter: one of any type is nil read as any.
    pub fn usage(&self) -> PropertyUsageFlags {
        if self.kind == VariantType::NIL {
            PropertyUsageFlags::NIL_IS_VARIANT
        } else {
            PropertyUsageFlags::NONE
        }
    }
}

/// A property a class exported, as Godot reads it: the name it is written
/// and read by, the type its declared value gave it, that value, and the
/// hint the editor shows it with. The value is kept apart from any Variant,
/// since a snapshot is read from any thread while a Variant belongs to the
/// one holding it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub kind: VariantType,
    pub hint: PropertyHint,
    pub hint_string: String,
    // What the property is used for besides being a script variable.
    usage: PropertyUsageFlags,
    default: DeclaredValue,
    default_text: String,
}

// The value a property was declared with, as a snapshot keeps it: a
// resource a file holds by that file's path, which the engine loads it from
// again, an array or a dictionary by what it holds, and any other value as
// the bytes the engine writes it as, which carry no object. A value that is
// or holds an object no file holds is kept as nothing, as GDScript keeps no
// default it cannot work out before running.
#[derive(Clone, Debug, PartialEq, Eq)]
enum DeclaredValue {
    Resource(String),
    Array(Vec<DeclaredValue>),
    Dictionary(Vec<(DeclaredValue, DeclaredValue)>),
    Bytes(Vec<u8>),
}

impl Property {
    /// The usage of a script's own variable: one the editor shows and a
    /// scene stores, unless it was declared to be used otherwise.
    pub fn usage(&self) -> PropertyUsageFlags {
        self.usage | PropertyUsageFlags::SCRIPT_VARIABLE
    }

    /// The property `name`, taking its type from the value it is declared
    /// with and shown as the editor shows a property of that type.
    pub fn new(name: String, default: &Variant) -> Self {
        Self {
            name,
            kind: default.get_type(),
            hint: PropertyHint::NONE,
            hint_string: String::new(),
            usage: PropertyUsageFlags::DEFAULT,
            default: DeclaredValue::from_variant(default),
            default_text: default.to_string(),
        }
    }

    /// The property as the editor is to show it, `hint_string` being what
    /// the hint is read with.
    pub fn with_hint(self, hint: PropertyHint, hint_string: String) -> Self {
        Self {
            hint,
            hint_string,
            ..self
        }
    }

    /// The property used as `usage` says besides being a script variable.
    pub fn with_usage(self, usage: PropertyUsageFlags) -> Self {
        Self { usage, ..self }
    }

    /// The property of type `kind` whatever value it is declared with, as a
    /// tool button is a Callable each object makes of its own method.
    pub fn with_kind(self, kind: VariantType) -> Self {
        Self { kind, ..self }
    }

    /// The property as an object of the class `class` names, which Godot
    /// fills in: its type is the object's rather than the declared value's,
    /// as a class names a type no value has to carry.
    pub fn with_class(self, hint: PropertyHint, class: String) -> Self {
        Self {
            kind: VariantType::OBJECT,
            hint,
            hint_string: class,
            ..self
        }
    }

    /// The value the property was declared with.
    pub fn default_value(&self) -> Variant {
        self.default.to_variant()
    }

    /// The value the property was declared with, as Godot prints it, which
    /// reads without the engine.
    pub fn default_text(&self) -> &str {
        &self.default_text
    }
}

impl DeclaredValue {
    fn from_variant(value: &Variant) -> Self {
        Self::read(value).unwrap_or_else(|| Self::Bytes(var_to_bytes(&Variant::nil()).to_vec()))
    }

    // `value` as kept, or none when it is or holds an object no file holds.
    fn read(value: &Variant) -> Option<Self> {
        match value.get_type() {
            VariantType::OBJECT => {
                let path = value.try_to::<Gd<Resource>>().ok()?.get_path().to_string();
                (!path.is_empty()).then_some(Self::Resource(path))
            }
            VariantType::ARRAY => value
                .to::<VarArray>()
                .iter_shared()
                .map(|element| Self::read(&element))
                .collect::<Option<_>>()
                .map(Self::Array),
            VariantType::DICTIONARY => value
                .to::<VarDictionary>()
                .iter_shared()
                .map(|(key, entry)| Some((Self::read(&key)?, Self::read(&entry)?)))
                .collect::<Option<_>>()
                .map(Self::Dictionary),
            _ => Some(Self::Bytes(var_to_bytes(value).to_vec())),
        }
    }

    fn to_variant(&self) -> Variant {
        match self {
            Self::Resource(path) => ResourceLoader::singleton()
                .load(path.as_str())
                .map_or_else(Variant::nil, |resource| resource.to_variant()),
            Self::Array(elements) => {
                let mut array = VarArray::new();
                for element in elements {
                    array.push(&element.to_variant());
                }
                array.to_variant()
            }
            Self::Dictionary(entries) => {
                let mut dictionary = VarDictionary::new();
                for (key, entry) in entries {
                    dictionary.set(&key.to_variant(), &entry.to_variant());
                }
                dictionary.to_variant()
            }
            Self::Bytes(bytes) => bytes_to_var(&PackedByteArray::from(bytes.as_slice())),
        }
    }
}

/// A heading the editor shows the properties under: a category, as
/// `export_category` writes one or as a class is headed by the file at
/// `path`, or a group or subgroup, as `export_group` and `export_subgroup`
/// write one, taking the properties whose names begin with `prefix`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Heading {
    Category { name: String, path: String },
    Group { name: String, prefix: String },
    Subgroup { name: String, prefix: String },
}

impl Heading {
    /// The name the heading is shown with.
    pub fn name(&self) -> &str {
        match self {
            Self::Category { name, .. }
            | Self::Group { name, .. }
            | Self::Subgroup { name, .. } => name,
        }
    }

    /// What Godot reads the heading with: a category's path, or the prefix a
    /// group or subgroup takes its properties by.
    pub fn hint_string(&self) -> &str {
        match self {
            Self::Category { path, .. } => path,
            Self::Group { prefix, .. } | Self::Subgroup { prefix, .. } => prefix,
        }
    }

    /// The usage that tells Godot which kind of heading it is.
    pub fn usage(&self) -> PropertyUsageFlags {
        match self {
            Self::Category { .. } => PropertyUsageFlags::CATEGORY,
            Self::Group { .. } => PropertyUsageFlags::GROUP,
            Self::Subgroup { .. } => PropertyUsageFlags::SUBGROUP,
        }
    }
}

/// What a class body declared for the editor to show, in the order it was
/// declared: a property, or a heading the properties after it are under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Member {
    Property(Property),
    Heading(Heading),
}

impl Member {
    /// The property this member is, unless it is a heading.
    pub fn property(&self) -> Option<&Property> {
        match self {
            Self::Property(property) => Some(property),
            Self::Heading(_) => None,
        }
    }
}

/// What a method its file's source defines calls: `await` itself, and the
/// names of the methods it calls, as the source writes them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MethodCalls {
    pub method: String,
    pub has_await: bool,
    pub callees: Vec<String>,
}

/// What a class has once its file has run: what its body declared, and the
/// methods it defines, those metaprogramming defined included, with the
/// digest of the source it ran and the names that source's `export` and
/// `signal` calls write, which tell what the source Godot holds now has
/// changed and what it cannot answer for, and what the methods its source
/// defines call.
#[derive(Clone, Debug, Default)]
pub struct Class {
    pub signals: Vec<Signal>,
    pub members: Vec<Member>,
    pub rpcs: Vec<Rpc>,
    pub methods: Vec<String>,
    pub digest: u64,
    pub export_names: Vec<String>,
    pub signal_names: Vec<String>,
    pub method_calls: Vec<MethodCalls>,
}

impl Class {
    // The signal of that name the class declared.
    fn signal_by_name(&self, name: &str) -> Option<&Signal> {
        self.signals.iter().find(|signal| signal.name == name)
    }

    // The property of that name the class declared.
    fn property_by_name(&self, name: &str) -> Option<&Property> {
        self.members
            .iter()
            .filter_map(Member::property)
            .find(|property| property.name == name)
    }

    // Whether `member` is a property the class exported through a name no
    // `export` call of the source it ran spelled, which only running tells.
    fn is_exported_unwritten(&self, member: &Member) -> bool {
        member
            .property()
            .is_some_and(|property| !self.export_names.contains(&property.name))
    }
}

/// What a file's source writes, for its class to be answered from: what its
/// header declares and the digest of that source.
#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    pub signals: &'a [Signal],
    pub rpcs: &'a [Rpc],
    pub signal_names: &'a [String],
    pub exports: &'a [Export],
    pub digest: u64,
}

/// A digest of a file's source, which differs whenever the source does.
pub fn digest(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// What a class body declares for the editor, as its source writes it.
#[derive(Clone, Debug, PartialEq)]
pub enum Export {
    /// A property or a heading the source writes out in full.
    Member(Member),
    /// An export whose value or hint the source does not write out: what
    /// the file declared as it ran answers for it, or `bare`, the property
    /// without the hint its keyword names, while the file has not run.
    Name {
        name: String,
        bare: Option<Property>,
    },
}

impl Export {
    /// What answers for it while the file has not run.
    pub fn member(&self) -> Option<Member> {
        match self {
            Self::Member(member) => Some(member.clone()),
            Self::Name { bare, .. } => bare.clone().map(Member::Property),
        }
    }
}

/// What the classes of a realm's files have, as one value that never changes
/// once it is published. Each class is shared between the snapshots that
/// have it, so a realm publishing one class more copies no other class.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    classes: HashMap<Arc<str>, Arc<Class>>,
    // The names of the methods that may await, worked out at the first
    // question and again once a class is recorded.
    awaiting_names: OnceLock<BTreeSet<String>>,
}

impl Snapshot {
    /// Whether a method named `method` may await, as some class's source
    /// writes one: its body calls `await`, or calls a method of a name that
    /// may. A name is taken for every method so named, since nothing tells
    /// which class a call reaches; a method so taken that never waits only
    /// costs its fiber.
    pub fn is_awaiting(&self, method: &str) -> bool {
        self.awaiting_names
            .get_or_init(|| self.collect_awaiting())
            .contains(method)
    }

    // The names `is_awaiting` answers for, each method reaching `await`
    // through any number of calls.
    fn collect_awaiting(&self) -> BTreeSet<String> {
        let calls: Vec<&MethodCalls> = self
            .classes
            .values()
            .flat_map(|class| class.method_calls.iter())
            .collect();
        let mut awaiting: BTreeSet<String> = calls
            .iter()
            .filter(|calls| calls.has_await)
            .map(|calls| calls.method.clone())
            .collect();
        loop {
            let reached: Vec<String> = calls
                .iter()
                .filter(|calls| !awaiting.contains(&calls.method))
                .filter(|calls| calls.callees.iter().any(|name| awaiting.contains(name)))
                .map(|calls| calls.method.clone())
                .collect();
            if reached.is_empty() {
                return awaiting;
            }
            awaiting.extend(reached);
        }
    }

    /// The signals the class of the file at `path` declared, in the order it
    /// declared them; none for a file that has not run.
    pub fn signals_by_path(&self, path: &str) -> &[Signal] {
        self.classes
            .get(path)
            .map_or(&[], |class| class.signals.as_slice())
    }

    /// What the class of the file at `path` declared for the editor, in the
    /// order it declared it; none for a file that has not run.
    pub fn members_by_path(&self, path: &str) -> &[Member] {
        self.classes
            .get(path)
            .map_or(&[], |class| class.members.as_slice())
    }

    /// The properties the class of the file at `path` exported, in the order
    /// it declared them; none for a file that has not run.
    pub fn properties_by_path(&self, path: &str) -> impl Iterator<Item = &Property> {
        self.members_by_path(path)
            .iter()
            .filter_map(Member::property)
    }

    /// The rpcs the classes of the files at `files` declared, the first
    /// file's first and one to a method: what each file declared as it ran
    /// from the source Godot holds, or what that source writes.
    pub fn collect_rpcs<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, Source<'a>)>,
    ) -> Vec<Rpc> {
        let mut rpcs: Vec<Rpc> = Vec::new();
        for (path, source) in files {
            let declared = match self.classes.get(path) {
                Some(class) if class.digest == source.digest => &class.rpcs[..],
                _ => source.rpcs,
            };
            for rpc in declared {
                if !rpcs.iter().any(|kept| kept.method == rpc.method) {
                    rpcs.push(rpc.clone());
                }
            }
        }
        rpcs
    }

    /// The signals the classes of the files at `files` declared, the first
    /// file's first and one to a name, as a class has what it declared and
    /// what it inherits. Each file comes with what its source writes, as
    /// `collect_members` takes it.
    pub fn collect_signals<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, Source<'a>)>,
    ) -> Vec<Signal> {
        let mut signals: Vec<Signal> = Vec::new();
        for (path, source) in files {
            for signal in self.merge_file_signals(path, source) {
                if !signals.iter().any(|kept| kept.name == signal.name) {
                    signals.push(signal);
                }
            }
        }
        signals
    }

    // The signals the class of the file at `path` declared, answered as
    // `merge_file_members` answers its members: from its source until it has
    // run, then from the run while the source is the one it ran. A source
    // changed since answers in the order it writes, taking what the run
    // declared for a signal whose parameters it does not write out or writes
    // by the same names, since only a run knows their types, and keeping what
    // the run declared through a name no `signal` call spelled.
    fn merge_file_signals(&self, path: &str, source: Source) -> Vec<Signal> {
        let Some(class) = self.classes.get(path) else {
            return source.signals.to_vec();
        };
        if class.digest == source.digest {
            return class.signals.clone();
        }
        let mut signals: Vec<Signal> = source
            .signal_names
            .iter()
            .filter_map(|name| {
                let ran = class.signal_by_name(name);
                match source.signals.iter().find(|signal| &signal.name == name) {
                    Some(written) => match ran {
                        Some(ran) if ran.has_parameters_named_as(written) => Some(ran),
                        _ => Some(written),
                    },
                    None => ran,
                }
                .cloned()
            })
            .collect();
        signals.extend(
            class
                .signals
                .iter()
                .filter(|signal| !class.signal_names.contains(&signal.name))
                .cloned(),
        );
        signals
    }

    /// The properties the classes of the files at `files` exported, the
    /// first file's first and one to a name, as a class has what it
    /// exported and what it inherits. Each file comes with what its source
    /// writes, as `collect_members` takes it.
    pub fn collect_properties<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, Source<'a>)>,
    ) -> Vec<Property> {
        let mut properties: Vec<Property> = Vec::new();
        for (path, source) in files {
            for member in self.merge_file_members(path, source) {
                if let Member::Property(property) = member
                    && !properties.iter().any(|kept| kept.name == property.name)
                {
                    properties.push(property);
                }
            }
        }
        properties
    }

    /// What the classes of the files at `files` have the editor show: each
    /// class's own category and then what it declared, the first file's
    /// first, as GDScript lists a script's members before the ones it
    /// inherits. A property is listed once, the nearest class's, while a
    /// heading belongs to the class that wrote it. Each file comes with what
    /// its source writes, which answers for it until it has run. Once it
    /// has, what the file declared as it ran answers while the source is the
    /// one it ran; a source changed since answers in the order it writes,
    /// taking what the run declared for an export it does not write out, and
    /// keeping what the run declared through a name no `export` call spelled.
    pub fn collect_members<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, Source<'a>)>,
    ) -> Vec<Member> {
        let mut members: Vec<Member> = Vec::new();
        for (path, source) in files {
            if is_editor_build() {
                members.push(Member::Heading(make_category(path)));
            }
            for member in self.merge_file_members(path, source) {
                let listed = member.property().is_some_and(|property| {
                    members
                        .iter()
                        .filter_map(Member::property)
                        .any(|kept| kept.name == property.name)
                });
                if !listed {
                    members.push(member);
                }
            }
        }
        members
    }

    // What the class of the file at `path` declared for the editor, as
    // `collect_members` answers each file.
    fn merge_file_members(&self, path: &str, source: Source) -> Vec<Member> {
        let Some(class) = self.classes.get(path) else {
            return source.exports.iter().filter_map(Export::member).collect();
        };
        if class.digest == source.digest {
            return class.members.clone();
        }
        let mut members: Vec<Member> = Vec::new();
        for export in source.exports {
            let member = match export {
                Export::Member(member) => Some(member.clone()),
                Export::Name { name, bare } => class
                    .property_by_name(name)
                    .or(bare.as_ref())
                    .cloned()
                    .map(Member::Property),
            };
            members.extend(member);
        }
        members.extend(
            class
                .members
                .iter()
                .filter(|member| class.is_exported_unwritten(member))
                .cloned(),
        );
        members
    }

    /// The methods the class of the file at `path` has of its own, as last
    /// read; none for a file that has not run.
    pub fn methods_by_path(&self, path: &str) -> &[String] {
        self.classes
            .get(path)
            .map_or(&[], |class| class.methods.as_slice())
    }

    /// Whether the class of the file at `path` has a method of that name of
    /// its own.
    pub fn has_method(&self, path: &str, name: &str) -> bool {
        self.methods_by_path(path)
            .iter()
            .any(|method| method == name)
    }

    /// Takes what the class of the file at `path` has, now that the file has
    /// run, in place of what it had before.
    pub fn record_class(&mut self, path: &str, class: Class) {
        self.classes.insert(Arc::from(path), Arc::new(class));
        self.awaiting_names = OnceLock::new();
    }

    /// Takes `methods` as what the class of the file at `path` has now, which
    /// Ruby may change after the file has run.
    pub fn record_methods(&mut self, path: &str, methods: Vec<String>) {
        if let Some(class) = self.classes.get_mut(path) {
            Arc::make_mut(class).methods = methods;
        }
    }
}

// Whether this is a build of the editor, which is what a category is for:
// GDScript's are compiled out of a game's build, so a game's properties
// carry none either. The build never changes, so the engine is asked once.
fn is_editor_build() -> bool {
    static EDITOR: OnceLock<bool> = OnceLock::new();
    *EDITOR.get_or_init(|| Os::singleton().has_feature("editor"))
}

// The category the editor heads a class's own members with, as it heads a
// GDScript's with the script it is written in: the file's name, read with
// the path it is at.
fn make_category(path: &str) -> Heading {
    Heading::Category {
        name: path.rsplit('/').next().unwrap_or(path).to_owned(),
        path: path.to_owned(),
    }
}

// What the game's realm has published. A mod's realm will publish its own,
// keyed by the mod as its bookkeeping is.
static SNAPSHOT: LazyLock<RwLock<Arc<Snapshot>>> = LazyLock::new(RwLock::default);

/// The snapshot published last, for answering Godot without entering a realm.
pub fn latest() -> Arc<Snapshot> {
    Arc::clone(&SNAPSHOT.read().unwrap_or_else(PoisonError::into_inner))
}

/// Publishes `snapshot` for every thread to read from.
pub fn publish(snapshot: Arc<Snapshot>) {
    *SNAPSHOT.write().unwrap_or_else(PoisonError::into_inner) = snapshot;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rung_signal() -> Signal {
        Signal {
            name: "rung".to_owned(),
            parameters: vec![Parameter::new("times".to_owned(), VariantType::NIL)],
        }
    }

    // A property as a class exported it; its value is the bytes the engine
    // would write, which no test of the snapshot alone can ask it for.
    fn tone() -> Property {
        Property {
            name: "tone".to_owned(),
            kind: VariantType::FLOAT,
            hint: PropertyHint::NONE,
            hint_string: String::new(),
            usage: PropertyUsageFlags::DEFAULT,
            default: DeclaredValue::Bytes(Vec::new()),
            default_text: "0.0".to_owned(),
        }
    }

    fn bell() -> Class {
        Class {
            signals: vec![rung_signal()],
            members: vec![Member::Property(tone())],
            methods: vec!["ring".to_owned()],
            ..Class::default()
        }
    }

    // @behavior RS-072
    #[test]
    fn a_file_that_ran_without_declaring_anything_answers_none_of_what_its_source_writes() {
        let mut snapshot = Snapshot::default();
        let written = [rung_signal()];
        let names = ["rung".to_owned()];
        let source = Source {
            signals: &written,
            rpcs: &[],
            signal_names: &names,
            exports: &[],
            digest: 0,
        };

        snapshot.record_class("res://bell.rb", Class::default());

        assert!(
            snapshot
                .collect_signals([("res://bell.rb", source)])
                .is_empty()
        );
        assert_eq!(
            snapshot.collect_signals([("res://lamp.rb", source)]),
            [rung_signal()]
        );
    }

    // @behavior RD-001
    #[test]
    fn a_class_answers_the_signals_its_file_declared() {
        let mut snapshot = Snapshot::default();

        snapshot.record_class("res://bell.rb", bell());

        assert_eq!(snapshot.signals_by_path("res://bell.rb"), [rung_signal()]);
    }

    // @behavior RD-011
    #[test]
    fn a_class_answers_the_properties_its_file_exported() {
        let mut snapshot = Snapshot::default();

        snapshot.record_class("res://bell.rb", bell());

        assert_eq!(
            snapshot
                .properties_by_path("res://bell.rb")
                .collect::<Vec<_>>(),
            [&tone()]
        );
    }

    // @behavior RD-005
    #[test]
    fn a_class_answers_the_methods_it_defines() {
        let mut snapshot = Snapshot::default();

        snapshot.record_class("res://bell.rb", bell());

        assert!(snapshot.has_method("res://bell.rb", "ring"));
    }
}
