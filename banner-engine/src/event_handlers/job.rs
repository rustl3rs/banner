use banner_parser::ast::{self, IdentifierListItem};

use crate::{
    event_handler::EventHandler, ListenForEvent, ListenForEventType, ListenForSystemEventResult,
    ListenForSystemEventScope, ListenForSystemEventType, Metadata, Select::*,
};

use super::{create_start_task_event_handler, task::generate_start_task_script};

pub fn get_eventhandlers_for_job(
    pipeline: Option<&ast::PipelineSpecification>,
    job: &ast::JobSpecification,
) -> Vec<EventHandler> {
    let pipeline = if let Some(pipeline) = pipeline {
        &pipeline.name
    } else {
        "_"
    };
    // create all the event handlers specific to a defined job. This requires:
    //   * an EH to trigger the first task in the job
    //   * an EH per other task in the job that triggers that task on the completion of the task preceeding it.
    //   * an EH to deal with the successful completion of the last task in the pipeline
    let mut event_handlers: Vec<EventHandler> = vec![];
    if job.tasks.len() == 0 {
        let eh = create_start_empty_job_event_handler(pipeline, job);
        event_handlers.push(eh);
        return event_handlers;
    }

    let mut iterator = job.tasks.iter().rev();
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
        let next = iterator.clone().next();
        if next.is_some() {
            let mut veh = create_start_task_event_handler(pipeline, job, task, next.unwrap());
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
    pipeline: &ast::PipelineSpecification,
    job: &IdentifierListItem,
    next: Option<&IdentifierListItem>,
) -> Vec<EventHandler> {
    let pipeline_tag = Metadata::new_banner_pipeline(&pipeline.name);
    let mut event_handlers: Vec<EventHandler> = vec![];
    match job {
        IdentifierListItem::Identifier(job) => {
            let job_tag = Metadata::new_banner_job(job);
            let description_tag = Metadata::new_banner_description(&format!(
                "Trigger the start of the job: {}/{}",
                &pipeline.name, &job
            ));
            match next {
                Some(next_job_item) => {
                    match next_job_item {
                        IdentifierListItem::Identifier(next_job) => {
                            let listen_for_event = ListenForEvent::new(ListenForEventType::System(
                                Only(ListenForSystemEventType::Done(
                                    Only(ListenForSystemEventScope::Job),
                                    Only(ListenForSystemEventResult::Success),
                                )),
                            ))
                            .with_pipeline_name(&pipeline.name)
                            .with_job_name(next_job)
                            .build();
                            let eh = EventHandler::new(
                                vec![pipeline_tag, job_tag, description_tag],
                                vec![listen_for_event],
                                generate_start_job_script(job, &pipeline.name),
                            );
                            event_handlers.push(eh);
                        }
                        IdentifierListItem::SequentialList(_) => todo!(),
                        IdentifierListItem::ParallelList(_) => todo!(),
                    };
                }
                None => todo!(),
            };
        }
        IdentifierListItem::SequentialList(_) => todo!(),
        IdentifierListItem::ParallelList(_) => todo!(),
    };
    event_handlers
}

// Create an event handler that triggers the start of the first task in the defined job
// when an event is raised that triggers the job.
pub fn create_start_job_event_handler(
    pipeline: &str,
    job: &str,
    task: &IdentifierListItem,
) -> Vec<EventHandler> {
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of the job: {}/{}/{}",
        pipeline, job, task
    ));
    let listen_for_event = ListenForEvent::new(ListenForEventType::System(Only(
        ListenForSystemEventType::Trigger(Only(ListenForSystemEventScope::Job)),
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(job)
    .build();

    let mut event_handlers: Vec<EventHandler> = vec![];
    match task {
        IdentifierListItem::Identifier(task) => {
            let script = generate_start_task_script(pipeline, job, task);
            let eh = EventHandler::new(
                vec![pipeline_tag, description_tag],
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
    let job_tag = Metadata::new_banner_job(&job.name);
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of empty job: {}/{}",
        pipeline, &job.name
    ));
    let listen_for_event = ListenForEvent::new(ListenForEventType::System(Only(
        ListenForSystemEventType::Done(
            Only(ListenForSystemEventScope::Job),
            Only(ListenForSystemEventResult::Success),
        ),
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(&job.name)
    .build();
    let eh = EventHandler::new(
        vec![pipeline_tag, job_tag, description_tag],
        vec![listen_for_event],
        generate_job_with_no_tasks_script(pipeline, &job.name),
    );
    eh
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
        IdentifierListItem::Identifier(task) => {
            let listen_for_success_event = create_success_event_listener(pipeline, job, task);
            let script = generate_finish_job_on_success_script(pipeline, &job.name);
            let eh = EventHandler::new(
                vec![
                    pipeline_tag.clone(),
                    job_tag.clone(),
                    description_tag.clone(),
                ],
                vec![listen_for_success_event],
                script,
            );
            event_handlers.push(eh);

            let listen_for_fail_event = create_fail_event_listener(pipeline, job, task);
            let script = generate_finish_job_on_fail_script(pipeline, &job.name);
            let eh = EventHandler::new(
                vec![pipeline_tag, job_tag, description_tag],
                vec![listen_for_fail_event],
                script,
            );

            event_handlers.push(eh);
        }
        IdentifierListItem::SequentialList(_) => todo!(),
        IdentifierListItem::ParallelList(_) => todo!(),
    };

    event_handlers
}

fn create_fail_event_listener(
    pipeline: &str,
    job: &ast::JobSpecification,
    task: &str,
) -> ListenForEvent {
    let listen_for_fail_event = ListenForEvent::new(ListenForEventType::System(Only(
        ListenForSystemEventType::Done(
            Only(ListenForSystemEventScope::Task),
            Only(ListenForSystemEventResult::Failed),
        ),
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(&job.name)
    .with_task_name(task)
    .build();
    listen_for_fail_event
}

fn create_success_event_listener(
    pipeline: &str,
    job: &ast::JobSpecification,
    task: &str,
) -> ListenForEvent {
    let listen_for_success_event = ListenForEvent::new(ListenForEventType::System(Only(
        ListenForSystemEventType::Done(
            Only(ListenForSystemEventScope::Task),
            Only(ListenForSystemEventResult::Success),
        ),
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(&job.name)
    .with_task_name(task)
    .build();
    listen_for_success_event
}

fn generate_finish_job_on_fail_script(pipeline: &str, job_name: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.job_fail("{pipeline}", "{job_name}").await;
        }}"###
    )
}

fn generate_finish_job_on_success_script(pipeline: &str, job_name: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.job_success("{pipeline}", "{job_name}").await;
        }}"###
    )
}

fn generate_job_with_no_tasks_script(pipeline: &str, job: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.log_message("Job [{pipeline}/{job}] has no tasks to trigger").await;
        }}"###
    )
}

fn generate_start_job_script(job: &str, pipeline: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.trigger_job("{pipeline}", "{job}").await;
        }}"###
    )
}
