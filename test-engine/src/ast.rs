// use std::{error::Error, f32::consts::E};

use crate::{from_pest::FromPest, grammar::Rule};

use from_pest::{ConversionError, Void};
use pest::iterators::Pairs;

// pub struct Pragma {}

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
            let mut inner = pair.into_inner();
            let inner = &mut inner;
            tracing::debug!("Pragma is:: {inner}");
            let prag = inner.next().unwrap();
            let this = match prag.as_rule() {
                Rule::single_line_pragma => {
                    let mut inner = prag.into_inner();
                    let inner = &mut inner;

                    let pair = inner.next().ok_or(ConversionError::NoMatch)?;
                    let mut inner = pair.clone().into_inner();

                    tracing::debug!("It's a single line pragma: {inner}");
                    let action = ActionOrAssert::from_pest(&mut inner)?;
                    tracing::debug!("PRAGMA: {action:?}");
                    Pragmas {
                        pragma: vec![action],
                    }
                }
                Rule::multi_line_pragma => {
                    let mut inner = prag.into_inner();
                    tracing::debug!("MULTI-LINE INNER: {inner:?}");
                    // let inner = &mut inner;
                    let mut actions = vec![];
                    while let Some(action) = inner.next() {
                        tracing::debug!("===> MLP - ACTION?: {action:?}");
                        let inner = &mut action.into_inner();
                        tracing::debug!("===> MLP - INNER ACTION?: {inner:?}");
                        let action = ActionOrAssert::from_pest(inner)?;
                        tracing::debug!("===> MLP - ACTION?: {action:?}");
                        actions.push(action);
                    }
                    Pragmas { pragma: actions }
                }
                _ => todo!(),
            };

            // if inner.clone().next().is_some() {
            //     tracing::trace!("when converting Pragmas, found extraneous {inner:?}");
            //     Err(ConversionError::Extraneous {
            //         current_node: "Pragmas",
            //     })?;
            // }
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
    Asserts(Vec<(Target, Outcome)>),
}

impl<'a> FromPest<'a> for ActionOrAssert {
    type Rule = Rule;
    type FatalError = Void;
    fn from_pest(pest: &mut Pairs<'a, Rule>) -> Result<Self, ConversionError<Void>> {
        tracing::debug!("============== ACTION_OR_ASSERT");
        let mut clone = pest.clone();

        let pair = clone.next().ok_or(ConversionError::NoMatch)?;
        let mut inner = pair.clone().into_inner();

        let this = match pair.clone().as_rule() {
            Rule::action => {
                tracing::debug!("Action: {}", inner);
                let action = inner.clone().next().ok_or(ConversionError::NoMatch)?;
                match action.as_rule() {
                    Rule::trigger_pipeline => {
                        tracing::debug!("TRIGGER_PIPELINE: {}", inner);
                        let pair = inner.next().ok_or(ConversionError::NoMatch)?;
                        tracing::debug!("TRIGGER_PIPELINE: {action:?}/{pair}");
                        let mut inner = pair.into_inner();
                        // let inner = &mut inner;

                        tracing::debug!("TRIGGER: {action:?}/{inner}");
                        let pipeline = inner
                            .next()
                            .ok_or(ConversionError::NoMatch)?
                            .as_str()
                            .to_string();
                        tracing::debug!("TARGET IS: {pipeline:?}");
                        if inner.clone().next().is_some() {
                            tracing::trace!(
                                "when converting ActionOrAssert, found extraneous {inner:?}"
                            );
                            Err(ConversionError::Extraneous {
                                current_node: "ActionOrAssert",
                            })?;
                        }
                        ActionOrAssert::Trigger(Target::Pipeline(pipeline))
                    }
                    Rule::trigger_job => {
                        tracing::debug!("TRIGGER_JOB: {}", inner);
                        let pair = inner.next().ok_or(ConversionError::NoMatch)?;
                        tracing::debug!("TRIGGER_JOB: {action:?}/{pair}");
                        let mut inner = pair.into_inner();

                        tracing::debug!("TRIGGER: {action:?}/{inner}");
                        let pipeline = inner
                            .next()
                            .ok_or(ConversionError::NoMatch)?
                            .as_str()
                            .to_string();
                        let job = inner
                            .next()
                            .ok_or(ConversionError::NoMatch)?
                            .as_str()
                            .to_string();
                        tracing::debug!("TARGET IS: {pipeline:?} / {job:?}");
                        if inner.clone().next().is_some() {
                            tracing::trace!(
                                "when converting ActionOrAssert, found extraneous {inner:?}"
                            );
                            Err(ConversionError::Extraneous {
                                current_node: "ActionOrAssert",
                            })?;
                        }
                        ActionOrAssert::Trigger(Target::Job(pipeline, job))
                    }
                    Rule::trigger_task => {
                        todo!()
                    }
                    _ => {
                        tracing::trace!(
                            "when converting ActionOrAssert, found no match: {action} / {:?}",
                            pair.as_rule()
                        );
                        return Err(ConversionError::NoMatch);
                    }
                }
            }
            Rule::expect => {
                let mut list_of_asserts = Vec::new();
                tracing::debug!("ASSERT: {inner}");
                while let Some(event) = inner.next() {
                    tracing::debug!("EVENT: {event:?}");
                    let mut inner = event.into_inner();
                    tracing::debug!("ASSERT IS: {inner:?}");
                    match inner.next() {
                        Some(pair) => match pair.as_rule() {
                            Rule::task_success | Rule::task_failure => {
                                let outcome = outcome_from_rule(pair.as_rule()).unwrap();
                                let mut inner = pair.into_inner();
                                let pipeline = inner
                                    .next()
                                    .ok_or(ConversionError::NoMatch)?
                                    .as_str()
                                    .to_string();
                                let job = inner
                                    .next()
                                    .ok_or(ConversionError::NoMatch)?
                                    .as_str()
                                    .to_string();
                                let task = inner
                                    .next()
                                    .ok_or(ConversionError::NoMatch)?
                                    .as_str()
                                    .to_string();
                                let task = Target::Task(pipeline, job, task);
                                tracing::debug!("ASSERT TASK: {task:?}");
                                tracing::debug!("ASSERT TASK INNER: {inner:?}");
                                list_of_asserts.push((task, outcome))
                            }
                            Rule::job_success | Rule::job_failure => {
                                let outcome = outcome_from_rule(pair.as_rule()).unwrap();
                                let mut inner = pair.into_inner();
                                let pipeline = inner
                                    .next()
                                    .ok_or(ConversionError::NoMatch)?
                                    .as_str()
                                    .to_string();
                                let job = inner
                                    .next()
                                    .ok_or(ConversionError::NoMatch)?
                                    .as_str()
                                    .to_string();
                                let task = Target::Job(pipeline, job);
                                tracing::debug!("ASSERT JOB: {task:?}");
                                list_of_asserts.push((task, outcome))
                            }
                            Rule::pipeline_success | Rule::pipeline_failure => {
                                let outcome = outcome_from_rule(pair.as_rule()).unwrap();
                                let mut inner = pair.into_inner();
                                let pipeline = inner
                                    .next()
                                    .ok_or(ConversionError::NoMatch)?
                                    .as_str()
                                    .to_string();
                                let task = Target::Pipeline(pipeline);
                                tracing::debug!("ASSERT PIPELINE: {task:?}");
                                list_of_asserts.push((task, outcome))
                            }
                            _ => {
                                tracing::debug!("ASSERT IS EMPTY");
                            }
                        },
                        None => {
                            tracing::debug!("ASSERT IS EMPTY");
                        }
                    }
                }
                tracing::debug!("ASSERTS: {list_of_asserts:?}");
                ActionOrAssert::Asserts(list_of_asserts)
            }
            _ => {
                tracing::trace!(
                    "when converting ActionOrAssert, found no match: {inner} / {:?}",
                    pair.as_rule()
                );
                return Err(ConversionError::NoMatch);
            }
        };
        *pest = clone;
        Ok(this)
    }
}

