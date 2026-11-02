//! Purpose: The record form: the values typed for one record, and the field the cursor is on.

use super::{Proxy, Record, Ttl, Zone};

/// The order the Tab key walks, for a new record.
const NEW_FIELDS: [FormField; 4] = [
    FormField::Name,
    FormField::Content,
    FormField::Ttl,
    FormField::Proxy,
];
/// An existing record keeps its name: only the content, TTL and proxy change.
const EXISTING_FIELDS: [FormField; 3] = [FormField::Content, FormField::Ttl, FormField::Proxy];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormField {
    Name,
    Content,
    Ttl,
    Proxy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A host being added: the form creates an `A` record.
    New,
    Existing(Record),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordForm {
    pub(crate) zone: Zone,
    pub(crate) target: Target,
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) ttl: String,
    pub(crate) proxy: Proxy,
    pub(crate) field: FormField,
    /// Why a value is empty or unknown, shown under the form.
    pub(crate) note: String,
}

impl RecordForm {
    /// The form for an existing record, with its current values filled in.
    pub(crate) fn edit(zone: Zone, record: Record) -> Self {
        Self {
            zone,
            name: record.name.clone(),
            content: record.content.clone(),
            ttl: ttl_text(record.ttl),
            proxy: record.proxy,
            target: Target::Existing(record),
            field: FormField::Content,
            note: String::new(),
        }
    }

    /// The form for a new `A` record, with the name and address already known.
    pub(crate) fn new_record(zone: Zone, name: String, content: String, note: String) -> Self {
        Self {
            zone,
            target: Target::New,
            name,
            content,
            ttl: ttl_text(Ttl::Automatic),
            proxy: Proxy::Off,
            field: FormField::Name,
            note,
        }
    }

    fn fields(&self) -> &'static [FormField] {
        match self.target {
            Target::New => &NEW_FIELDS,
            Target::Existing(_) => &EXISTING_FIELDS,
        }
    }

    /// Moves the cursor to the next field, or the previous one, wrapping round.
    pub(crate) fn step(&mut self, forward: bool) {
        let fields = self.fields();
        let Some(position) = fields.iter().position(|field| *field == self.field) else {
            return;
        };
        let count = fields.len();
        let next = if forward {
            (position + 1) % count
        } else {
            (position + count - 1) % count
        };
        if let Some(field) = fields.get(next) {
            self.field = *field;
        }
    }

    /// Types `ch` into the focused text field. The proxy field takes no text.
    pub(crate) fn type_char(&mut self, ch: char) {
        if let Some(text) = self.focused_text() {
            text.push(ch);
        }
    }

    pub(crate) fn backspace(&mut self) {
        if let Some(text) = self.focused_text() {
            text.pop();
        }
    }

    /// Space on the proxy field: the proxy switches, where the record type has one.
    pub(crate) fn toggle_proxy(&mut self) {
        if self.field == FormField::Proxy {
            self.proxy = self.proxy.toggled();
        }
    }

    fn focused_text(&mut self) -> Option<&mut String> {
        match self.field {
            FormField::Name => Some(&mut self.name),
            FormField::Content => Some(&mut self.content),
            FormField::Ttl => Some(&mut self.ttl),
            FormField::Proxy => None,
        }
    }
}

fn ttl_text(ttl: Ttl) -> String {
    match ttl {
        Ttl::Automatic => "1".to_owned(),
        Ttl::Seconds(seconds) => seconds.to_string(),
    }
}
