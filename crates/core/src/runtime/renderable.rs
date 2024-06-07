use std::any::Any;
use std::fmt::Debug;
use std::io::Write;

use crate::error::Result;

use super::Runtime;

/// Any object (tag/block) that can be rendered by liquid must implement this trait.
pub trait Renderable: Send + Sync + Debug {
    /// Renders the Renderable instance given a Liquid runtime.
    fn render(&self, runtime: &dyn Runtime) -> Result<String> {
        let mut data = Vec::new();
        self.render_to(&mut data, runtime)?;
        Ok(String::from_utf8(data).expect("render only writes UTF-8"))
    }

    /// Renders the Renderable instance given a Liquid runtime.
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> Result<()>;


    /// Checks if a block can be blank or not.
    fn is_blank(&self) -> bool {
        false
    }

    /// Checks if a block is just Text or something else. We remove empty text in if,unless,case in some cases
    fn is_text(&self) -> bool {
        false
    }
}
