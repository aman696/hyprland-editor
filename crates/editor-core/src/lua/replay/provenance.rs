//! Bounded expression evidence, independent of modeled Lua value identity.
//! Evaluation edges are conservative dependencies, never edit/ownership authority.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionKind {
    Literal,
    Read,
    Table,
    Function,
    Operation,
    Call,
    Varargs,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionSource {
    /// Index into the immutable Program.documents array; its hash/load name stay pinned there.
    pub document: usize,
    pub span: Range<usize>,
    pub line: usize,
    pub kind: ExpressionKind,
    pub result: usize,
    /// Prior value forwarded by a read, selected result or return. None can mean an
    /// intrinsic/default/generated value; it never establishes literal ownership.
    pub selected: Option<u32>,
    /// Executed child expressions, including selector/control/evaluation effects.
    /// These are NOT all interchangeable literal edit targets or minimal dataflow.
    pub evaluated: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueSource {
    pub expression: Option<u32>,
    /// Values only, in the same order as the paired Observed::Table rows. Cycles
    /// and non-table values have no rows; the Observed shape distinguishes them.
    pub rows: Vec<ValueSource>,
}

impl Engine<'_> {
    fn source_cost(&mut self, value: &impl Serialize) -> Result<()> {
        struct Count(usize);
        impl std::io::Write for Count {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 = self.0.saturating_add(bytes.len());
                if self.0 > 2 * MAX_DOCUMENT_BYTES {
                    return Err(std::io::Error::other("source evidence output limit"));
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut count = Count(self.output);
        serde_json::to_writer(&mut count, value).map_err(|_| {
            Failure::Budget("Expression source evidence exceeds output budget".into())
        })?;
        self.output = count.0;
        Ok(())
    }
    pub(super) fn trace_expression(
        &mut self,
        doc: usize,
        node: Node<'_>,
        values: &mut [Value],
        evaluated: Vec<u32>,
    ) -> Result<()> {
        let kind = match node.kind() {
            "nil" | "true" | "false" | "number" | "string" => ExpressionKind::Literal,
            "identifier" | "dot_index_expression" | "bracket_index_expression" => {
                ExpressionKind::Read
            }
            "table_constructor" => ExpressionKind::Table,
            "function_definition" => ExpressionKind::Function,
            "function_call" => ExpressionKind::Call,
            "vararg_expression" => ExpressionKind::Varargs,
            _ => ExpressionKind::Operation,
        };
        for (result, value) in values.iter_mut().enumerate() {
            let edges = evaluated.len() + usize::from(value.origin.is_some());
            if self.expression_sources.len() >= 8192
                || self.source_edges.saturating_add(edges) > 32768
            {
                return self.budget("Expression provenance exceeds its node/edge budget");
            }
            let source = ExpressionSource {
                document: doc,
                span: node.byte_range(),
                line: node.start_position().row + 1,
                kind: kind.clone(),
                result,
                selected: value.origin,
                evaluated: evaluated.clone(),
            };
            self.source_cost(&source)?;
            self.source_edges += edges;
            let id = self.expression_sources.len() as u32;
            self.expression_sources.push(source);
            value.origin = Some(id);
            if let Some(parent) = self.source_frames.last_mut() {
                parent.push(id);
            }
        }
        Ok(())
    }
    fn value_source(
        &self,
        value: &Value,
        seen: &mut BTreeSet<usize>,
        depth: usize,
    ) -> Result<ValueSource> {
        if depth > 64 {
            return self.budget("Value provenance depth exceeds 64");
        }
        let mut result = ValueSource {
            expression: value.origin,
            rows: vec![],
        };
        if let Kind::Table(id) = value.kind {
            if seen.insert(id) {
                for value in self.tables[id].values.values() {
                    result.rows.push(self.value_source(value, seen, depth + 1)?);
                }
                seen.remove(&id);
            }
        }
        Ok(result)
    }
    pub(super) fn observed_sources(
        &mut self,
        values: &[Value],
    ) -> Result<(Vec<Observed>, Vec<ValueSource>)> {
        // The first pass already bounds the snapshot's depth, node count and byte
        // retention. This second pass cannot invoke code or mutate table storage.
        let observed = self.observed(values)?;
        let sources = values
            .iter()
            .map(|v| self.value_source(v, &mut BTreeSet::new(), 0))
            .collect::<Result<Vec<_>>>()?;
        self.source_cost(&sources)?;
        Ok((observed, sources))
    }
}
