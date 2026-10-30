//! Captured legacy bytes and restricted projections, with no restored path access.
use hyprland_editor_core::{import, legacy_session, session, Draft, Setting, BASELINE};
#[derive(Debug)]
pub(super) struct Document {
    pub content: String,
    pub report: import::ImportReport,
    pub session: legacy_session::Report,
}
impl Document {
    pub fn new(content: String, catalogue: &[Setting]) -> Result<Self, String> {
        #[cfg(not(test))]
        let parsed = match super::worker::run(super::worker::Request::ParseLegacy {
            source: content.clone(),
        })? {
            super::worker::Response::LegacyParsed(report) => *report,
            _ => return Err("Unexpected legacy parser response".into()),
        };
        #[cfg(test)]
        let parsed =
            hyprland_editor_core::legacy::Document::new(content.clone(), catalogue)?.into_report();
        if parsed.session.schema_sha256 != legacy_session::schema_hash()
            || parsed.source_sha256 != hyprland_editor_core::lua::source_hash(&content)
            || parsed.session.source_sha256 != parsed.source_sha256
            || parsed.scalar.draft.version != BASELINE
            || parsed.session.sites.len() != parsed.session.draft.entries.len()
        {
            return Err("Legacy projection does not match the captured source".into());
        }
        parsed.scalar.draft.validate(catalogue)?;
        parsed.session.draft.validate(BASELINE)?;
        let mut report = parsed.scalar;
        report
            .skipped
            .retain(|line| !parsed.session.sites.iter().any(|site| site.line == *line));
        Ok(Self {
            content,
            report,
            session: parsed.session,
        })
    }
    pub fn edited_with_session(
        &self,
        draft: &Draft,
        session: &session::Draft,
    ) -> Result<String, String> {
        match super::worker::run(super::worker::Request::CopyLegacy {
            source: self.content.clone(),
            draft: draft.clone(),
            session: session.clone(),
        })? {
            super::worker::Response::Copy(source) => Ok(source),
            _ => Err("Unexpected legacy copy response".into()),
        }
    }
}
