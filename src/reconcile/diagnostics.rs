use super::types::{Diagnostic, ReconcileError};

pub(crate) trait DiagnosticsModule {
    fn error(&mut self, diagnostics: Vec<Diagnostic>) -> ReconcileError;
}
