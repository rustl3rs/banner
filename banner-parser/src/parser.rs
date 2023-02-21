use banner_engine::{Image, Tag, TaskDefinition};
use from_pest::FromPest;
use pest::Parser;
use std::error::Error;
use tracing::trace;

use crate::{
    ast::{self, Pipeline, Task},
    grammar::{BannerParser, Rule},
};

extern crate from_pest;
extern crate pest;

pub fn validate_pipeline(code: String) -> Result<Pipeline, Box<dyn Error + Send + Sync>> {
    let mut parse_tree = BannerParser::parse(Rule::pipeline_definition, &code)?;
    trace!("parse tree = {:#?}", parse_tree);
    let syntax_tree: Pipeline = Pipeline::from_pest(&mut parse_tree).expect("infallible");
    trace!("syntax tree = {:#?}", syntax_tree);
    Ok(syntax_tree)
}

impl From<Task> for TaskDefinition {
    fn from(value: Task) -> Self {
        let pipeline_tag = Tag::new(
            String::from("banner.io/pipeline"),
            String::from("test-pipeline"),
        );
        let job_tag = Tag::new(String::from("banner.io/job"), String::from("test-job"));
        let task_tag = Tag::new(String::from("banner.io/task"), String::from("test-task"));
        let tags = vec![pipeline_tag, job_tag, task_tag];
        let image = Image::new(value.image_identifier.image, None);
        let mut command = match value.execute_command {
            ast::StringLiteral::Raw(string) => {
                let c: Vec<String> = string.content.split(r#" "#).map(|s| s.into()).collect();
                c
            }
            ast::StringLiteral::String(string) => {
                let c: Vec<String> = string.content.split(r#"\s"#).map(|s| s.into()).collect();
                c
            }
        };
        command.push(value.script.content);
        let td = Self::new(tags, image, command, vec![], vec![]);
        td
    }
}
#[cfg(test)]
mod tests {
    use expect_test::{expect, Expect};

    use super::*;

    fn check(code: &str, expect: Expect) {
        match validate_pipeline(code.to_owned()) {
            Ok(ast) => {
                let actual = format!("{:#?}", ast);
                expect.assert_eq(&actual);
            }
            Err(e) => {
                println!("{:#?}", e);
                assert!(false)
            }
        }
    }

    #[test]
    fn can_parse_comment() {
        let code = String::from("//a test");
        check(&code, expect![[r#"
            Pipeline {
                tasks: [],
                eoi: EOI,
            }"#]])
    }

    #[test]
    fn can_parse_task_with_comment() {
        let code = r#######"
        // this task does the unit testing of the app
        task unit-test(image: rustl3rs/banner-rust-build, execute: r#"/bin/bash -c"#) {
            r#####"bash
            echo testing, testing, 1, 2, 3!
            "#####
        }
        "#######;

        check(code, expect![[r#"
            Pipeline {
                tasks: [
                    Task {
                        name: Identifier {
                            image: "unit-test",
                        },
                        image_identifier: ImageIdentifier {
                            image: "rustl3rs/banner-rust-build",
                        },
                        execute_command: Raw(
                            RawString {
                                content: "/bin/bash -c",
                            },
                        ),
                        script: RawString {
                            content: "bash\n            echo testing, testing, 1, 2, 3!",
                        },
                    },
                ],
                eoi: EOI,
            }"#]])
    }
}
