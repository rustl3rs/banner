use banner_parser::ast;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pragma {
    pub context: String,
    pub src: String,
}

#[derive(Debug)]
pub struct PragmasBuilder {
    pub(crate) contexts: Vec<String>,
}

impl PragmasBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self { contexts: vec![] }
    }

    #[must_use]
    pub fn register_context(mut self, context: &str) -> Self {
        self.contexts.push(context.to_string());
        self
    }

    pub(crate) fn build_from(&self, pragmas: &[ast::Pragma]) -> Vec<Pragma> {
        pragmas
            .iter()
            .filter(|p| self.contexts.contains(&p.context))
            .map(|pragma| {
                let context = pragma.context.clone();
                let src = pragma.src.clone();
                Pragma { context, src }
            })
            .collect()
    }
}

impl Default for PragmasBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test_pragma_builder {
    use super::*;

    #[test]
    fn pragma_builder_registers_context() {
        let pb = PragmasBuilder::new().register_context("test");
        assert_eq!(pb.contexts, vec!["test".to_string()]);
    }
}
