use banner_parser::ast::{self, IdentifierListItem, JobSpecification, PipelineSpecification};
use itertools::Itertools;

use crate::{
    event_handler::EventHandler, ListenForEvent, ListenForEventType, ListenForSystemEventResult,
    ListenForSystemEventScope, ListenForSystemEventType, Metadata, Select::*, TaskDefinition,
    JOB_TAG, PIPELINE_TAG, TASK_TAG,
};

pub fn get_eventhandlers_for_task(
    pipeline: Option<&PipelineSpecification>,
    job: Option<&JobSpecification>,
    task: &ast::TaskSpecification,
) -> Vec<EventHandler> {
    // create all the event handlers specific to a defined task. This requires:
    //   * an EH to execute the task when a trigger task event has been raised for this specific task.
    let pipeline_name = if let Some(pipeline) = pipeline {
        &pipeline.name
    } else {
        "_"
    };
    let job_name = if let Some(job) = job { &job.name } else { "_" };
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline_name);
    let job_tag = Metadata::new_banner_job(job_name);
    let task_tag = Metadata::new_banner_task(&task.name);
    let description_tag =
        Metadata::new_banner_description(&format!("Execute the task: {}", &task.name));
    let listen_for_event = ListenForEvent::new(ListenForEventType::System(Only(
        ListenForSystemEventType::Trigger(Only(ListenForSystemEventScope::Task)),
    )))
    .with_pipeline_name(pipeline_name)
    .with_job_name(job_name)
    .with_task_name(&task.name)
    .build();
    let script = generate_execute_task_script("", pipeline_name, job_name, &task.name); // TODO: fix scope
    let eh = EventHandler::new(
        vec![pipeline_tag, job_tag, task_tag, description_tag],
        vec![listen_for_event],
        script,
    );
    vec![eh]
}

pub fn get_eventhandlers_for_task_definition(task_def: &TaskDefinition) -> Vec<EventHandler> {
    let mut tags: Vec<Metadata> = task_def.tags().iter().map(|tag| tag.clone()).collect_vec();
    let task_name = task_def
        .tags()
        .iter()
        .find(|tag| tag.key() == TASK_TAG)
        .unwrap()
        .value()
        .clone();
    let job_name = task_def
        .tags()
        .iter()
        .find(|tag| tag.key() == JOB_TAG)
        .unwrap()
        .value()
        .clone();
    let pipeline_name = task_def
        .tags()
        .iter()
        .find(|tag| tag.key() == PIPELINE_TAG)
        .unwrap()
        .value()
        .clone();
    let description_tag =
        Metadata::new_banner_description(&format!("Execute the task: {}", task_name));
    tags.extend(vec![description_tag]);

    let listen_for_event = ListenForEvent::new(ListenForEventType::System(Only(
        ListenForSystemEventType::Trigger(Only(ListenForSystemEventScope::Task)),
    )))
    .with_pipeline_name(pipeline_name)
    .with_job_name(job_name)
    .with_task_name(task_name)
    .build();
    let script = generate_execute_task_script("", pipeline_name, job_name, task_name); // TODO: fix scope
    let eh = EventHandler::new(tags, vec![listen_for_event], script);
    vec![eh]
}

