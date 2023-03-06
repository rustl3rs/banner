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

pub fn validate_pipeline(code: String) -> Result<Pipeline, Box<dyn Error + Send + Sync>> {
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
    Ok(syntax_tree)
}

#[cfg(test)]
mod tests {
    use std::fs;

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

    #[traced_test]
    #[test]
    fn can_parse_comment() {
        let code = String::from("// a test");
        check(
            &code,
            expect![[r#"
                Pipeline {
                    imports: [],
                    images: [],
                    tasks: [],
                    jobs: None,
                    pipelines: None,
                    _eoi: EOI,
                }"#]],
        )
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

        check(
            code,
            expect![[r#"
                Pipeline {
                    imports: [],
                    images: [],
                    tasks: [
                        Task {
                            tags: [],
                            name: "unit-test",
                            image: "rustl3rs/banner-rust-build:latest",
                            command: RawString(
                                "/bin/bash -c",
                            ),
                            script: "bash\n            echo testing, testing, 1, 2, 3!",
                        },
                    ],
                    jobs: None,
                    pipelines: None,
                    _eoi: EOI,
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
                    jobs: None,
                    pipelines: None,
                    _eoi: EOI,
                }"#]],
        )
    }

    // #[traced_test]
    #[test]
    fn can_parse_uri() {
        let code = r#######"
        import file://./single_task.ban
        // import https://github.com/rustl3rs/banner/pipeline-assets/echo_task.ban
        "#######;

        check(
            code,
            expect![[r#"
                Pipeline {
                    imports: [
                        Import {
                            uri: "file://./single_task.ban",
                        },
                    ],
                    images: [],
                    tasks: [],
                    jobs: None,
                    pipelines: None,
                    _eoi: EOI,
                }"#]],
        )
    }

    #[traced_test]
    #[test]
    fn can_parse_banner_pipeline() {
        let code = fs::read_to_string("../pipeline-assets/banner-pipeline.ban")
            .expect("Should have been able to read the file");

        check(
            &code,
            expect![[r#"
                Pipeline {
                    imports: [
                        Import {
                            uri: "https://github.com/rustl3rs/banner/pipeline-assets/echo_task.ban",
                        },
                    ],
                    images: [
                        Images {
                            name: "rust_build_image",
                            image: Image {
                                name: "rust:latest",
                                mounts: [
                                    Mount {
                                        source: "src",
                                        destination: RawString(
                                            "/source-code",
                                        ),
                                    },
                                ],
                                envs: [
                                    EnviromentVariable {
                                        key: "RUSTLOG",
                                        value: RawString(
                                            "${log_level}",
                                        ),
                                    },
                                    EnviromentVariable {
                                        key: "RUST_BACKTRACE",
                                        value: RawString(
                                            "1",
                                        ),
                                    },
                                ],
                            },
                        },
                    ],
                    tasks: [
                        Task {
                            tags: [
                                Tag {
                                    key: "rustl3rs.com/owner",
                                    value: "pms1969",
                                },
                            ],
                            name: "test",
                            image: "${rust_build_image}",
                            command: RawString(
                                "bash -exo pipefail -c",
                            ),
                            script: "cd /source-code\n    \n    # would be good to capture code coverage stats here.\n    cargo test",
                        },
                    ],
                    jobs: Some(
                        JobSpecification {
                            name: "unit-test",
                            tasks: [
                                "cowsay,",
                                "test,",
                            ],
                        },
                    ),
                    pipelines: Some(
                        PipelineSpecification {
                            name: "banner",
                            jobs: [
                                "unit-test,",
                            ],
                        },
                    ),
                    _eoi: EOI,
                }"#]],
        )
    }
}
