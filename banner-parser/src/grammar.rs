extern crate pest;

#[derive(Parser)]
#[grammar = "grammar/banner.pest"]
pub struct BannerParser;

#[cfg(test)]
mod image_definition_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = BannerParser::parse(Rule::image_definition, input);
        println!("{:#?}", parse_tree);
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                println!("{:#?}", e);
                assert!(false)
            }
        }
    }

    #[test]
    fn can_parse_tasks_image_definition() {
        let input = r#"alpine"#;
        check(input);

        let input = r#"alpine:latest"#;
        check(input);

        let input = r#"alpine:3.12.0"#;
        check(input);

        let input = r#"alpine:3.12.0-rc1"#;
        check(input);

        let input = r#"alpine:3.12.0-rc1+build1"#;
        check(input);

        let input = r#"alpine:3.12.0-rc1+build1.2"#;
        check(input);

        let input = r#"alpine:3.12.0-rc1+build1.2.3"#;
        check(input);

        let input = r#"alpine:@sha1234"#;
        check(input);

        let input = r#"${defined_image}"#;
        check(input);
    }
}

#[cfg(test)]
mod task_definition_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = BannerParser::parse(Rule::task_definition, input);
        println!("{:?}", parse_tree);
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                println!("{:#?}", e);
                assert!(false)
            }
        }
    }

    #[test]
    fn can_parse_tasks_definition() {
        let input = r##"task test(image: rust, execute: "") {r#""#}"##;
        check(input);

        let input = r##"task test(image: rust:latest, execute: "") {r#""#}"##;
        check(input);

        let input = r##"task test(image: ${rust_build_image}, execute: "") {r#""#}"##;
        check(input);
    }
}
