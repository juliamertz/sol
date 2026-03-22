use std::fmt::Debug;
use std::sync::Arc;

#[derive(Clone)]
pub struct SourceInfo {
    name: Arc<str>,
    source: Arc<str>,
}

impl SourceInfo {
    pub fn new(name: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
        }
    }

    pub fn inner(&self) -> &str {
        &self.source
    }
}

impl std::fmt::Debug for SourceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceInfo")
            .field("name", &self.name)
            .field("source", &"<redacted>");
        Ok(())
    }
}

impl miette::SourceCode for SourceInfo {
    fn read_span<'a>(
        &'a self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        let inner_contents =
            self.inner()
                .read_span(span, context_lines_before, context_lines_after)?;
        let mut contents = miette::MietteSpanContents::new_named(
            self.name.to_string(),
            inner_contents.data(),
            *inner_contents.span(),
            inner_contents.line(),
            inner_contents.column(),
            inner_contents.line_count(),
        );
        contents = contents.with_language("sol");
        Ok(Box::new(contents))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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
        let length = other.offset - self.offset + other.length;
        Self {
            offset: self.offset,
            length,
        }
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
