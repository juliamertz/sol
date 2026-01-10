use std::fmt::Debug;
use std::sync::Arc;

use miette::{NamedSource, SourceCode};

/// Cheaply clonable wrapper of `miette::NamedSource`
#[derive(Clone)]
pub struct SourceInfo {
    src: Arc<NamedSource<String>>,
}

impl SourceInfo {
    pub fn new(name: impl AsRef<str>, source: String) -> Self {
        Self {
            src: Arc::new(NamedSource::new(name, source)),
        }
    }
}

impl SourceCode for SourceInfo {
    fn read_span<'a>(
        &'a self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        self.src
            .read_span(span, context_lines_before, context_lines_after)
    }
}

impl Debug for SourceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(self.src.name()).finish()
    }
}