// Create an event handler to trigger on successful completion of the previous task.
pub fn create_start_task_event_handler(
    pipeline: &str,
    job: &ast::JobSpecification,
    task: &IdentifierListItem,
    next: &IdentifierListItem,
) -> Vec<EventHandler> {
    let job_tag = Metadata::new_banner_job(&job.name);
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);

    let mut event_handlers: Vec<EventHandler> = vec![];
    match (task, next) {
        (IdentifierListItem::Identifier(task), IdentifierListItem::Identifier(next)) => {
            let description_tag = Metadata::new_banner_description(&format!(
                "Trigger the start of the task: {}/{}/{}",
                pipeline, &job.name, task
            ));

            let listen_for_event = ListenForEvent::new(ListenForEventType::System(Only(
                ListenForSystemEventType::Done(
                    Only(ListenForSystemEventScope::Task),
                    Only(ListenForSystemEventResult::Success),
                ),
            )))
            .with_pipeline_name(pipeline)
            .with_job_name(&job.name)
            .with_task_name(next)
            .build();

            let task_tag = Metadata::new_banner_task(task);
            let eh = EventHandler::new(
                vec![pipeline_tag, job_tag, task_tag, description_tag],
                vec![listen_for_event],
                generate_start_task_script(pipeline, &job.name, task),
            );

            event_handlers.push(eh);
        }
        (IdentifierListItem::Identifier(task), IdentifierListItem::SequentialList(_))
        | (IdentifierListItem::Identifier(task), IdentifierListItem::ParallelList(_)) => {
            // create an event handler that will trigger the task when all the previous parallel tasks have completed.
            let job_tag = Metadata::new_banner_job(&job.name);
            let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
            let prev_tasks = flatten_task_for_finish(next);
            let description_tag = Metadata::new_banner_description(&format!(
                "Trigger the start of the task: {}/{}/{}",
                pipeline, &job.name, task
            ));
            let script = generate_execute_task_after_tasks_complete(
                "",
                pipeline,
                &job.name,
                task,
                &prev_tasks,
            );

            let events = prev_tasks
                .iter()
                .map(|task| {
                    ListenForEvent::new(ListenForEventType::System(Only(
                        ListenForSystemEventType::Done(Only(ListenForSystemEventScope::Task), Any),
                    )))
                    .with_pipeline_name(pipeline)
                    .with_job_name(&job.name)
                    .with_task_name(task)
                    .build()
                })
                .collect::<Vec<ListenForEvent>>();

            let task_tag = Metadata::new_banner_task(task);
            let eh = EventHandler::new(
                vec![pipeline_tag, job_tag, task_tag, description_tag],
                events,
                script,
            );

            event_handlers.push(eh);
        }
        (IdentifierListItem::SequentialList(_), IdentifierListItem::Identifier(_)) => todo!(),
        (IdentifierListItem::SequentialList(_), IdentifierListItem::SequentialList(_)) => todo!(),
        (IdentifierListItem::SequentialList(_), IdentifierListItem::ParallelList(_)) => todo!(),
        (IdentifierListItem::ParallelList(_), IdentifierListItem::Identifier(prev)) => {
            let job_tag = Metadata::new_banner_job(&job.name);
            let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
            let listen_for_event = ListenForEvent::new(ListenForEventType::System(Only(
                ListenForSystemEventType::Done(
                    Only(ListenForSystemEventScope::Task),
                    Only(ListenForSystemEventResult::Success),
                ),
            )))
            .with_pipeline_name(pipeline)
            .with_job_name(&job.name)
            .with_task_name(prev)
            .build();

            for task in flatten_task_for_start(task).iter() {
                let description_tag = Metadata::new_banner_description(&format!(
                    "Trigger the start of the task: {}/{}/{}",
                    pipeline, &job.name, task
                ));
                let script = generate_start_task_script(pipeline, &job.name, task);
                let task_tag = Metadata::new_banner_task(task);
                let eh = EventHandler::new(
                    vec![
                        pipeline_tag.clone(),
                        job_tag.clone(),
                        task_tag,
                        description_tag,
                    ],
                    vec![listen_for_event.clone()],
                    script,
                );
                event_handlers.push(eh);
            }
        }
        (IdentifierListItem::ParallelList(_), IdentifierListItem::SequentialList(_)) => todo!(),
        (IdentifierListItem::ParallelList(_), IdentifierListItem::ParallelList(_)) => todo!(),
    }

    event_handlers
}

fn flatten_task_for_start(task: &IdentifierListItem) -> Vec<&str> {
    match task {
        IdentifierListItem::Identifier(task) => vec![task],
        IdentifierListItem::SequentialList(tasks) => {
            let first_task = tasks.first().unwrap();
            flatten_task_for_finish(first_task)
        }
        IdentifierListItem::ParallelList(tasks) => {
            let mut flattened_tasks = vec![];
            for task in tasks {
                let flattened_task = flatten_task_for_start(task);
                flattened_tasks.extend(flattened_task);
            }
            flattened_tasks
        }
    }
}

fn flatten_task_for_finish(task: &IdentifierListItem) -> Vec<&str> {
    match task {
        IdentifierListItem::Identifier(task) => vec![task],
        IdentifierListItem::SequentialList(tasks) => {
            let last_task = tasks.last().unwrap();
            flatten_task_for_finish(last_task)
        }
        IdentifierListItem::ParallelList(tasks) => {
            let mut flattened_tasks = vec![];
            for task in tasks {
                let flattened_task = flatten_task_for_finish(task);
                flattened_tasks.extend(flattened_task);
            }
            flattened_tasks
        }
    }
}

pub(crate) fn generate_start_task_script(pipeline: &str, job: &str, task: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.trigger_task("{pipeline}", "{job}", "{task}").await;
        }}"###
    )
}

