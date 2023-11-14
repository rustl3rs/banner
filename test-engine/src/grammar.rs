extern crate pest;

#[derive(Parser)]
#[grammar = "grammar/pragma.pest"]
pub struct TestPragmaParser;

#[cfg(test)]
mod trigger_tests {
    use pest::Parser;

    use super::*;
    fn check_trigger_pipeline(input: &str) {
        let parse_tree = TestPragmaParser::parse(Rule::trigger_pipeline, input);
        tracing::debug!("{parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    fn check_trigger_job(input: &str) {
        let parse_tree = TestPragmaParser::parse(Rule::trigger_job, input);
        tracing::debug!("{parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    fn check_trigger_task(input: &str) {
        let parse_tree = TestPragmaParser::parse(Rule::trigger_task, input);
        tracing::debug!("{parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    #[test]
    fn can_parse_trigger_pipeline() {
        let input = r#"trigger_pipeline(test)"#;
        check_trigger_pipeline(input);
    }

    #[test]
    fn can_parse_trigger_job() {
        let input = r#"trigger_job(test_pipeline, test-job)"#;
        check_trigger_job(input);
    }

    #[test]
    fn can_parse_trigger_task() {
        let input = r#"trigger_task(test-pipe, test_job, testTask)"#;
        check_trigger_task(input);
    }
}

#[cfg(test)]
mod expect_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = TestPragmaParser::parse(Rule::expect, input);
        tracing::debug!("{parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    #[test]
    fn can_parse_successfull_events() {
        let input = r#"expect [
                task_success(test_pipeline, test_job, testTask),
                job_success(test_pipeline, test_job),
                pipeline_success(test_pipeline),
            ];
            "#;
        check(input);
    }

    #[test]
    fn can_parse_failure_events() {
        let input = r#"expect [
                task_failure(test_pipeline, test_job, testTask),
                job_failure(test_pipeline, test_job),
                pipeline_failure(test_pipeline),
            ];
            "#;
        check(input);
    }
}

#[cfg(test)]
mod pragma_tests {
    use pest::Parser;

    use super::*;
    fn check(input: &str) {
        let parse_tree = TestPragmaParser::parse(Rule::pragma, input);
        tracing::debug!("{parse_tree:?}");
        match parse_tree {
            Ok(_) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    // #[test]
    // fn can_parse_single_line_pragma() {
    //     let input = r#"trigger_pipeline(test_pipeline)"#;
    //     check(input);
    // }

    #[test]
    fn can_parse_multiline_pragma() {
        let input = r#"trigger_pipeline(test_pipeline);
            expect [
                task_failure(test_pipeline, test_job, testTask),
                job_failure(test_pipeline, test_job),
                pipeline_failure(test_pipeline),
            ];
            "#;
        check(input);
    }
}
