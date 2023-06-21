use banner_parser::ast::{self, IdentifierListItem, PipelineSpecification};

use crate::{
    event_handler::EventHandler, event_handlers::create_start_pipeline_job_event_handler, Event,
    EventType, Metadata, SystemEventResult, SystemEventScope, SystemEventType,
};

pub fn create_finished_pipeline_event_handler(
    pipeline: &ast::PipelineSpecification,
    job: &IdentifierListItem,
) -> Vec<EventHandler> {
    let pipeline_tag = Metadata::new_banner_pipeline(&pipeline.name);
    let description_tag = Metadata::new_banner_description(&format!(
        "Signal the completion of the pipeline: {}; Last job was: {}",
        &pipeline.name, &job
    ));

    let mut event_handlers = Vec::<EventHandler>::new();
    match job {
        IdentifierListItem::Identifier(job) => {
            let event = create_success_event_listener(&pipeline.name, job);
            let script = generate_finish_pipeline_on_success_script(&pipeline.name);
            let eh = EventHandler::new(
                vec![pipeline_tag.clone(), description_tag.clone()],
                vec![event],
                script,
            );
            event_handlers.push(eh);

            let event = create_fail_event_listener(&pipeline.name, job);
            let script = generate_finish_pipeline_on_fail_script(&pipeline.name);
            let eh = EventHandler::new(vec![pipeline_tag, description_tag], vec![event], script);
            event_handlers.push(eh);
        }
        IdentifierListItem::SequentialList(jobs) => todo!(),
        IdentifierListItem::ParallelList(jobs) => todo!(),
    };

    event_handlers
}

fn create_success_event_listener(pipeline: &str, job: &str) -> Event {
    Event::new(EventType::System(SystemEventType::Done(
        SystemEventScope::Job,
        SystemEventResult::Success,
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(job)
    .build()
}

fn create_fail_event_listener(pipeline: &str, job: &str) -> Event {
    Event::new(EventType::System(SystemEventType::Done(
        SystemEventScope::Job,
        SystemEventResult::Failed,
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(job)
    .build()
}

pub fn create_start_empty_pipeline_event_handler(pipeline: &PipelineSpecification) -> EventHandler {
    let pipeline_tag = Metadata::new_banner_pipeline(&pipeline.name);
    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of the pipeline: {}",
        &pipeline.name
    ));
    let listen_for_event = Event::new(EventType::System(SystemEventType::Trigger(
        SystemEventScope::Pipeline,
    )))
    .with_pipeline_name(&pipeline.name)
    .build();
    let script = generate_pipeline_with_no_jobs_script(&pipeline.name);
    let eh = EventHandler::new(
        vec![pipeline_tag, description_tag],
        vec![listen_for_event],
        script,
    );
    eh
}

pub fn create_start_pipeline_event_handler(
    pipeline: &ast::PipelineSpecification,
    job: &IdentifierListItem,
) -> EventHandler {
    let pipeline_tag = Metadata::new_banner_pipeline(&pipeline.name);
    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of the pipeline: {}/{}",
        &pipeline.name, &job
    ));
    let listen_for_event = Event::new(EventType::System(SystemEventType::Trigger(
        SystemEventScope::Pipeline,
    )))
    .with_pipeline_name(&pipeline.name)
    .build();
    let script = match job {
        IdentifierListItem::Identifier(job) => generate_start_pipeline_script_single_job(&pipeline.name, job),
        IdentifierListItem::SequentialList(jobs) => panic!("Sequential lists in this context are not supported. This should really never happen, since the parser should have caught this."),
        IdentifierListItem::ParallelList(jobs) => todo!(),
    };
    let eh = EventHandler::new(
        vec![pipeline_tag, description_tag],
        vec![listen_for_event],
        script,
    );
    eh
}

pub fn get_eventhandlers_for_pipeline(pipeline: &ast::PipelineSpecification) -> Vec<EventHandler> {
    // create all the event handlers specific to a defined pipeline. This requires:
    //   * an EH to trigger the first job in the pipeline
    //   * an EH per other job in the pipeline that triggers that job on the completion of the job preceeding it.
    //   * an EH to deal with the successful completion of the last job in the pipeline
    tracing::trace!("get_eventhandlers_for_pipeline: pipeline: {pipeline:?}");
    let mut event_handlers: Vec<EventHandler> = vec![];
    if pipeline.jobs.len() == 0 {
        let eh = create_start_empty_pipeline_event_handler(pipeline);
        event_handlers.push(eh);
        return event_handlers;
    }

    let mut iterator = pipeline.jobs.iter().rev();
    let mut some_job = iterator.next();

    // Create event handler to emit an event when the last job finishes.
    if let Some(job) = some_job {
        let mut veh = create_finished_pipeline_event_handler(pipeline, job);
        event_handlers.append(&mut veh);
    }

    while let Some(job) = some_job {
        // For every job we need to create:
        //   * an event handler to trigger on successful completion of the previous job.
        let next = iterator.clone().next();
        tracing::trace!("get_eventhandlers_for_pipeline: job: {job:?}, next: {next:?}");
        if next.is_some() {
            let mut eh = create_start_pipeline_job_event_handler(pipeline, job, next);
            event_handlers.append(&mut eh);
        } else {
            // Create event handler to accept pipeline triggers.  This event will only ever
            // emit another event which is a trigger to the first job(s) defined.
            let eh = create_start_pipeline_event_handler(pipeline, job);
            event_handlers.push(eh);
        }
        some_job = iterator.next();
    }

    event_handlers
}

fn generate_finish_pipeline_on_success_script(pipeline: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.pipeline_success("{pipeline}").await;
        }}
        "###
    )
}

fn generate_finish_pipeline_on_fail_script(pipeline: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.pipeline_fail("{pipeline}").await;
        }}
        "###
    )
}

fn generate_start_pipeline_script_single_job(pipeline: &str, job_name: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.trigger_job("{pipeline}", "{job_name}").await;
        }}
        "###
    )
}

fn generate_pipeline_with_no_jobs_script(pipeline: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.log_message("Pipeline [{pipeline}] has no jobs to trigger").await;
        }}
        "###
    )
}
