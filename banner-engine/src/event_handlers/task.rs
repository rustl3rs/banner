use banner_parser::ast::{self, IdentifierListItem, JobSpecification, PipelineSpecification};

use crate::{
    event_handler::EventHandler, Event, EventType, Metadata, SystemEventResult, SystemEventScope,
    SystemEventType,
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
    let listen_for_event = Event::new(EventType::System(SystemEventType::Trigger(
        SystemEventScope::Task,
    )))
    .with_task_name(&task.name)
    .with_job_name(job_name)
    .with_pipeline_name(pipeline_name)
    .build();
    let script = generate_execute_task_script("", pipeline_name, job_name, &task.name); // TODO: fix scope
    let eh = EventHandler::new(
        vec![pipeline_tag, job_tag, task_tag, description_tag],
        vec![listen_for_event],
        script,
    );
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

    let mut event_handers: Vec<EventHandler> = vec![];
    match (task, next) {
        (IdentifierListItem::Identifier(task), IdentifierListItem::Identifier(next)) => {
            let description_tag = Metadata::new_banner_description(&format!(
                "Trigger the start of the task: {}/{}/{}",
                pipeline, &job.name, task
            ));

            let listen_for_event = Event::new(EventType::System(SystemEventType::Done(
                SystemEventScope::Task,
                SystemEventResult::Success,
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

            event_handers.push(eh);
        }
        (IdentifierListItem::Identifier(_), IdentifierListItem::SequentialList(_)) => todo!(),
        (IdentifierListItem::Identifier(_), IdentifierListItem::ParallelList(_)) => todo!(),
        (IdentifierListItem::SequentialList(_), IdentifierListItem::Identifier(_)) => todo!(),
        (IdentifierListItem::SequentialList(_), IdentifierListItem::SequentialList(_)) => todo!(),
        (IdentifierListItem::SequentialList(_), IdentifierListItem::ParallelList(_)) => todo!(),
        (IdentifierListItem::ParallelList(_), IdentifierListItem::Identifier(_)) => todo!(),
        (IdentifierListItem::ParallelList(_), IdentifierListItem::SequentialList(_)) => todo!(),
        (IdentifierListItem::ParallelList(_), IdentifierListItem::ParallelList(_)) => todo!(),
    }

    event_handers
}

pub(crate) fn generate_start_task_script(pipeline: &str, job: &str, task: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.trigger_task("{pipeline}", "{job}", "{task}").await;
        }}
        "###
    )
}

fn generate_execute_task_script(scope: &str, pipeline: &str, job: &str, task: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.execute_task_name_in_scope("{scope}", "{pipeline}", "{job}", "{task}").await;
        }}
        "###
    )
}