fn generate_execute_task_script(scope: &str, pipeline: &str, job: &str, task: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.execute_task_name_in_scope("{scope}", "{pipeline}", "{job}", "{task}").await;
        }}"###
    )
}

fn generate_execute_task_after_tasks_complete(
    scope: &str,
    pipeline: &str,
    job: &str,
    task: &str,
    parallel_tasks_to_complete: &Vec<&str>,
) -> String {
    let script_vec = parallel_tasks_to_complete
        .iter()
        .map(|task| format!("\"{task}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r###"pub async fn main (engine, event) {{
    // mark this task as having completed.
    let values = engine.get_pipeline_metadata_from_event(event).await;
    let pipeline = values.0;
    let job = values.1;
    let task = values.2;

    let key = format!("{scope}/{{}}/{{}}/{{}}", pipeline, job, task);
    // engine.log_message(format!("========> EVENT - Key {{:?}}", key)).await;
    // engine.log_message(format!("=====> EventType1: {{:?}}", event.get_type())).await;
    match event.get_type() {{
        EventType::System(event_type) => {{
            match event_type {{
                SystemEventType::Done(scope, result) => {{
                    match scope {{
                        SystemEventScope::Task => {{
                            engine.set_state_for_task("latest", key, result).await;
                            // match result {{
                            //     SystemEventResult::Success => {{
                            //         engine.set_state_for_task("latest", key, "success").await;
                            //     }},
                            //     SystemEventResult::Failure => {{
                            //         engine.set_state_for_task("latest", key, "failure").await;
                            //     }},
                            //     _ => return,
                            // }}
                        }},
                        _ => return,
                    }}
                }},
                _ => return,
            }}
            // engine.log_message(format!("=====> EventType: {{}}", event_type)).await;
        }}
        // Some(result) => {{
        // }}
        // None => {{
        //     panic!("Result is None");
        // }},
        _ => {{
            engine.log_message(format!("Result is not a EventType::System")).await;
            return
        }},
    }}

    // if any of the parallel tasks have not completed or failed, then we really do have nothing to do.
    // return early.
    for awaiting_task in [{script_vec}] {{
        let key = format!("{scope}/{pipeline}/{job}/{{}}", awaiting_task);
        let state = engine.get_from_state("latest", key).await;
        // engine.log_message(format!("========> EVENT - State {{:?}}", state)).await;
        match state {{
            Some(result) => {{
                // engine.log_message(format!("========> EVENT - Result {{}}", result)).await;
                match result {{
                    SystemEventResult::Success => {{
                        // engine.log_message(format!("========> EVENT - Task {{}} has completed successfully.", awaiting_task)).await;
                    }},
                    _ => {{
                        // engine.log_message(format!("========> EVENT - Task {{}} has did not completed successfully. You might want to try re-running.", awaiting_task)).await;
                        return;
                    }},
                }}
            }}
            None => {{
                // engine.log_message(format!("========> EVENT - Task {{}} has not completed successfully yet. Waiting for it to complete.", awaiting_task)).await;
                return;
            }}
        }}
    }}

    engine.trigger_task("{pipeline}", "{job}", "{task}").await;
}}"###
    )
}

#[cfg(test)]
mod script_tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn generates_correct_script() {
        let expect = expect![[r#"
            pub async fn main (engine, event) {
                // mark this task as having completed.
                let key = format!("scope/pipeline1/job1/task3");
                match event {
                    Some(result) => {
                        engine.set_state_for_task("latest", key, result).await;
                    }
                    None => {
                        return;
                        // panic!("Result is None");
                    },
                }

                // if any of the parallel tasks have not completed or failed, then we really do have nothing to do.
                // return early.
                for awaiting_task in ["task1","task2"] {
                    let key = format!("scope/pipeline1/job1/{}", awaiting_task);
                    let state = engine.get_from_state("latest", key).await;
                    engine.log_message(format!("========> EVENT - State {:?}", state)).await;
                    match state {
                        Some(result) => {
                            // engine.log_message(format!("========> EVENT - Task {} - result {:?}", awaiting_task, result)).await;
                            match result {
                                Success => {
                                    engine.log_message(format!("========> EVENT - Task {} has completed successfully.", awaiting_task)).await;
                                },
                                _ => {
                                    engine.log_message(format!("========> EVENT - Task {} has did not complete successfully. You might want to try re-running.", awaiting_task)).await;
                                    return;
                                },
                            }
                        }
                        None => {
                            engine.log_message(format!("========> EVENT - Task {} has not completed successfully yet. Waiting for it to complete.", awaiting_task)).await;
                            return;
                        }
                    }
                }

                engine.execute_task_name_in_scope("scope", "pipeline1", "job1", "task3").await;
            }"#]];
        let script = generate_execute_task_after_tasks_complete(
            "scope",
            "pipeline1",
            "job1",
            "task3",
            &vec!["task1", "task2"],
        );
        expect.assert_eq(&script);
    }
}

#[cfg(test)]
mod flatten_tasks_tests {
    use expect_test::{expect, Expect};

    use super::*;

    fn check_flatten_tasks_for_finish(tasks: &IdentifierListItem, expect: Expect) {
        let actual = format!("{:#?}", flatten_task_for_finish(tasks));
        expect.assert_eq(&actual);
    }

    fn check_flatten_tasks_for_start(tasks: &IdentifierListItem, expect: Expect) {
        let actual = format!("{:#?}", flatten_task_for_start(tasks));
        expect.assert_eq(&actual);
    }

    #[test]
    fn two_parallel_tasks_to_finish() {
        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::Identifier("task1".to_string()),
            IdentifierListItem::Identifier("task2".to_string()),
        ]);
        check_flatten_tasks_for_finish(
            &tasks,
            expect![[r#"
            [
                "task1",
                "task2",
            ]"#]],
        )
    }

    #[test]
    fn two_parallel_tasks_to_start() {
        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::Identifier("task1".to_string()),
            IdentifierListItem::Identifier("task2".to_string()),
        ]);
        check_flatten_tasks_for_start(
            &tasks,
            expect![[r#"
            [
                "task1",
                "task2",
            ]"#]],
        )
    }

    #[test]
    fn sequential_and_parallel_tasks_to_finish() {
        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::SequentialList(vec![
                IdentifierListItem::Identifier("task2".to_string()),
                IdentifierListItem::Identifier("task3".to_string()),
            ]),
            IdentifierListItem::Identifier("task1".to_string()),
        ]);
        check_flatten_tasks_for_finish(
            &tasks,
            expect![[r#"
            [
                "task3",
                "task1",
            ]"#]],
        );

        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::Identifier("task1".to_string()),
            IdentifierListItem::SequentialList(vec![
                IdentifierListItem::Identifier("task2".to_string()),
                IdentifierListItem::Identifier("task3".to_string()),
            ]),
        ]);
        check_flatten_tasks_for_finish(
            &tasks,
            expect![[r#"
            [
                "task1",
                "task3",
            ]"#]],
        );

        let tasks = IdentifierListItem::SequentialList(vec![
            IdentifierListItem::Identifier("task1".to_string()),
            IdentifierListItem::ParallelList(vec![
                IdentifierListItem::Identifier("task2".to_string()),
                IdentifierListItem::SequentialList(vec![
                    IdentifierListItem::Identifier("task3".to_string()),
                    IdentifierListItem::Identifier("task4".to_string()),
                ]),
            ]),
        ]);
        check_flatten_tasks_for_finish(
            &tasks,
            expect![[r#"
            [
                "task2",
                "task4",
            ]"#]],
        );
    }

    #[test]
    fn sequential_and_parallel_tasks_to_start() {
        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::SequentialList(vec![
                IdentifierListItem::Identifier("task2".to_string()),
                IdentifierListItem::Identifier("task3".to_string()),
            ]),
            IdentifierListItem::Identifier("task1".to_string()),
        ]);
        check_flatten_tasks_for_start(
            &tasks,
            expect![[r#"
            [
                "task2",
                "task1",
            ]"#]],
        );

        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::Identifier("task1".to_string()),
            IdentifierListItem::SequentialList(vec![
                IdentifierListItem::Identifier("task2".to_string()),
                IdentifierListItem::Identifier("task3".to_string()),
            ]),
        ]);
        check_flatten_tasks_for_start(
            &tasks,
            expect![[r#"
            [
                "task1",
                "task2",
            ]"#]],
        );

        let tasks = IdentifierListItem::SequentialList(vec![
            IdentifierListItem::Identifier("task1".to_string()),
            IdentifierListItem::ParallelList(vec![
                IdentifierListItem::Identifier("task2".to_string()),
                IdentifierListItem::SequentialList(vec![
                    IdentifierListItem::Identifier("task3".to_string()),
                    IdentifierListItem::Identifier("task4".to_string()),
                ]),
            ]),
        ]);
        check_flatten_tasks_for_start(
            &tasks,
            expect![[r#"
            [
                "task1",
            ]"#]],
        );
    }
}