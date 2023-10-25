use banner_parser::ast::{self, IdentifierListItem};

use crate::{
    event_handler::EventHandler,
    ListenForEvent, ListenForEventType, ListenForSystemEventResult, ListenForSystemEventScope,
    ListenForSystemEventType, Metadata,
    Select::{Any, Only},
};

use super::{create_start_task_event_handler, task::generate_start_task_script};

pub fn get_eventhandlers_for_job(
    pipeline: Option<&ast::PipelineSpecification>,
    job: &ast::JobSpecification,
) -> Vec<EventHandler> {
    let mut event_handlers: Vec<EventHandler> = vec![];
    let pipeline = if let Some(pipeline) = pipeline {
        &pipeline.name
    } else {
        "_"
    };
    // create all the event handlers specific to a defined job. This requires:
    //   * an EH to trigger the first task in the job
    //   * an EH per other task in the job that triggers that task on the completion of the task preceeding it.
    //   * an EH to deal with the successful completion of the last task in the pipeline
    if job.tasks.is_empty() {
        let eh = create_start_empty_job_event_handler(pipeline, job);
        event_handlers.push(eh);
        return event_handlers;
    }

    let mut iterator = job.tasks.iter().rev().peekable();
    let mut some_task = iterator.next();

    // Create event handler to emit an event when the last task finishes.
    if let Some(task) = some_task {
        log::debug!("The job does have at least one task.  Task is: {task}");
        let mut veh = create_finished_job_event_handlers(pipeline, job, task);
        event_handlers.append(&mut veh);
    }

    while let Some(task) = some_task {
        // For every job we need to create:
        //   * an event handler to trigger on successful completion of the previous task.
        let next = iterator.peek();
        if let Some(jt) = next {
            let mut veh = create_start_task_event_handler(pipeline, job, task, jt);
            event_handlers.append(&mut veh);
        } else {
            // Create event handler to accept job triggers.  This event will only ever
            // emit another event which is a trigger to the first task(s) defined.
            let mut eh = create_start_job_event_handler(pipeline, &job.name, task);
            event_handlers.append(&mut eh);
        }
        some_task = iterator.next();
    }

    event_handlers
}