#[derive(Debug)]
pub enum OutcomeFromRuleError {
    InvalidRule,
}

fn outcome_from_rule(rule: Rule) -> Result<Outcome, OutcomeFromRuleError> {
    match rule {
        Rule::task_success | Rule::job_success | Rule::pipeline_success => Ok(Outcome::Success),
        Rule::task_failure | Rule::job_failure | Rule::pipeline_failure => Ok(Outcome::Failure),
        _ => Err(OutcomeFromRuleError::InvalidRule),
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
    #[tracing_test::traced_test]
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
            }
"#]],
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn can_get_ast_from_multi_line_pragma_pipeline_triggers_only() {
        let input = r#"trigger_pipeline(test_pipeline);
            // Some comment
            trigger_pipeline(test_pipeline2);
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
                    Trigger(
                        Pipeline(
                            "test_pipeline2",
                        ),
                    ),
                ],
            }
"#]],
        );
    }

    #[test]
    #[tracing_test::traced_test]
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
                        Asserts(
                            [
                                (
                                    Task(
                                        "test_pipeline",
                                        "test_job",
                                        "testTask",
                                    ),
                                    Failure,
                                ),
                                (
                                    Job(
                                        "test_pipeline",
                                        "test_job",
                                    ),
                                    Failure,
                                ),
                                (
                                    Pipeline(
                                        "test_pipeline",
                                    ),
                                    Failure,
                                ),
                            ],
                        ),
                    ],
                }
            "#]],
        );
    }

    #[test]
    #[tracing_test::traced_test]
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
                        Asserts(
                            [
                                (
                                    Task(
                                        "test_pipeline",
                                        "test_job",
                                        "testTask",
                                    ),
                                    Failure,
                                ),
                                (
                                    Job(
                                        "test_pipeline",
                                        "test_job",
                                    ),
                                    Failure,
                                ),
                                (
                                    Pipeline(
                                        "test_pipeline",
                                    ),
                                    Failure,
                                ),
                            ],
                        ),
                    ],
                }
            "#]],
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn can_get_ast_from_multiline_pragma_job() {
        let input = r#"trigger_job(test_pipeline, success_job);
            expect [
                task_failure(test_pipeline, test_job, testTask),
                job_failure(test_pipeline, test_job),
                pipeline_failure(test_pipeline),
                task_success(test_pipeline, test_job, successTask),
                job_success(test_pipeline, success_job),
            ];
            "#;
        check(
            input,
            &expect![[r#"
                Pragmas {
                    pragma: [
                        Trigger(
                            Job(
                                "test_pipeline",
                                "success_job",
                            ),
                        ),
                        Asserts(
                            [
                                (
                                    Task(
                                        "test_pipeline",
                                        "test_job",
                                        "testTask",
                                    ),
                                    Failure,
                                ),
                                (
                                    Job(
                                        "test_pipeline",
                                        "test_job",
                                    ),
                                    Failure,
                                ),
                                (
                                    Pipeline(
                                        "test_pipeline",
                                    ),
                                    Failure,
                                ),
                                (
                                    Task(
                                        "test_pipeline",
                                        "test_job",
                                        "successTask",
                                    ),
                                    Success,
                                ),
                                (
                                    Job(
                                        "test_pipeline",
                                        "success_job",
                                    ),
                                    Success,
                                ),
                            ],
                        ),
                    ],
                }
            "#]],
        );
    }
}
