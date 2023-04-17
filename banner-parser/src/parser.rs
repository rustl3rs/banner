use from_pest::FromPest;
use pest::Parser;
use std::error::Error;
use tracing::trace;

use crate::{
    ast::Pipeline,
    grammar::{BannerParser, Rule},
};

extern crate from_pest;
extern crate pest;

pub fn parse_file(code: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    trace!("code = {:#?}", &code);
    let mut parse_tree = BannerParser::parse(Rule::pipeline_definition, &code)?;
    trace!("parse tree = {:#?}", parse_tree);
    let syntax_tree: Pipeline = match Pipeline::from_pest(&mut parse_tree) {
        Ok(tree) => tree,
        Err(e) => {
            trace!("ERROR = {:#?}", e);
            panic!("{:?}", e);
        }
    };
    trace!("syntax tree = {:#?}", syntax_tree);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use expect_test::{expect, Expect};
    use tracing_test::traced_test;

    use super::*;

    fn check(code: &str, expect: Expect) {
        match parse_file(code.to_owned()) {
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

    #[traced_test]
    #[test]
    fn can_parse_comment() {
        let code = String::from("// a test");
        check(&code, expect!["()"])
    }

    // #[traced_test]
    #[test]
    fn can_parse_task_with_comment() {
        let code = r#######"
        // this task does the unit testing of the app
        task unit-test(image: rustl3rs/banner-rust-build:latest, execute: r#"/bin/bash -c"#) {
            r#####"bash
            echo testing, testing, 1, 2, 3!
            "#####
        }
        "#######;

        check(code, expect!["()"])
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

        check(code, expect!["()"])
    }

    // #[traced_test]
    #[test]
    fn can_parse_uri() {
        let code = r#######"
        import file://./single_task.ban
        // import https://github.com/rustl3rs/banner/pipeline-assets/echo_task.ban
        "#######;

        check(code, expect!["()"])
    }

    #[traced_test]
    #[test]
    fn can_parse_banner_pipeline() {
        let code = fs::read_to_string("../pipeline-assets/banner-pipeline.ban")
            .expect("Should have been able to read the file");

        check(&code, expect!["()"])
    }
}

#[cfg(test)]
mod pipeline_from_ast_tests {
    use expect_test::{expect, Expect};
    use tracing_test::traced_test;

    use super::*;

    fn check(code: &str, expect: Expect) {
        let mut parse_tree = BannerParser::parse(Rule::pipeline_definition, &code).unwrap();
        match Pipeline::from_pest(&mut parse_tree) {
            Ok(tree) => {
                let actual = format!("{:#?}", tree);
                expect.assert_eq(&actual);
            }
            Err(e) => {
                trace!("ERROR = {:#?}", e);
                panic!("{:?}", e);
            }
        }
    }

    #[test]
    fn test_syntax() {
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
            &code,
            expect![[r#"
            Pipeline {
                imports: [],
                images: [],
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
                jobs: [],
                pipelines: [],
                eoi: EOI,
            }"#]],
        )
    }

    #[test]
    fn can_parse_pipeline_with_job_and_task() {
        let code = r#######"
        task cowsay(image: kmcgivern/cowsay-alpine:latest, execute: r#""#) {r#""#}

        job build [
            cowsay,
        ]

        pipeline test [
            build,
        ]
        "#######;

        check(
            &code,
            expect![[r#"
                Pipeline {
                    imports: [],
                    images: [],
                    tasks: [
                        Task {
                            tags: [],
                            name: "cowsay",
                            image: "kmcgivern/cowsay-alpine:latest",
                            command: RawString(
                                "",
                            ),
                            script: "",
                        },
                    ],
                    jobs: [
                        JobSpecification {
                            name: "build",
                            tasks: [
                                "cowsay",
                            ],
                        },
                    ],
                    pipelines: [
                        PipelineSpecification {
                            name: "test",
                            jobs: [
                                "build",
                            ],
                        },
                    ],
                    eoi: EOI,
                }"#]],
        )
    }

    #[traced_test]
    #[test]
    fn can_parse_pipeline_with_job() {
        let code = r#######"
        job build []

        pipeline test [
            build,
        ]
        "#######;

        check(
            &code,
            expect![[r#"
                Pipeline {
                    imports: [],
                    images: [],
                    tasks: [],
                    jobs: [
                        JobSpecification {
                            name: "build",
                            tasks: [],
                        },
                    ],
                    pipelines: [
                        PipelineSpecification {
                            name: "test",
                            jobs: [
                                "build",
                            ],
                        },
                    ],
                    eoi: EOI,
                }"#]],
        )
    }
}
