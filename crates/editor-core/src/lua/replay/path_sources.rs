//! Conservative executed-path history, not a minimal control-dependency or edit proof.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    If,
    ElseIf,
    While,
    Repeat,
    NumericFor,
    And,
    Or,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionOutcome {
    Truthiness {
        truthy: bool,
    },
    NumericRange {
        current: i64,
        end: i64,
        step: i64,
        entered: bool,
    },
    IntegerRangeOverflow {
        current: i64,
        step: i64,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathDecision {
    /// Before the next API/loader event. Array order also orders decisions within it.
    pub at_order: usize,
    pub site: WriteSite,
    pub kind: DecisionKind,
    pub inputs: Vec<u32>,
    pub outcome: DecisionOutcome,
}
impl Engine<'_> {
    pub(super) fn path_decision(
        &mut self,
        doc: usize,
        node: Node<'_>,
        kind: DecisionKind,
        inputs: Vec<u32>,
        outcome: DecisionOutcome,
    ) -> Result<()> {
        if self.path_decisions.len() >= 8192
            || self.source_edges.saturating_add(inputs.len()) > 32768
        {
            return self.budget("Executed path history exceeds its node/edge budget");
        }
        let decision = PathDecision {
            at_order: self.order,
            site: WriteSite {
                document: doc,
                span: node.byte_range(),
                line: node.start_position().row + 1,
            },
            kind,
            inputs,
            outcome,
        };
        self.source_cost(&decision)?;
        self.source_edges += decision.inputs.len();
        self.path_decisions.push(decision);
        Ok(())
    }
    pub(super) fn truth_decision(
        &mut self,
        doc: usize,
        node: Node<'_>,
        kind: DecisionKind,
        value: &Value,
    ) -> Result<()> {
        self.path_decision(
            doc,
            node,
            kind,
            value.origin.into_iter().collect(),
            DecisionOutcome::Truthiness {
                truthy: value.is_truthy(),
            },
        )
    }
}
