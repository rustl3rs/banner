use crate::errors;

use errors::ParseError;
use std::{error::Error, fmt::Display};
use tracing::{debug, span, Level};
use tree_sitter::Tree;
use tree_sitter_traversal::{traverse, Order};

#[derive(Debug)]
pub struct AST {
    pub(crate) tree: Tree,
    pub(crate) code: String,
}

impl AST {
    pub fn new(tree: Tree, code: String) -> Self {
        Self { tree, code }
    }

    pub fn print(&self) {
        let span = span!(Level::DEBUG, "AST");
        let _enter = span.enter();

        let cursor = traverse(self.tree.walk(), Order::Pre);
        for node in cursor {
            debug!("{node:?}");
        }
    }
}

pub fn validate_pipeline(code: String) -> Result<AST, Box<dyn Error + Send + Sync>> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(tree_sitter_banner::language())?;
    let tree = parser.parse(&code, None).unwrap();

    if tree.root_node().has_error() {
        Err(Box::new(ParseError::new(
            String::from("Parsing Errors > 0"),
            AST::new(tree, code),
        )))
    } else {
        Ok(AST::new(tree, code))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn can_parse_task() {
        assert!(super::validate_pipeline(String::from("//a test")).is_ok());
        // assert!(false)
    }

    #[test]
    fn can_print_ast() {
        let ast = super::validate_pipeline(String::from("//a test")).unwrap();
        ast.print();
        assert!(true)
    }
}
