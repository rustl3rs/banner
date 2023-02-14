use tree_sitter_traversal::{traverse, Order};

pub fn parse_task() {
    let code = r######"
    // this task does the unit testing of the app
    task unit-test(image: rustl3rs/banner-rust-build, execute: "/bin/bash -c") {
    r#####"bash
    # this is a bash comment  
    echo rustl3rs herd!
    # basically a no-op.
    # But a good start to our testing.
    "#####
    }
    
    // job []
    "######;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_banner::language())
        .expect("Error loading banner grammar");
    let tree = parser.parse(code, None).unwrap();
    let cursor = traverse(tree.walk(), Order::Pre).collect::<Vec<_>>();
    for node in cursor {
        println!("{:?}", node);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_parse_task() {
        super::parse_task();
        // assert!(false)
    }
}
