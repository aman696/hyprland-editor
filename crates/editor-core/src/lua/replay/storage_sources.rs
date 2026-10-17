//! Storage provenance is model evidence, never a source-edit or filesystem capability.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteKind {
    LocalDeclaration,
    Assignment,
    FunctionDeclaration,
    TableConstructor,
    Parameter,
    NumericFor,
    Rawset,
    ModeledIntrinsic,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteSite {
    pub document: usize,
    pub span: Range<usize>,
    pub line: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum StorageKey {
    Bool(bool),
    Int(i64),
    FloatBits(String),
    Bytes(Vec<u8>),
    Table(usize),
    Closure(usize),
}
impl From<&Key> for StorageKey {
    fn from(key: &Key) -> Self {
        match key {
            Key::Bool(v) => Self::Bool(*v),
            Key::Int(v) => Self::Int(*v),
            Key::Float(v) => Self::FloatBits(format!("{v:016x}")),
            Key::Bytes(v) => Self::Bytes(v.to_vec()),
            Key::Table(v) => Self::Table(*v),
            Key::Closure(v) => Self::Closure(*v),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoragePlace {
    Cell { identity: usize },
    Table { identity: usize, key: StorageKey },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteSource {
    /// Before the next API/loader order; the record index orders writes in that interval.
    pub at_order: usize,
    pub kind: WriteKind,
    pub site: Option<WriteSite>,
    pub place: StoragePlace,
    pub previous: Option<u32>,
    pub value_expression: Option<u32>,
    pub value_write: Option<u32>,
    pub value_table: Option<usize>,
    pub key_expression: Option<u32>,
    pub receiver_expression: Option<u32>,
    pub receiver_write: Option<u32>,
    /// Nil removes a table slot; a nil local remains a cell assignment.
    pub deletes_slot: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeySource {
    /// First insertion of the current present slot, not a traversal-order certificate.
    pub introduced_by: Option<u32>,
    pub latest_write: Option<u32>,
}
pub(super) struct WriteContext {
    pub(super) kind: WriteKind,
    pub(super) site: Option<WriteSite>,
    pub(super) key_expression: Option<u32>,
    pub(super) receiver_expression: Option<u32>,
    pub(super) receiver_write: Option<u32>,
}
impl WriteContext {
    pub(super) fn intrinsic() -> Self {
        Self {
            kind: WriteKind::ModeledIntrinsic,
            site: None,
            key_expression: None,
            receiver_expression: None,
            receiver_write: None,
        }
    }
    pub(super) fn at(doc: usize, node: Node<'_>, kind: WriteKind) -> Self {
        Self {
            kind,
            site: Some(WriteSite {
                document: doc,
                span: node.byte_range(),
                line: node.start_position().row + 1,
            }),
            ..Self::intrinsic()
        }
    }
}
pub(super) struct Address {
    pub(super) place: Place,
    pub(super) key_expression: Option<u32>,
    pub(super) receiver_expression: Option<u32>,
    pub(super) receiver_write: Option<u32>,
}
impl Address {
    pub(super) fn cell(id: usize) -> Self {
        Self {
            place: Place::Cell(id),
            key_expression: None,
            receiver_expression: None,
            receiver_write: None,
        }
    }
}
impl Engine<'_> {
    pub(super) fn write_source(
        &mut self,
        place: StoragePlace,
        previous: Option<u32>,
        value: &Value,
        context: WriteContext,
    ) -> Result<Option<u32>> {
        if !self.track_writes {
            return Ok(None);
        }
        let source = WriteSource {
            at_order: self.order,
            kind: context.kind,
            site: context.site,
            deletes_slot: matches!(place, StoragePlace::Table { .. }) && value.is_nil(),
            place,
            previous,
            value_expression: value.origin,
            value_write: value.storage_write,
            value_table: if let Kind::Table(id) = value.kind {
                Some(id)
            } else {
                None
            },
            key_expression: context.key_expression,
            receiver_expression: context.receiver_expression,
            receiver_write: context.receiver_write,
        };
        let edges = [
            source.previous,
            source.value_expression,
            source.value_write,
            source.key_expression,
            source.receiver_expression,
            source.receiver_write,
        ]
        .into_iter()
        .flatten()
        .count();
        if self.storage_writes.len() >= 8192 || self.source_edges.saturating_add(edges) > 32768 {
            return self.budget("Storage write provenance exceeds its node/edge budget");
        }
        self.source_cost(&source)?;
        self.source_edges += edges;
        let id = self.storage_writes.len() as u32;
        self.storage_writes.push(source);
        Ok(Some(id))
    }
    pub(super) fn cell_at(&mut self, mut value: Value, context: WriteContext) -> Result<usize> {
        if self.cells.len() >= 16_384 {
            return self.budget("Static replay cell limit exceeded");
        }
        let id = self.cells.len();
        value.storage_write =
            self.write_source(StoragePlace::Cell { identity: id }, None, &value, context)?;
        self.cells.push(value);
        Ok(id)
    }
    pub(super) fn cell_set(
        &mut self,
        id: usize,
        mut value: Value,
        context: WriteContext,
    ) -> Result<()> {
        value.storage_write = self.write_source(
            StoragePlace::Cell { identity: id },
            self.cells[id].storage_write,
            &value,
            context,
        )?;
        self.cells[id] = value;
        Ok(())
    }
}
