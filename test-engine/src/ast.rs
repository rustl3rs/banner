use crate::{from_pest::FromPest, grammar::Rule};

use from_pest::{ConversionError, Void};
use pest::iterators::{Pair, Pairs};

pub struct Pragma {}

#[derive(Debug, Clone, PartialEq)]
pub struct Pragmas {
    pub pragma: Vec<ActionOrAssert>,
}

impl<'a> FromPest<'a> for Pragmas {
    type Rule = Rule;
    type FatalError = Void;
    fn from_pest(pest: &mut Pairs<'a, Rule>) -> Result<Self, ConversionError<Void>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(ConversionError::NoMatch)?;
        if pair.as_rule() == Rule::pragma {
            let span = pair.as_span();
            let mut inner = pair.into_inner();
            let inner = &mut inner;
            let this = Pragmas {
                pragma: FromPest::from_pest(inner)?,
            };
            if inner.clone().next().is_some() {
                log::trace!("when converting Pragmas, found extraneous {inner:?}");
                Err(ConversionError::Extraneous {
                    current_node: "Pragmas",
                })?;
            }
            *pest = clone;
            Ok(this)
        } else {
            Err(ConversionError::NoMatch)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionOrAssert {
    Trigger(Target),
    Assertion(Target, Outcome),
}

impl<'a> FromPest<'a> for ActionOrAssert {
    type Rule = Rule;
    type FatalError = Void;
    fn from_pest(pest: &mut Pairs<'a, Rule>) -> Result<Self, ConversionError<Void>> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Pipeline(String),
    Job(String, String),
    Task(String, String, String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Success,
    Failure,
}

#[cfg(test)]
mod ast_tests {
    use expect_test::{expect, Expect};
    use pest::Parser;

    use crate::grammar::TestPragmaParser;

    use super::*;
    fn check(input: &str, expect: &Expect) {
        tracing::trace!("input = {input:#?}");
        let mut parse_tree = TestPragmaParser::parse(Rule::pragma, input).unwrap();
        tracing::trace!("parse tree = {:#?}", parse_tree);
        match Pragmas::from_pest(&mut parse_tree) {
            Ok(tree) => {
                tracing::trace!("tree = {:#?}", tree);
                expect.assert_debug_eq(&tree);
            }
            Err(e) => panic!("{e:?}"),
        }
    }

    #[test]
    fn can_get_ast_from_single_line_pragma() {
        let input = r#"trigger_pipeline(test_pipeline)"#;
        check(
            input,
            &expect![[r#"
            Pragmas {
                pragma: [
                    Trigger(
                        Pipeline(
                            "test_pipeline",
                        ),
                    ),
                ],
            });"#]],
        );
    }

    #[test]
    fn can_get_ast_from_multiline_pragma() {
        let input = r#"trigger_pipeline(test_pipeline);
            expect [
                task_failure(test_pipeline, test_job, testTask),
                job_failure(test_pipeline, test_job),
                pipeline_failure(test_pipeline),
            ];
            "#;
        check(
            input,
            &expect![[r#"
            Pragmas {
                pragma: [
                    Trigger(
                        Pipeline(
                            "test_pipeline",
                        ),
                    ),
                    Assertion(
                        Task(
                            "test_pipeline",
                            "test_job",
                            "testTask",
                        ),
                        Failure,
                    ),
                    Assertion(
                        Job(
                            "test_pipeline",
                            "test_job",
                        ),
                        Failure,
                    ),
                    Assertion(
                        Pipeline(
                            "test_pipeline",
                        ),
                        Failure,
                    ),
                ],
            });"#]],
        );
    }

    #[test]
    fn can_get_ast_from_multi_line_pragma_with_comments() {
        let input = r#"
            // trigger the pipeline
            trigger_pipeline(test_pipeline);
            // assert the task, job and pipeline failed
            expect [
                task_failure(test_pipeline, test_job, testTask),
                job_failure(test_pipeline, test_job),
                pipeline_failure(test_pipeline),
            ];
            "#;

        check(
            input,
            &expect![[r#"
            Pragmas {
                pragma: [
                    Trigger(
                        Pipeline(
                            "test_pipeline",
                        ),
                    ),
                    Assertion(
                        Task(
                            "test_pipeline",
                            "test_job",
                            "testTask",
                        ),
                        Failure,
                    ),
                    Assertion(
                        Job(
                            "test_pipeline",
                            "test_job",
                        ),
                        Failure,
                    ),
                    Assertion(
                        Pipeline(
                            "test_pipeline",
                        ),
                        Failure,
                    ),
                ],
            });"#]],
        );
    }
}
