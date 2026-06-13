use crate::core::context::Context;
use anyhow::Result;
use std::sync::Arc;

pub type PreRequestHook = Arc<dyn Fn(&mut Context) -> Result<()> + Send + Sync>;

#[allow(dead_code)]
pub struct Middleware {
    hooks: Vec<PreRequestHook>,
}

#[allow(dead_code)]
impl Middleware {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register(&mut self, hook: PreRequestHook) {
        self.hooks.push(hook);
    }

    pub fn execute(&self, ctx: &mut Context) -> Result<()> {
        for hook in &self.hooks {
            hook(ctx)?;
        }
        Ok(())
    }
}

#[allow(dead_code)]
impl Default for Middleware {
    fn default() -> Self {
        Self::new()
    }
}
