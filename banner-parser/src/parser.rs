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
mod banner_parser_tests {
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
        let code = fs::read_to_string("../pipeline-assets/banner-pipeline-get-only.ban")
            .expect("Should have been able to read the file");

        check(&code, expect!["()"])
    }
}

#[cfg(test)]
mod pipeline_from_ast_tests {
    use expect_test::{expect, Expect};
    use from_pest::FromPest;
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

    #[traced_test]
    #[test]
    fn test_double_forward_slash_in_string() {
        let code = r#######"
        task unit-test(image: rustl3rs/banner-rust-build, execute: "/bin/bash -c") {
            r#"
            // this is a comment
            curl https://banner.io/api/v1/echo
            "#
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
                            tags: [],
                            name: "unit-test",
                            image: "rustl3rs/banner-rust-build",
                            command: StringLiteral(
                                "/bin/bash -c",
                            ),
                            script: RawString(
                                1,
                                "\n            // this is a comment\n            curl https://banner.io/api/v1/echo\n            ",
                            ),
                        },
                    ],
                    jobs: [],
                    pipelines: [],
                    eoi: EOI,
                }"#]],
        )
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
                                1,
                                "/bin/bash -c",
                            ),
                            script: RawString(
                                5,
                                "bash\n            echo testing, testing, 1, 2, 3!\n            ",
                            ),
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
                                1,
                                "",
                            ),
                            script: RawString(
                                1,
                                "",
                            ),
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
    fn can_parse_pipeline_with_job_and_task_reversed() {
        let code = r#######"
            pipeline my_pipeline [
                build
            ]
            
            import file://./single_task.ban
            
            job build [
                // this is a comment...
                cowsay,
                cowsay
            ]
            
            [tag: banner.io/owner=me]
            [tag: banner.io/company=rustl3rs]
            task cowsay(image: kmcgivern/cowsay-alpine:latest, execute: r#""#) {r#""#}
        "#######;

        check(
            &code,
            expect![[r#"
            Pipeline {
                imports: [
                    Import {
                        uri: "file://./single_task.ban",
                    },
                ],
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
                        name: "cowsay",
                        image: "kmcgivern/cowsay-alpine:latest",
                        command: RawString(
                            1,
                            "",
                        ),
                        script: RawString(
                            1,
                            "",
                        ),
                    },
                ],
                jobs: [
                    JobSpecification {
                        name: "build",
                        tasks: [
                            "cowsay",
                            "cowsay",
                        ],
                    },
                ],
                pipelines: [
                    PipelineSpecification {
                        name: "my_pipeline",
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
        job build [build]

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
                            tasks: [
                                "build",
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
    fn can_parse_job() {
        let code = r#######"
        job build [build, test]
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
                        tasks: [
                            "build",
                            "test",
                        ],
                    },
                ],
                pipelines: [],
                eoi: EOI,
            }"#]],
        )
    }

    #[traced_test]
    #[test]
    fn can_parse_task_with_var() {
        let code = r#######"
        let build_image = Image {
            name=rancher/alpine-git:latest,
            mount=[
                ${src} => "/source",
            ]
        }
        
        task unit-test(image: ${build_image}, execute: r#"/bin/bash -c"#) {
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
                    images: [
                        ImageDefinition {
                            name: "build_image",
                            image: Image {
                                name: "rancher/alpine-git:latest",
                                mounts: [
                                    Mount {
                                        source: EngineSupplied(
                                            "src",
                                        ),
                                        destination: StringLiteral(
                                            "/source",
                                        ),
                                    },
                                ],
                                envs: [],
                            },
                        },
                    ],
                    tasks: [
                        Task {
                            tags: [],
                            name: "unit-test",
                            image: "${build_image}",
                            command: RawString(
                                1,
                                "/bin/bash -c",
                            ),
                            script: RawString(
                                5,
                                "bash\n            echo testing, testing, 1, 2, 3!\n            ",
                            ),
                        },
                    ],
                    jobs: [],
                    pipelines: [],
                    eoi: EOI,
                }"#]],
        )
    }
}

#[cfg(test)]
mod string_tests {
    use expect_test::{expect, Expect};
    use tracing_test::traced_test;

    use super::*;

    fn check(code: &str, expect: Expect) {
        match BannerParser::parse(Rule::string_literal, &code) {
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

    #[traced_test]
    #[test]
    fn test_string_parsing() {
        let code = r#######""this is a string""#######;
        check(
            &code,
            expect![[r#"
            [
                Pair {
                    rule: string_literal,
                    span: Span {
                        str: "\"this is a string\"",
                        start: 0,
                        end: 18,
                    },
                    inner: [
                        Pair {
                            rule: standard_string,
                            span: Span {
                                str: "\"this is a string\"",
                                start: 0,
                                end: 18,
                            },
                            inner: [
                                Pair {
                                    rule: string_content,
                                    span: Span {
                                        str: "this is a string",
                                        start: 1,
                                        end: 17,
                                    },
                                    inner: [],
                                },
                            ],
                        },
                    ],
                },
            ]"#]],
        );

        let code = r#######"r#"this is a string"#"#######;
        check(
            &code,
            expect![[r##"
            [
                Pair {
                    rule: string_literal,
                    span: Span {
                        str: "r#\"this is a string\"#",
                        start: 0,
                        end: 21,
                    },
                    inner: [
                        Pair {
                            rule: raw_string,
                            span: Span {
                                str: "r#\"this is a string\"#",
                                start: 0,
                                end: 21,
                            },
                            inner: [
                                Pair {
                                    rule: raw_string_interior,
                                    span: Span {
                                        str: "this is a string",
                                        start: 3,
                                        end: 19,
                                    },
                                    inner: [],
                                },
                            ],
                        },
                    ],
                },
            ]"##]],
        );
    }
}

#[cfg(test)]
mod identifier_list_tests {
    use expect_test::{expect, Expect};
    use tracing_test::traced_test;

    use super::*;

    fn check(code: &str, expect: Expect) {
        match BannerParser::parse(Rule::identifier_list, &code) {
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

    #[traced_test]
    #[test]
    fn single_item_list() {
        let code = r#######"cowsay"#######;
        check(
            &code,
            expect![[r#"
            [
                Pair {
                    rule: identifier_list,
                    span: Span {
                        str: "cowsay",
                        start: 0,
                        end: 6,
                    },
                    inner: [
                        Pair {
                            rule: identifier,
                            span: Span {
                                str: "cowsay",
                                start: 0,
                                end: 6,
                            },
                            inner: [],
                        },
                    ],
                },
            ]"#]],
        );

        let code = r#######"cowsay,"#######;
        check(
            &code,
            expect![[r#"
            [
                Pair {
                    rule: identifier_list,
                    span: Span {
                        str: "cowsay,",
                        start: 0,
                        end: 7,
                    },
                    inner: [
                        Pair {
                            rule: identifier,
                            span: Span {
                                str: "cowsay",
                                start: 0,
                                end: 6,
                            },
                            inner: [],
                        },
                    ],
                },
            ]"#]],
        );
    }

    #[traced_test]
    #[test]
    fn multi_item_list() {
        let code = r#######"cowsay,test"#######;
        check(
            &code,
            expect![[r#"
            [
                Pair {
                    rule: identifier_list,
                    span: Span {
                        str: "cowsay,test",
                        start: 0,
                        end: 11,
                    },
                    inner: [
                        Pair {
                            rule: identifier,
                            span: Span {
                                str: "cowsay",
                                start: 0,
                                end: 6,
                            },
                            inner: [],
                        },
                        Pair {
                            rule: identifier,
                            span: Span {
                                str: "test",
                                start: 7,
                                end: 11,
                            },
                            inner: [],
                        },
                    ],
                },
            ]"#]],
        );

        let code = r#######"cowsay,test,"#######;
        check(
            &code,
            expect![[r#"
            [
                Pair {
                    rule: identifier_list,
                    span: Span {
                        str: "cowsay,test,",
                        start: 0,
                        end: 12,
                    },
                    inner: [
                        Pair {
                            rule: identifier,
                            span: Span {
                                str: "cowsay",
                                start: 0,
                                end: 6,
                            },
                            inner: [],
                        },
                        Pair {
                            rule: identifier,
                            span: Span {
                                str: "test",
                                start: 7,
                                end: 11,
                            },
                            inner: [],
                        },
                    ],
                },
            ]"#]],
        );
    }
}
