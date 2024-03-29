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
        tracing::debug!("{parse_tree:#?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
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
        println!("{parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    #[test]
    fn can_parse_tasks_definition() {
        // TODO: remove the space from the raw string once https://github.com/rust-lang/rust-clippy/issues/11737
        // is resolved as fixed.

        let input = r##"task test(image: rust, execute: "") {r#" "#}"##;
        check(input);

        let input = r##"task test(image: rust:latest, execute: "") {r#" "#}"##;
        check(input);

        let input = r##"task test(image: ${rust_build_image}, execute: "") {r#" "#}"##;
        check(input);

        let input = r##"task test(image: ${rust_build_image}, execute: r#"bash -e -c"#) {r#" "#}"##;
        check(input);
    }
}

#[cfg(test)]
mod mount_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = BannerParser::parse(Rule::mount, input);
        println!("{parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    #[test]
    fn can_parse_mount_definition() {
        let input = r#""/tmp/source-code" => "/source-code","#;
        check(input);

        let input = r#"pipeline.job.task.src => "/source-code","#;
        check(input);

        let input = r#"${src} => "/source-code","#;
        check(input);
    }
}

#[cfg(test)]
mod image_spec_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = BannerParser::parse(Rule::image_specification, input);
        println!("INPUT: {input}\nTREE: {parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    #[test]
    fn can_parse() {
        let input = r##"name=rust,"##;
        check(input);

        let input = r#"name=rust,mount=["/tmp/source-code" => "/source-code",],","#;
        check(input);

        let input = r#"name=rust,mount=[${src} => "/source-code",],","#;
        check(input);

        let input =
            r#"name=rust,mount=[pipe-line.job.task.src => "/source-code",],env[ENV_VAR="value"]","#;
        check(input);
    }
}

#[cfg(test)]
mod let_statement_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = BannerParser::parse(Rule::let_statement, input);
        println!("INPUT: {input}\nTREE: {parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    #[test]
    fn can_parse() {
        let input = r##"let image1 = Image{name=rust}"##;
        check(input);

        let input =
            r#"let image1 = Image{name=rust,mount=["/tmp/source-code" => "/source-code",]}","#;
        check(input);

        let input = r#"let image1 = Image{name=rust,mount=[${src} => "/source-code",]}"#;
        check(input);

        let input = r#"let image1 = Image{name=rust,env=[ENV_VAR = "value",]}"#;
        check(input);

        let input = r#"let image1 = Image{name=rust,mount=[pipeline.job.task.src => "/source-code",],env=[ENV_VAR="value",]}"#;
        check(input);
    }
}

#[cfg(test)]
mod env_var_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = BannerParser::parse(Rule::env_var, input);
        println!("INPUT: {input}\nTREE: {parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    #[test]
    fn can_parse() {
        let input = r#"ENV_VAR="value","#;
        check(input);
    }
}

#[cfg(test)]
mod string_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = BannerParser::parse(Rule::string_literal, input);
        println!("INPUT: {input}\nTREE: {parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    #[test]
    fn can_parse() {
        let input = r#""this is a test""#;
        check(input);

        let input = r#""try something /\/ new""#;
        check(input);

        let input = r#""//""#;
        check(input);

        let input = r##"r#"//"#"##;
        check(input);
    }
}

#[cfg(test)]
mod pragma_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = BannerParser::parse(Rule::pragma, input);
        println!("INPUT: {input}\nTREE: {parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    #[test]
    fn can_parse_singleline_pragma() {
        let input = r##"#pragma test assert(success);"##;
        check(input);
    }

    #[test]
    fn can_parse_mulitline_pragma() {
        let input = r##"#pragma test
                assert(success);
#pragma end;
            "##;
        check(input);
    }
}