/// Creates a job start event handler for the any job.
/// This needs to listen for the completion of the previous job if there is one.
///
/// # Panics
///
/// Should not panic.
pub fn create_start_pipeline_job_event_handler(
    pipeline_name: &str,
    current: &IdentifierListItem,
    next: &IdentifierListItem,
) -> Vec<EventHandler> {
    let mut event_handlers: Vec<EventHandler> = vec![];
    match (current, next) {
        (
            IdentifierListItem::Identifier(job_name, _),
            IdentifierListItem::Identifier(nj_name, _),
        ) => {
            // nj = next job
            // The next_job should be triggered when the current job completes.
            // This is a sequential part of the pipeline, so it's a straight forward one job finishes
            // and the next job starts.
            let pipeline_tag = Metadata::new_banner_pipeline(pipeline_name);
            let job_tag = Metadata::new_banner_job(nj_name);
            let description_tag = Metadata::new_banner_description(&format!(
                "Trigger the start of the single job: {}/{}",
                pipeline_name, &nj_name
            ));
            let listen_for_event = ListenForEvent::new_builder(ListenForEventType::System(Only(
                ListenForSystemEventType::Done(
                    Only(ListenForSystemEventScope::Job),
                    Only(ListenForSystemEventResult::Success),
                ),
            )))
            .with_pipeline_name(pipeline_name)
            .with_job_name(job_name)
            .build();
            let eh = EventHandler::new(
                vec![pipeline_tag, job_tag, description_tag],
                vec![listen_for_event],
                generate_start_job_script(pipeline_name, nj_name),
            );
            event_handlers.push(eh);
        }
        (
            IdentifierListItem::Identifier(job_name, _),
            IdentifierListItem::SequentialList(_) | IdentifierListItem::ParallelList(_),
        ) => {
            // if we have a straight forward job followed by a list of jobs; either parallel
            // or sequential; then we need to trigger the first job in the list when the current
            // job finishes.
            let pipeline_tag = Metadata::new_banner_pipeline(pipeline_name);

            let lfe = ListenForEvent::new_builder(ListenForEventType::System(Only(
                ListenForSystemEventType::Done(
                    Only(ListenForSystemEventScope::Job),
                    Only(ListenForSystemEventResult::Success),
                ),
            )))
            .with_pipeline_name(pipeline_name)
            .with_job_name(job_name)
            .build();

            let next_jobs = flatten_jobs_for_start(next);
            for nj in next_jobs {
                let job_tag = Metadata::new_banner_job(nj);
                let description_tag = Metadata::new_banner_description(&format!(
                    "Trigger the start of the list-job: {pipeline_name}/{nj}"
                ));
                let eh = EventHandler::new(
                    vec![pipeline_tag.clone(), job_tag, description_tag],
                    vec![lfe.clone()],
                    generate_start_job_script(pipeline_name, nj),
                );
                event_handlers.push(eh);
            }
        }
        (IdentifierListItem::SequentialList(_), IdentifierListItem::SequentialList(_)) => todo!(),
        (IdentifierListItem::SequentialList(_), IdentifierListItem::ParallelList(_)) => todo!(),
        (
            IdentifierListItem::SequentialList(_) | IdentifierListItem::ParallelList(_),
            IdentifierListItem::Identifier(nj_name, _),
        ) => {
            // if we have any list type before a straight forward job, then we need to trigger
            // the next job after all the last jobs in the previous list have finished.
            let pipeline_tag = Metadata::new_banner_pipeline(pipeline_name);
            let job_tag = Metadata::new_banner_job(nj_name);
            let description_tag = Metadata::new_banner_description(&format!(
                "Trigger the start of the job: {pipeline_name}/{nj_name}"
            ));

            let previous_jobs = flatten_jobs_for_finish(current);

            for job in &previous_jobs {
                let listen_for_event = ListenForEvent::new_builder(ListenForEventType::System(
                    Only(ListenForSystemEventType::Done(
                        Only(ListenForSystemEventScope::Job),
                        Only(ListenForSystemEventResult::Success),
                    )),
                ))
                .with_pipeline_name(pipeline_name)
                .with_job_name(job)
                .build();

                let script = generate_execute_job_after_job_complete(
                    "",
                    pipeline_name,
                    nj_name,
                    &previous_jobs,
                );
                let eh = EventHandler::new(
                    vec![
                        pipeline_tag.clone(),
                        job_tag.clone(),
                        description_tag.clone(),
                    ],
                    vec![listen_for_event],
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

fn generate_execute_job_after_job_complete(
    scope: &str,
    pipeline: &str,
    job: &str,
    previous_jobs: &[&str],
) -> String {
    let script_vec = previous_jobs
        .iter()
        .map(|job| format!("\"{job}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r###"pub async fn main (engine, event) {{
    // mark this job as having completed.
    let values = engine.get_pipeline_metadata_from_event(event).await;
    let pipeline = values.0;
    let job = values.1;

    let key = format!("{scope}/{{}}/{{}}", pipeline, job);
    match event.get_type() {{
        EventType::System(event_type) => {{
            match event_type {{
                SystemEventType::Done(scope, result) => {{
                    match scope {{
                        SystemEventScope::Job => {{
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

    // if any of the parallel jobs have not completed or failed, then we really do have nothing to do.
    // return early.
    for awaiting_job in [{script_vec}] {{
        let key = format!("{scope}/{pipeline}/{{}}", awaiting_job);
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

    engine.trigger_job("{pipeline}", "{job}").await;
}}"###
    )
}

fn flatten_jobs_for_start(job: &IdentifierListItem) -> Vec<&str> {
    match job {
        IdentifierListItem::Identifier(task, _) => vec![task],
        IdentifierListItem::SequentialList(tasks) => {
            let first_task = tasks.first().unwrap();
            flatten_jobs_for_finish(first_task)
        }
        IdentifierListItem::ParallelList(tasks) => {
            let mut flattened_tasks = vec![];
            for task in tasks {
                let flattened_task = flatten_jobs_for_start(task);
                flattened_tasks.extend(flattened_task);
            }
            flattened_tasks
        }
    }
}

fn flatten_jobs_for_finish(job: &IdentifierListItem) -> Vec<&str> {
    match job {
        IdentifierListItem::Identifier(job, _) => {
            vec![job]
        }
        IdentifierListItem::SequentialList(list) => {
            let last_job = list.last().unwrap();
            flatten_jobs_for_finish(last_job)
        }
        IdentifierListItem::ParallelList(list) => {
            let mut flattened_jobs = vec![];
            for job in list {
                let flattened_job = flatten_jobs_for_finish(job);
                flattened_jobs.extend(flattened_job);
            }
            flattened_jobs
        }
    }
}

// Create an event handler that triggers the start of the first task in the defined job
// when an event is raised that triggers the job.
pub fn create_start_job_event_handler(
    pipeline: &str,
    job: &str,
    task: &IdentifierListItem,
) -> Vec<EventHandler> {
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let job_tag = Metadata::new_banner_job(job);
    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of the job: {pipeline}/{job}/{task}"
    ));
    let listen_for_event = ListenForEvent::new_builder(ListenForEventType::System(Only(
        ListenForSystemEventType::Trigger(Only(ListenForSystemEventScope::Job)),
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(job)
    .build();

    let mut event_handlers: Vec<EventHandler> = vec![];
    match task {
        IdentifierListItem::Identifier(task, _) => {
            let task_tag = Metadata::new_banner_job(task);
            let script = generate_start_task_script(pipeline, job, task);
            let eh = EventHandler::new(
                vec![pipeline_tag, job_tag, task_tag, description_tag],
                vec![listen_for_event],
                script,
            );
            event_handlers.push(eh);
        }
        IdentifierListItem::SequentialList(_) => todo!(),
        IdentifierListItem::ParallelList(_) => todo!(),
    }
    event_handlers
}

fn create_start_empty_job_event_handler(
    pipeline: &str,
    job: &ast::JobSpecification,
) -> EventHandler {
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let job_tag = Metadata::new_banner_job(&job.name);
    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of empty job: {}/{}",
        pipeline, &job.name
    ));
    let listen_for_event = ListenForEvent::new_builder(ListenForEventType::System(Only(
        ListenForSystemEventType::Done(
            Only(ListenForSystemEventScope::Job),
            Only(ListenForSystemEventResult::Success),
        ),
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(&job.name)
    .build();

    EventHandler::new(
        vec![pipeline_tag, job_tag, description_tag],
        vec![listen_for_event],
        generate_job_with_no_tasks_script(pipeline, &job.name),
    )
}

fn create_finished_job_event_handlers(
    pipeline: &str,
    job: &ast::JobSpecification,
    task: &IdentifierListItem,
) -> Vec<EventHandler> {
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let job_tag = Metadata::new_banner_job(&job.name);
    let description_tag = Metadata::new_banner_description(&format!(
        "Signal the completion of the job: {}/{}; Last task was: {}",
        pipeline, &job.name, task
    ));

    let mut event_handlers: Vec<EventHandler> = vec![];
    match task {
        IdentifierListItem::Identifier(task, _) => {
            let listen_for_event = create_finish_event_listener(pipeline, job, task);
            let script = generate_finish_job_script();
            let eh = EventHandler::new(
                vec![
                    pipeline_tag.clone(),
                    job_tag.clone(),
                    description_tag.clone(),
                ],
                vec![listen_for_event],
                script,
            );
            event_handlers.push(eh);
        }
        IdentifierListItem::SequentialList(_) => todo!(),
        IdentifierListItem::ParallelList(_) => todo!(),
    };

    event_handlers
}

fn create_finish_event_listener(
    pipeline: &str,
    job: &ast::JobSpecification,
    task: &str,
) -> ListenForEvent {
    ListenForEvent::new_builder(ListenForEventType::System(Only(
        ListenForSystemEventType::Done(Only(ListenForSystemEventScope::Task), Any),
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(&job.name)
    .with_task_name(task)
    .build()
}

fn generate_finish_job_script() -> String {
    r###"pub async fn main (engine, event) {
            engine.job_complete(event).await;
        }"###
        .to_string()
}

fn generate_job_with_no_tasks_script(pipeline: &str, job: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.log_message("Job [{pipeline}/{job}] has no tasks to trigger").await;
        }}"###
    )
}

fn generate_start_job_script(pipeline: &str, job: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.trigger_job("{pipeline}", "{job}").await;
        }}"###
    )
}
