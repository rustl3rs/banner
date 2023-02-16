use std::{error::Error, fmt::Display};

use tracing::debug;
use tree_sitter::Node;
use tree_sitter_traversal::{traverse, Order};

use crate::ast::AST;

#[derive(Debug)]
pub struct ParseError {
    description: String,
    ast: AST,
}

impl ParseError {
    pub fn new(description: String, ast: AST) -> Self {
        Self { description, ast }
    }

    pub fn description(&self) -> &str {
        self.description.as_ref()
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // debug print the ast.
        self.ast.print();

        let cursor = traverse(self.ast.tree.walk(), Order::Pre);
        let mut prev: Option<Node> = None;
        for node in cursor {
            match node.kind() {
                "ERROR" => {
                    match prev {
                        Some(p) => {
                            if p.end_position() <= node.end_position() {
                                continue;
                            }
                            prev = None;
                        }
                        None => prev = Some(node),
                    }

                    // TODO: This needs to work better. At least it's a start.
                    write!(
                        f,
                        "Error Starts here ==>> {} <<=== and ends here!\n",
                        &self.ast.code[node.byte_range()]
                    )?
                }
                "source_file" => (),
                _ => (),
            }
        }

        Ok(())
    }
}

impl Error for ParseError {}
