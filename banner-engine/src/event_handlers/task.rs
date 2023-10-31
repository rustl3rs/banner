use banner_parser::ast::{self, IdentifierListItem, JobSpecification};
use itertools::Itertools;

use crate::{
    event_handler::EventHandler,
    ListenForEvent, ListenForEventType, ListenForSystemEventResult, ListenForSystemEventScope,
    ListenForSystemEventType, Metadata,
    Select::{Any, Only},
    TaskDefinition, JOB_TAG, PIPELINE_TAG, TASK_TAG,
};

pub fn get_eventhandlers_for_task_definition(task_def: &TaskDefinition) -> Vec<EventHandler> {
    let mut tags: Vec<Metadata> = task_def.tags().iter().cloned().collect_vec();
    let pipeline_name = task_def
        .tags()
        .iter()
        .find(|tag| tag.key() == PIPELINE_TAG)
        .map_or("_", |tag| tag.value());
    let job_name = task_def
        .tags()
        .iter()
        .find(|tag| tag.key() == JOB_TAG)
        .map_or("_", |tag| tag.value());
    let task_name = task_def
        .tags()
        .iter()
        .find(|tag| tag.key() == TASK_TAG)
        .map_or("_", |tag| tag.value());
    let description_tag =
        Metadata::new_banner_description(&format!("Execute the task: {task_name}"));
    tags.extend(vec![description_tag]);

    let listen_for_event = ListenForEvent::new_builder(ListenForEventType::System(Only(
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
    let mut event_handlers: Vec<EventHandler> = vec![];
    match (task, next) {
        (IdentifierListItem::Identifier(task, _), IdentifierListItem::Identifier(next, _)) => {
            let eh = event_handler_for_task_followed_by_task(pipeline, job, task, next);
            event_handlers.push(eh);
        }
        (
            IdentifierListItem::Identifier(task, _),
            IdentifierListItem::SequentialList(_) | IdentifierListItem::ParallelList(_),
        ) => {
            let eh = event_handler_for_task_followed_by_list(job, pipeline, task, next);
            event_handlers.push(eh);
        }
        (IdentifierListItem::SequentialList(_), IdentifierListItem::Identifier(_, _)) => todo!(),
        (IdentifierListItem::SequentialList(_), IdentifierListItem::SequentialList(_)) => todo!(),
        (IdentifierListItem::SequentialList(_), IdentifierListItem::ParallelList(_)) => todo!(),
        (IdentifierListItem::ParallelList(_), IdentifierListItem::Identifier(next, _)) => {
            let next_event_handlers =
                event_handlers_for_list_followed_by_task(job, pipeline, task, next);
            event_handlers.extend(next_event_handlers);
        }
        (IdentifierListItem::ParallelList(_), IdentifierListItem::SequentialList(_)) => todo!(),
        (IdentifierListItem::ParallelList(_), IdentifierListItem::ParallelList(_)) => todo!(),
    }

    event_handlers
}

fn event_handlers_for_list_followed_by_task(
    job: &JobSpecification,
    pipeline: &str,
    task: &IdentifierListItem,
    next: &str,
) -> Vec<EventHandler> {
    let job_tag = Metadata::new_banner_job(&job.name);
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let listen_for_event = ListenForEvent::new_builder(ListenForEventType::System(Only(
        ListenForSystemEventType::Done(
            Only(ListenForSystemEventScope::Task),
            Only(ListenForSystemEventResult::Success),
        ),
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(&job.name)
    .with_task_name(next)
    .build();

    let mut next_event_handlers: Vec<EventHandler> = vec![];
    for task in &flatten_task_for_start(task) {
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
        next_event_handlers.push(eh);
    }
    next_event_handlers
}

fn event_handler_for_task_followed_by_list(
    job: &JobSpecification,
    pipeline: &str,
    task: &str,
    next: &IdentifierListItem,
) -> EventHandler {
    // create an event handler that will trigger the task when all the previous parallel tasks have completed.
    let job_tag = Metadata::new_banner_job(&job.name);
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let prev_tasks = flatten_task_for_finish(next);
    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of the task: {}/{}/{}",
        pipeline, &job.name, task
    ));
    let script =
        generate_execute_task_after_tasks_complete("", pipeline, &job.name, task, &prev_tasks);

    let events = prev_tasks
        .iter()
        .map(|task| {
            ListenForEvent::new_builder(ListenForEventType::System(Only(
                ListenForSystemEventType::Done(Only(ListenForSystemEventScope::Task), Any),
            )))
            .with_pipeline_name(pipeline)
            .with_job_name(&job.name)
            .with_task_name(task)
            .build()
        })
        .collect::<Vec<ListenForEvent>>();

    let task_tag = Metadata::new_banner_task(task);

    EventHandler::new(
        vec![pipeline_tag, job_tag, task_tag, description_tag],
        events,
        script,
    )
}

fn event_handler_for_task_followed_by_task(
    pipeline: &str,
    job: &JobSpecification,
    task: &str,
    next: &str,
) -> EventHandler {
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let job_tag = Metadata::new_banner_job(&job.name);

    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of the task: {}/{}/{}",
        pipeline, &job.name, task
    ));

    let listen_for_event = ListenForEvent::new_builder(ListenForEventType::System(Only(
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

    EventHandler::new(
        vec![pipeline_tag, job_tag, task_tag, description_tag],
        vec![listen_for_event],
        generate_start_task_script(pipeline, &job.name, task),
    )
}

fn flatten_task_for_start(task: &IdentifierListItem) -> Vec<&str> {
    match task {
        IdentifierListItem::Identifier(task, _) => vec![task],
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
        IdentifierListItem::Identifier(task, _) => vec![task],
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
    parallel_tasks_to_complete: &[&str],
) -> String {
    let script_vec = parallel_tasks_to_complete
        .iter()
        .map(|task| format!("\"{task}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r###"pub async fn main (engine, event) {{
    // mark this task as having completed.
    let values = get_pipeline_metadata_from_event(event);
    let pipeline = values.0;
    let job = values.1;
    let task = values.2;

    let key = format!("{scope}/{{}}/{{}}/{{}}", pipeline, job, task);
    match event.get_type() {{
        EventType::System(event_type) => {{
            match event_type {{
                SystemEventType::Done(scope, result) => {{
                    match scope {{
                        SystemEventScope::Task => {{
                            engine.set_state_for_task("latest", key, result).await;
                        }},
                        _ => return,
                    }}
                }},
                _ => return,
            }}
        }}
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
        match state {{
            Some(result) => {{
                match result {{
                    SystemEventResult::Success => {{}},
                    _ => {{
                        return;
                    }},
                }}
            }}
            None => {{
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
                let values = get_pipeline_metadata_from_event(event);
                let pipeline = values.0;
                let job = values.1;
                let task = values.2;

                let key = format!("scope/{}/{}/{}", pipeline, job, task);
                match event.get_type() {
                    EventType::System(event_type) => {
                        match event_type {
                            SystemEventType::Done(scope, result) => {
                                match scope {
                                    SystemEventScope::Task => {
                                        engine.set_state_for_task("latest", key, result).await;
                                    },
                                    _ => return,
                                }
                            },
                            _ => return,
                        }
                    }
                    _ => {
                        engine.log_message(format!("Result is not a EventType::System")).await;
                        return
                    },
                }

                // if any of the parallel tasks have not completed or failed, then we really do have nothing to do.
                // return early.
                for awaiting_task in ["task1","task2"] {
                    let key = format!("scope/pipeline1/job1/{}", awaiting_task);
                    let state = engine.get_from_state("latest", key).await;
                    match state {
                        Some(result) => {
                            match result {
                                SystemEventResult::Success => {},
                                _ => {
                                    return;
                                },
                            }
                        }
                        None => {
                            return;
                        }
                    }
                }

                engine.trigger_task("pipeline1", "job1", "task3").await;
            }"#]];
        let script = generate_execute_task_after_tasks_complete(
            "scope",
            "pipeline1",
            "job1",
            "task3",
            &["task1", "task2"],
        );
        expect.assert_eq(&script);
    }
}

#[cfg(test)]
mod flatten_tasks_tests {
    use expect_test::{expect, Expect};

    use super::*;

    fn check_flatten_tasks_for_finish(tasks: &IdentifierListItem, expect: &Expect) {
        let actual = format!("{:#?}", flatten_task_for_finish(tasks));
        expect.assert_eq(&actual);
    }

    fn check_flatten_tasks_for_start(tasks: &IdentifierListItem, expect: &Expect) {
        let actual = format!("{:#?}", flatten_task_for_start(tasks));
        expect.assert_eq(&actual);
    }

    #[test]
    fn two_parallel_tasks_to_finish() {
        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::Identifier("task1".to_string(), vec![]),
            IdentifierListItem::Identifier("task2".to_string(), vec![]),
        ]);
        check_flatten_tasks_for_finish(
            &tasks,
            &expect![[r#"
            [
                "task1",
                "task2",
            ]"#]],
        );
    }

    #[test]
    fn two_parallel_tasks_to_start() {
        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::Identifier("task1".to_string(), vec![]),
            IdentifierListItem::Identifier("task2".to_string(), vec![]),
        ]);
        check_flatten_tasks_for_start(
            &tasks,
            &expect![[r#"
            [
                "task1",
                "task2",
            ]"#]],
        );
    }

    #[test]
    fn sequential_and_parallel_tasks_to_finish() {
        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::SequentialList(vec![
                IdentifierListItem::Identifier("task2".to_string(), vec![]),
                IdentifierListItem::Identifier("task3".to_string(), vec![]),
            ]),
            IdentifierListItem::Identifier("task1".to_string(), vec![]),
        ]);
        check_flatten_tasks_for_finish(
            &tasks,
            &expect![[r#"
            [
                "task3",
                "task1",
            ]"#]],
        );

        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::Identifier("task1".to_string(), vec![]),
            IdentifierListItem::SequentialList(vec![
                IdentifierListItem::Identifier("task2".to_string(), vec![]),
                IdentifierListItem::Identifier("task3".to_string(), vec![]),
            ]),
        ]);
        check_flatten_tasks_for_finish(
            &tasks,
            &expect![[r#"
            [
                "task1",
                "task3",
            ]"#]],
        );

        let tasks = IdentifierListItem::SequentialList(vec![
            IdentifierListItem::Identifier("task1".to_string(), vec![]),
            IdentifierListItem::ParallelList(vec![
                IdentifierListItem::Identifier("task2".to_string(), vec![]),
                IdentifierListItem::SequentialList(vec![
                    IdentifierListItem::Identifier("task3".to_string(), vec![]),
                    IdentifierListItem::Identifier("task4".to_string(), vec![]),
                ]),
            ]),
        ]);
        check_flatten_tasks_for_finish(
            &tasks,
            &expect![[r#"
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
                IdentifierListItem::Identifier("task2".to_string(), vec![]),
                IdentifierListItem::Identifier("task3".to_string(), vec![]),
            ]),
            IdentifierListItem::Identifier("task1".to_string(), vec![]),
        ]);
        check_flatten_tasks_for_start(
            &tasks,
            &expect![[r#"
            [
                "task2",
                "task1",
            ]"#]],
        );

        let tasks = IdentifierListItem::ParallelList(vec![
            IdentifierListItem::Identifier("task1".to_string(), vec![]),
            IdentifierListItem::SequentialList(vec![
                IdentifierListItem::Identifier("task2".to_string(), vec![]),
                IdentifierListItem::Identifier("task3".to_string(), vec![]),
            ]),
        ]);
        check_flatten_tasks_for_start(
            &tasks,
            &expect![[r#"
            [
                "task1",
                "task2",
            ]"#]],
        );

        let tasks = IdentifierListItem::SequentialList(vec![
            IdentifierListItem::Identifier("task1".to_string(), vec![]),
            IdentifierListItem::ParallelList(vec![
                IdentifierListItem::Identifier("task2".to_string(), vec![]),
                IdentifierListItem::SequentialList(vec![
                    IdentifierListItem::Identifier("task3".to_string(), vec![]),
                    IdentifierListItem::Identifier("task4".to_string(), vec![]),
                ]),
            ]),
        ]);
        check_flatten_tasks_for_start(
            &tasks,
            &expect![[r#"
            [
                "task1",
            ]"#]],
        );
    }
}
