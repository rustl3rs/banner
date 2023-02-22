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
    fn from(value: Task) -> Self {
        let tags = value
            .tags
            .into_iter()
            .map(|t| Tag::new(t.key.content, t.value.content))
            .collect();
        let image = Image::new(value.image_identifier.image, None);
        let mut command = match value.execute_command {
            ast::StringLiteral::Raw(string) => {
                let c: Vec<String> = string
                    .content
                    .split_whitespace()
                    .map(|s| s.into())
                    .collect();
                c
            }
            ast::StringLiteral::String(string) => {
                let c: Vec<String> = string
                    .content
                    .split_whitespace()
                    .map(|s| s.into())
                    .collect();
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
                tasks: [
                    Task {
                        tags: [],
                        name: Identifier {
                            name: "unit-test",
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
            }"#]],
        )
    }

    #[traced_test]
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
                tasks: [
                    Task {
                        tags: [
                            Tag {
                                key: TagKey {
                                    content: "banner.io/owner",
                                },
                                value: TagValue {
                                    content: "me",
                                },
                            },
                            Tag {
                                key: TagKey {
                                    content: "banner.io/company",
                                },
                                value: TagValue {
                                    content: "rustl3rs",
                                },
                            },
                        ],
                        name: Identifier {
                            name: "unit-test",
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
            }"#]],
        )
    }
}
