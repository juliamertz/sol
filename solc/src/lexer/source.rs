use std::fmt::Debug;
use std::sync::Arc;

use miette::{NamedSource, SourceCode};

/// Cheaply clonable wrapper of `miette::NamedSource`
#[derive(Clone)]
pub struct SourceInfo(Arc<NamedSource<String>>);

impl SourceInfo {
    pub fn new(name: impl AsRef<str>, source: String) -> Self {
        Self(Arc::new(NamedSource::new(name, source)))
    }
}

impl SourceCode for SourceInfo {
    fn read_span<'a>(
        &'a self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        self.0
            .read_span(span, context_lines_before, context_lines_after)
    }
}

impl Debug for SourceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(self.0.name()).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    offset: usize,
    length: usize,
}

impl Span {
    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn enclosing_to(&self, other: &Self) -> Self {
        let len = other.offset - self.offset + other.length;
        Span::from((self.offset, len))
    }
}

impl From<(usize, usize)> for Span {
    fn from((offset, length): (usize, usize)) -> Self {
        Self { offset, length }
    }
}

impl From<Span> for miette::SourceSpan {
    fn from(val: Span) -> Self {
        (val.offset, val.length).into()
    }
}
