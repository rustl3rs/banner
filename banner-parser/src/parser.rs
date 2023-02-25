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
    trace!("code = {:#?}", &code);
    let mut parse_tree = BannerParser::parse(Rule::pipeline_definition, &code)?;
    trace!("parse tree = {:#?}", parse_tree);
    let syntax_tree: Pipeline = Pipeline::from_pest(&mut parse_tree).expect("infallible");
    trace!("syntax tree = {:#?}", syntax_tree);
    Ok(syntax_tree)
}

impl From<Task> for TaskDefinition {
    fn from(task: Task) -> Self {
        let tags = task
            .tags
            .into_iter()
            .map(|t| Tag::new(t.key, t.value))
            .collect();
        let image = Image::new(task.image, None);
        let mut command: Vec<String> = task
            .command
            .as_str()
            .split_whitespace()
            .map(|s| s.into())
            .collect();
        command.push(task.script);
        let td = Self::new(tags, image, command, vec![], vec![]);
        td
    }
}

#[cfg(test)]
mod tests {
    use expect_test::{expect, Expect};
    use tracing_test::traced_test;

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
        check(
            &code,
            expect![[r#"
                Pipeline {
                    imports: [],
                    tasks: [],
                    eoi: EOI,
                }"#]],
        )
    }

    // #[traced_test]
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

        check(
            code,
            expect![[r#"
                Pipeline {
                    imports: [],
                    tasks: [
                        Task {
                            tags: [],
                            name: "unit-test",
                            image: "rustl3rs/banner-rust-build",
                            command: RawString(
                                "/bin/bash -c",
                            ),
                            script: "bash\n            echo testing, testing, 1, 2, 3!",
                        },
                    ],
                    eoi: EOI,
                }"#]],
        )
    }

    // #[traced_test]
    #[test]
    fn can_parse_task_with_tag_attribute() {
        let code = r#######"
        [tag: banner.io/owner=me]
        [tag: banner.io/company=rustl3rs]
        task unit-test(image: rustl3rs/banner-rust-build, execute: r#"/bin/bash -c"#) {
            r#####"bash
            echo testing, testing, 1, 2, 3!
            "#####
        }
        "#######;

        check(
            code,
            expect![[r#"
                Pipeline {
                    imports: [],
                    tasks: [
                        Task {
                            tags: [
                                Tag {
                                    key: "banner.io/owner",
                                    value: "me",
                                },
                                Tag {
                                    key: "banner.io/company",
                                    value: "rustl3rs",
                                },
                            ],
                            name: "unit-test",
                            image: "rustl3rs/banner-rust-build",
                            command: RawString(
                                "/bin/bash -c",
                            ),
                            script: "bash\n            echo testing, testing, 1, 2, 3!",
                        },
                    ],
                    eoi: EOI,
                }"#]],
        )
    }

    #[traced_test]
    #[test]
    fn can_parse_uri() {
        let code = r#######"
        import file://./single_task.ban
        "#######;

        check(code, expect![[r#"
            Pipeline {
                imports: [
                    Import {
                        uri: "file://./single_task.ban",
                    },
                ],
                tasks: [],
                eoi: EOI,
            }"#]])
    }
}
