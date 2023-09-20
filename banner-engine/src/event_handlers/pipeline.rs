use banner_parser::ast::{self, IdentifierListItem, PipelineSpecification};

use crate::{
    event_handler::EventHandler, event_handlers::create_start_pipeline_job_event_handler,
    ListenForEvent, ListenForEventType, ListenForSystemEventResult, ListenForSystemEventScope,
    ListenForSystemEventType, Metadata, Select::*,
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
        IdentifierListItem::SequentialList(_jobs) => todo!(),
        IdentifierListItem::ParallelList(_jobs) => todo!(),
    };

    event_handlers
}

fn create_success_event_listener(pipeline: &str, job: &str) -> ListenForEvent {
    ListenForEvent::new_builder(ListenForEventType::System(Only(
        ListenForSystemEventType::Done(
            Only(ListenForSystemEventScope::Job),
            Only(ListenForSystemEventResult::Success),
        ),
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(job)
    .build()
}

fn create_fail_event_listener(pipeline: &str, job: &str) -> ListenForEvent {
    ListenForEvent::new_builder(ListenForEventType::System(Only(
        ListenForSystemEventType::Done(
            Only(ListenForSystemEventScope::Job),
            Only(ListenForSystemEventResult::Failed),
        ),
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
    let listen_for_event = ListenForEvent::new_builder(ListenForEventType::System(Only(
        ListenForSystemEventType::Trigger(Only(ListenForSystemEventScope::Pipeline)),
    )))
    .with_pipeline_name(&pipeline.name)
    .build();
    let script = generate_pipeline_with_no_jobs_script(&pipeline.name);

    EventHandler::new(
        vec![pipeline_tag, description_tag],
        vec![listen_for_event],
        script,
    )
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
    let listen_for_event = ListenForEvent::new_builder(ListenForEventType::System(Only(
        ListenForSystemEventType::Trigger(Only(ListenForSystemEventScope::Pipeline)),
    )))
    .with_pipeline_name(&pipeline.name)
    .build();
    let script = match job {
        IdentifierListItem::Identifier(job) => generate_start_pipeline_script_single_job(&pipeline.name, job),
        IdentifierListItem::SequentialList(_jobs) => panic!("Sequential lists in this context are not supported. This should really never happen, since the parser should have caught this."),
        IdentifierListItem::ParallelList(jobs) => {
            let start_jobs:Vec<&str> = jobs.iter().flat_map(|j| match j {
                IdentifierListItem::Identifier(id) => vec![id.as_str()],
                IdentifierListItem::SequentialList(list) => get_first_jobs_sequential(list),
                IdentifierListItem::ParallelList(list) => get_all_jobs_parallel(list),
            }).collect();
            generate_start_pipeline_script_multi_job(&pipeline.name, start_jobs)
        },
    };

    EventHandler::new(
        vec![pipeline_tag, description_tag],
        vec![listen_for_event],
        script,
    )
}

fn get_all_jobs_parallel(list: &[IdentifierListItem]) -> Vec<&str> {
    list.iter()
        .flat_map(|j| match j {
            IdentifierListItem::Identifier(id) => vec![id.as_str()],
            IdentifierListItem::SequentialList(list) => get_first_jobs_sequential(list),
            IdentifierListItem::ParallelList(list) => get_all_jobs_parallel(list),
        })
        .collect()
}

fn get_first_jobs_sequential(list: &[IdentifierListItem]) -> Vec<&str> {
    let first = list.first().unwrap();
    match first {
        IdentifierListItem::Identifier(id) => vec![id.as_str()],
        IdentifierListItem::SequentialList(list) => get_first_jobs_sequential(list),
        IdentifierListItem::ParallelList(list) => get_all_jobs_parallel(list),
    }
}

pub fn get_eventhandlers_for_pipeline(pipeline: &ast::PipelineSpecification) -> Vec<EventHandler> {
    // create all the event handlers specific to a defined pipeline. This requires:
    //   * an EH to trigger the first job in the pipeline
    //   * an EH per other job in the pipeline that triggers that job on the completion of the job preceeding it.
    //   * an EH to deal with the successful completion of the last job in the pipeline
    tracing::trace!("get_eventhandlers_for_pipeline: pipeline: {pipeline:?}");
    let mut event_handlers: Vec<EventHandler> = vec![];
    if pipeline.jobs.is_empty() {
        let eh = create_start_empty_pipeline_event_handler(pipeline);
        event_handlers.push(eh);
        return event_handlers;
    }

    let first_job = match pipeline.jobs.first() {
        Some(job) => {
            let eh = create_start_pipeline_event_handler(pipeline, job);
            event_handlers.push(eh);
            job
        }
        None => return event_handlers,
    };

    // if there was a last, there is most certainly a first.
    let last_job = pipeline.jobs.last().unwrap();
    let mut eh = create_finished_pipeline_event_handler(pipeline, last_job);
    event_handlers.append(&mut eh);

    // no point continuing if there is only one job.
    if last_job == first_job {
        return event_handlers;
    }

    let mut iterator = pipeline.jobs.iter();
    let _ = iterator.next();

    let eh = get_event_handlers_for_job_type(pipeline, first_job, &mut iterator);
    event_handlers.extend(eh);

    event_handlers
}

fn get_event_handlers_for_job_type(
    pipeline: &PipelineSpecification,
    current_job: &IdentifierListItem,
    iterator: &mut std::slice::Iter<'_, IdentifierListItem>,
) -> Vec<EventHandler> {
    let mut event_handlers: Vec<EventHandler> = vec![];
    let next_job = iterator.next();

    log::debug!(
        target: "task_log",
        "get_event_handlers_for_job_type: Current Job is {current_job:?}"
    );

    if next_job.is_none() {
        log::debug!(
            target: "task_log",
            "get_event_handlers_for_job_type: Next Job is NONE"
        );
        return event_handlers;
    }

    let next_job = next_job.unwrap();
    match next_job {
        IdentifierListItem::Identifier(_) => {
            log::debug!(
                target: "task_log",
                "get_event_handlers_for_job_type: NEXT JOB: {next_job:?}"
            );
            let eh = create_start_pipeline_job_event_handler(&pipeline.name, current_job, next_job);
            event_handlers.extend(eh);
        }
        IdentifierListItem::ParallelList(list) => {
            log::debug!(
                target: "task_log",
                "get_event_handlers_for_job_type: parallel list: {list:?}"
            );
            let eh = create_start_pipeline_job_event_handler(&pipeline.name, current_job, next_job);
            event_handlers.extend(eh);

            for job in list.iter() {
                match job {
                    IdentifierListItem::Identifier(_) => (),
                    IdentifierListItem::SequentialList(list) => {
                        let mut iter = list.iter();
                        let _ = iter.next();
                        let eh = get_event_handlers_for_job_type(
                            pipeline,
                            list.first().unwrap(),
                            &mut iter,
                        );
                        event_handlers.extend(eh);
                    }
                    IdentifierListItem::ParallelList(_) => todo!(),
                }
            }
        }
        IdentifierListItem::SequentialList(list) => {
            log::debug!(
                target: "task_log",
                "get_event_handlers_for_job_type: seq list: {list:?}"
            );
            let eh = create_start_pipeline_job_event_handler(&pipeline.name, current_job, next_job);
            event_handlers.extend(eh);

            let mut iter = list.iter();
            let _ = iter.next();
            let eh = get_event_handlers_for_job_type(pipeline, list.first().unwrap(), &mut iter);
            event_handlers.extend(eh);
        }
    }

    let eh = get_event_handlers_for_job_type(pipeline, next_job, iterator);
    event_handlers.extend(eh);

    event_handlers
}

fn generate_finish_pipeline_on_success_script(pipeline: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.pipeline_success("{pipeline}").await;
        }}"###
    )
}

fn generate_finish_pipeline_on_fail_script(pipeline: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.pipeline_fail("{pipeline}").await;
        }}"###
    )
}

fn generate_start_pipeline_script_single_job(pipeline: &str, job_name: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.trigger_job("{pipeline}", "{job_name}").await;
        }}"###
    )
}

fn generate_pipeline_with_no_jobs_script(pipeline: &str) -> String {
    format!(
        r###"pub async fn main (engine, event) {{
            engine.log_message("Pipeline [{pipeline}] has no jobs to trigger").await;
        }}"###
    )
}

fn generate_start_pipeline_script_multi_job(pipeline: &str, jobs: Vec<&str>) -> String {
    let mut script = String::from(r###"pub async fn main (engine, event) {{"###);
    for job in jobs {
        script.push_str(&format!(
            r###"
            engine.trigger_job("{pipeline}", "{job}").await;
            "###
        ));
    }
    script.push_str("        }}");
    script
}
