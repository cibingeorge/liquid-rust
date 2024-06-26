use std::io::Write;

use crate::error::{Result, ResultLiquidReplaceExt};
use crate::runtime::Renderable;
use crate::runtime::Runtime;

/// A raw template expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Text {
    text: String,
}

impl Text {
    /// Create a raw template expression.
    pub(crate) fn new<S: Into<String>>(text: S) -> Text {
        Text { text: text.into() }
    }
}

impl Renderable for Text {
    fn render_to(&self, writer: &mut dyn Write, _runtime: &dyn Runtime) -> Result<()> {
        write!(writer, "{}", &self.text).replace("Failed to render")?;
        Ok(())
    }

    fn is_blank(&self) -> bool {
        self.text.find(|c: char| !c.is_ascii_whitespace()).is_none()
    }

    fn is_text(&self) -> bool {
        true
    }
}
