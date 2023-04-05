use std::{collections::HashSet, error::Error, fmt::Display, fs, path::PathBuf};

use banner_parser::{
    ast::{self, Import, JobSpecification, PipelineSpecification},
    grammar::{BannerParser, Rule},
    FromPest, Iri, Parser,
};
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use log::trace;

use crate::{
    event_handler::EventHandler, matching_banner_metadata, Event, EventType, Metadata,
    SystemEventResult, SystemEventScope, SystemEventType, TaskDefinition, MATCHING_TAG,
};

#[derive(Debug)]
pub struct Pipeline {
    pub tasks: Vec<TaskDefinition>,
    pub event_handlers: Vec<EventHandler>,
}

impl Pipeline {
    pub(crate) fn events_matching(&self, event: &Event) -> Vec<EventHandler> {
        // figure out what events are listening for this event....... eventtype + tag searching mostly.
        self.event_handlers
            .iter()
            .filter(|eh| eh.is_listening_for(event)) // filter on the type of event
            .filter(|eh| {
                let matching_tags: Vec<Metadata> = eh
                    .tags()
                    .iter()
                    .filter_map(|tag| {
                        if tag.key() == MATCHING_TAG {
                            let tag = Metadata::from(tag.value());
                            Some(tag)
                        } else {
                            None
                        }
                    })
                    .collect();
                matching_banner_metadata(&matching_tags, event.metadata())
            }) // filter on the specific tags
            .map(|eh| (*eh).clone())
            .collect()
    }

    pub(crate) fn get_task(&self, task_name: &str) -> &TaskDefinition {
        self.tasks
            .iter()
            .find_map(|task| {
                if task.get_name() == task_name {
                    Some(task)
                } else {
                    None
                }
            })
            .unwrap()
    }
}

pub async fn build_and_validate_pipeline(
    code: &str,
) -> Result<Pipeline, Box<dyn Error + Send + Sync>> {
    let mut main_segment = code_to_ast(code);
    // try and gather all the errors in one place before returning them all.
    let mut errors: Vec<Box<dyn Error + Send + Sync>> = vec![];
    // we keep track of all imports in an attempt to prevent cyclic dependencies.
    // we'll also limit the depth we will travel at some point too.
    // TODO: limit the depth of the importing.
    let mut all_imports: HashSet<String> = HashSet::new();

    while main_segment.imports.len() > 0 {
        let imports = main_segment.imports.clone();

        // detect any cycles.
        let mut import_set: HashSet<String> =
            imports.iter().map(|import| import.uri.clone()).collect();
        let isc = import_set.clone();
        let cyclic_uris: HashSet<_> = all_imports.intersection(&isc).collect();
        // remove cycles from the imports and keep going
        if cyclic_uris.len() > 0 {
            cyclic_uris.into_iter().for_each(|uri| {
                import_set.remove(uri);
                errors.push(Box::new(CyclicImportError::new(format!(
                    "Tried to import {} more than once.",
                    uri
                ))));
            });
        }

        // ensure we keep track of the current imports.
        all_imports.extend(import_set.into_iter());
        main_segment.imports = vec![];

        let segments = get_segments(&imports).await;
        for segment in segments {
            match segment {
                Ok(segment) => {
                    main_segment = main_segment + segment;
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }
    }

    if errors.len() > 0 {
        let error = AggregatePipelineConstructionError::new(errors);
        return Err(Box::new(error));
    }

    let pipeline = ast_to_repr(main_segment);
    Ok(pipeline)
}

fn ast_to_repr(ast: ast::Pipeline) -> Pipeline {
    let tasks = ast
        .tasks
        .iter()
        .map(|task| {
            let task_def = TaskDefinition::from(task);

            task_def
        })
        .collect();

    // Must convert tasks, jobs and pipelines for now.
    // In future must also support free floating `on_event`
    let task_events: Vec<EventHandler> = ast
        .tasks
        .iter()
        .map(|task| {
            trace!(target: "task_log", "Getting event handlers for task: {}", task.name);
            let jobs: Vec<&JobSpecification> = ast
                .jobs
                .iter()
                .filter(|job| (*job).tasks.contains(&task.name))
                .collect();
            if jobs.len() > 0 {
                jobs.iter()
                    .map(|job| {
                        let pipelines: Vec<&PipelineSpecification> = ast
                            .pipelines
                            .iter()
                            .filter(|pipeline| (*pipeline).jobs.contains(&job.name))
                            .collect();
                        if pipelines.len() > 0 {
                            pipelines
                                .iter()
                                .map(|pipeline| {
                                    let veh: Vec<EventHandler> = get_eventhandlers_for_task(
                                        Some(*pipeline),
                                        Some(*job),
                                        &task,
                                    );
                                    veh
                                })
                                .flatten()
                                .collect()
                        } else {
                            let veh: Vec<EventHandler> =
                                get_eventhandlers_for_task(None, Some(*job), &task);
                            veh
                        }
                    })
                    .flatten()
                    .collect()
            } else {
                let veh: Vec<EventHandler> = get_eventhandlers_for_task(None, None, &task);
                veh
            }
        })
        .flatten()
        .collect();

    let job_events: Vec<EventHandler> = ast
        .jobs
        .iter()
        .map(|job| {
            let pipelines: Vec<&PipelineSpecification> = ast
                .pipelines
                .iter()
                .filter(|pipeline| pipeline.jobs.contains(&job.name))
                .collect();
            if pipelines.len() > 0 {
                pipelines
                    .iter()
                    .map(|pipeline| {
                        let veh: Vec<EventHandler> =
                            get_eventhandlers_for_job(Some(*pipeline), job);
                        veh
                    })
                    .flatten()
                    .collect::<Vec<EventHandler>>()
            } else {
                let veh: Vec<EventHandler> = get_eventhandlers_for_job(None, job);
                veh
            }
        })
        .flatten()
        .collect();
    let pipeline_events: Vec<EventHandler> = ast
        .pipelines
        .iter()
        .map(|pipeline| {
            let veh: Vec<EventHandler> = get_eventhandlers_for_pipeline(pipeline);
            veh
        })
        .flatten()
        .collect();
    let event_handlers = pipeline_events
        .into_iter()
        .chain(job_events.into_iter())
        .chain(task_events.into_iter())
        .collect();

    Pipeline {
        tasks,
        event_handlers,
    }
}

fn get_eventhandlers_for_task(
    pipeline: Option<&PipelineSpecification>,
    job: Option<&JobSpecification>,
    task: &ast::Task,
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

fn generate_execute_task_script(scope: &str, pipeline: &str, job: &str, task: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.execute_task_name_in_scope("{scope}", "{pipeline}", "{job}", "{task}").await;
        }}
        "###
    )
}

fn get_eventhandlers_for_pipeline(pipeline: &ast::PipelineSpecification) -> Vec<EventHandler> {
    // create all the event handlers specific to a defined pipeline. This requires:
    //   * an EH to trigger the first job in the pipeline
    //   * an EH per other job in the pipeline that triggers that job on the completion of the job preceeding it.
    //   * an EH to deal with the successful completion of the last job in the pipeline
    let mut event_handlers: Vec<EventHandler> = vec![];
    if pipeline.jobs.len() == 0 {
        let eh = create_start_empty_pipeline_event_handler(pipeline);
        event_handlers.push(eh);
        return event_handlers;
    }

    let mut iterator = pipeline.jobs.iter().rev();
    let mut some_job = iterator.next();
    while let Some(job) = some_job {
        // Create event handler to emit an event when the last job finishes.
        let eh = create_finished_pipeline_event_handler(pipeline, job);
        event_handlers.push(eh);

        // For every job we need to create:
        //   * an event handler to trigger on successful completion of the previous job.
        let next = iterator.clone().next();
        if next.is_some() {
            let eh = create_start_pipeline_job_event_handler(pipeline, job, next);
            event_handlers.push(eh);
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

fn create_start_empty_pipeline_event_handler(pipeline: &PipelineSpecification) -> EventHandler {
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

fn create_start_pipeline_event_handler(
    pipeline: &ast::PipelineSpecification,
    job: &String,
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
    let script = generate_start_pipeline_script(&pipeline.name, job);
    let eh = EventHandler::new(
        vec![pipeline_tag, description_tag],
        vec![listen_for_event],
        script,
    );
    eh
}

fn create_start_pipeline_job_event_handler(
    pipeline: &ast::PipelineSpecification,
    job: &String,
    next: Option<&String>,
) -> EventHandler {
    let job_tag = Metadata::new_banner_job(job);
    let pipeline_tag = Metadata::new_banner_pipeline(&pipeline.name);
    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of the job: {}/{}",
        &pipeline.name, &job
    ));
    let listen_for_event = Event::new(EventType::System(SystemEventType::Done(
        SystemEventScope::Job,
        SystemEventResult::Success,
    )))
    .with_pipeline_name(&pipeline.name)
    .with_job_name(next.unwrap())
    .build();
    let eh = EventHandler::new(
        vec![pipeline_tag, job_tag, description_tag],
        vec![listen_for_event],
        generate_start_job_script(job, &pipeline.name),
    );
    eh
}

fn create_finished_pipeline_event_handler(
    pipeline: &ast::PipelineSpecification,
    job: &String,
) -> EventHandler {
    let pipeline_tag = Metadata::new_banner_pipeline(&pipeline.name);
    let description_tag = Metadata::new_banner_description(&format!(
        "Signal the completion of the pipeline: {}; Last job was: {}",
        &pipeline.name, &job
    ));
    let listen_for_event = Event::new(EventType::System(SystemEventType::Done(
        SystemEventScope::Job,
        SystemEventResult::Success,
    )))
    .with_pipeline_name(&pipeline.name)
    .with_job_name(job)
    .build();
    let script = generate_finish_pipeline_script(&pipeline.name);
    EventHandler::new(
        vec![pipeline_tag, description_tag],
        vec![listen_for_event],
        script,
    )
}

fn generate_finish_pipeline_script(pipeline: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.trigger_pipeline("{pipeline}").await;
        }}
        "###
    )
}

fn generate_start_job_script(job: &str, pipeline: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.trigger_job("{pipeline}", "{job}").await;
        }}
        "###
    )
}

fn generate_start_pipeline_script(pipeline: &str, job_name: &str) -> String {
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

fn get_eventhandlers_for_job(
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
    while let Some(task) = some_task {
        // Create event handler to emit an event when the last task finishes.
        let eh = create_finished_job_event_handler(pipeline, job, task);
        event_handlers.push(eh);

        // For every job we need to create:
        //   * an event handler to trigger on successful completion of the previous task.
        let next = iterator.clone().next();
        if next.is_some() {
            let eh = create_start_task_event_handler(pipeline, job, task, next);
            event_handlers.push(eh);
        } else {
            // Create event handler to accept job triggers.  This event will only ever
            // emit another event which is a trigger to the first task(s) defined.
            let eh = create_start_job_event_handler(pipeline, &job.name, task);
            event_handlers.push(eh);
        }
        some_task = iterator.next();
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
    let listen_for_event = Event::new(EventType::System(SystemEventType::Done(
        SystemEventScope::Job,
        SystemEventResult::Success,
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

fn generate_job_with_no_tasks_script(pipeline: &str, job: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.log_message("Job [{pipeline}/{job}] has no tasks to trigger").await;
        }}
        "###
    )
}

// Create an event handler that triggers the start of the first task in the defined job
// when an event is raised that triggers the job.
fn create_start_job_event_handler(pipeline: &str, job: &str, task: &str) -> EventHandler {
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let description_tag = Metadata::new_banner_description(&format!(
        "Trigger the start of the job: {}/{}/{}",
        pipeline, job, task
    ));
    let listen_for_event = Event::new(EventType::System(SystemEventType::Trigger(
        SystemEventScope::Job,
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(job)
    .build();
    let script = generate_start_task_script(pipeline, job, task);
    let eh = EventHandler::new(
        vec![pipeline_tag, description_tag],
        vec![listen_for_event],
        script,
    );
    eh
}

fn generate_start_task_script(pipeline: &str, job: &str, task: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.trigger_task("{pipeline}", "{job}", "{task}").await;
        }}
        "###
    )
}

// Creaet an event handler to trigger on successful completion of the previous task.
fn create_start_task_event_handler(
    pipeline: &str,
    job: &ast::JobSpecification,
    task: &str,
    next: Option<&String>,
) -> EventHandler {
    let task_tag = Metadata::new_banner_task(task);
    let job_tag = Metadata::new_banner_job(&job.name);
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
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
    .with_task_name(next.unwrap())
    .build();
    let eh = EventHandler::new(
        vec![pipeline_tag, job_tag, task_tag, description_tag],
        vec![listen_for_event],
        generate_start_task_script(pipeline, &job.name, task),
    );
    eh
}

fn create_finished_job_event_handler(
    pipeline: &str,
    job: &ast::JobSpecification,
    task: &str,
) -> EventHandler {
    let pipeline_tag = Metadata::new_banner_pipeline(pipeline);
    let job_tag = Metadata::new_banner_job(&job.name);
    let description_tag = Metadata::new_banner_description(&format!(
        "Signal the completion of the job: {}/{}; Last task was: {}",
        pipeline, &job.name, task
    ));
    let listen_for_event = Event::new(EventType::System(SystemEventType::Done(
        SystemEventScope::Job,
        SystemEventResult::Success,
    )))
    .with_pipeline_name(pipeline)
    .with_job_name(&job.name)
    .build();
    let script = generate_finish_job_script(pipeline, &job.name);
    EventHandler::new(
        vec![pipeline_tag, job_tag, description_tag],
        vec![listen_for_event],
        script,
    )
}

fn generate_finish_job_script(pipeline: &str, job_name: &str) -> String {
    format!(
        r###"
        pub async fn main (engine) {{
            engine.job_success("{pipeline}", "{job_name}").await;
        }}
        "###
    )
}

async fn get_segments(
    imports: &Vec<Import>,
) -> Vec<Result<ast::Pipeline, Box<dyn Error + Send + Sync>>> {
    let mut results: Vec<Result<ast::Pipeline, Box<dyn Error + Send + Sync>>> = vec![];
    for import in imports {
        let uri = Iri::new(&import.uri).unwrap();
        match uri.scheme().as_str() {
            "file" => results.push(load_file(&uri)),
            "https" | "http" => results.push(load_url(&uri).await),
            "s3" => results.push(load_s3(&uri)),
            "git" => results.push(load_git(&uri)),
            _ => {
                let error = UnsupportedUriError::new(uri.to_string().clone());
                results.push(Err(Box::new(error)))
            }
        };
    }
    results
}

fn load_git(_uri: &Iri) -> Result<ast::Pipeline, Box<dyn Error + Send + Sync>> {
    todo!()
}

fn load_s3(_uri: &Iri) -> Result<ast::Pipeline, Box<dyn Error + Send + Sync>> {
    todo!()
}

async fn load_url<'a>(uri: &Iri<'a>) -> Result<ast::Pipeline, Box<dyn Error + Send + Sync>> {
    if uri.scheme().as_str() == "http" {
        let client = hyper::Client::builder().build::<_, hyper::Body>(HttpConnector::new());
        let resp = client.get(uri.as_str().parse()?).await?;
        let body = hyper::body::to_bytes(resp).await?;
        let code = std::str::from_utf8(&body).unwrap();

        try_code_to_ast(code, &uri)
    } else {
        let client = hyper::Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
        let resp = client.get(uri.as_str().parse()?).await?;
        let body = hyper::body::to_bytes(resp.into_body()).await?;
        let code = std::str::from_utf8(&body).unwrap();

        try_code_to_ast(code, &uri)
    }
}

fn load_file(uri: &Iri) -> Result<ast::Pipeline, Box<dyn Error + Send + Sync>> {
    let file = PathBuf::from(format!(
        "{}{}",
        uri.authority().unwrap(),
        uri.path().as_str()
    ));
    let pipeline = fs::read_to_string(&file).expect("Should have been able to read the file");
    try_code_to_ast(&pipeline, uri)
}

// This should be infallible.
// The pipelines that get passed in should already have undergone transformation and validation
fn code_to_ast(code: &str) -> ast::Pipeline {
    let mut parse_tree = BannerParser::parse(Rule::pipeline_definition, &code).unwrap();
    match ast::Pipeline::from_pest(&mut parse_tree) {
        Ok(tree) => tree,
        Err(e) => {
            trace!("ERROR = {:#?}", e);
            panic!("{:?}", e);
        }
    }
}

fn try_code_to_ast(code: &str, uri: &Iri) -> Result<ast::Pipeline, Box<dyn Error + Sync + Send>> {
    let mut parse_tree = BannerParser::parse(Rule::pipeline_definition, &code).unwrap();
    match ast::Pipeline::from_pest(&mut parse_tree) {
        Ok(tree) => Ok(tree),
        Err(e) => {
            trace!("ERROR parsing/ingesting URI: {uri}\n{:#?}", e);
            let error = CompositionError::new(uri);
            return Err(Box::new(error));
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CyclicImportError {
    description: String,
}

impl CyclicImportError {
    pub fn new(description: String) -> Self {
        Self { description }
    }

    pub fn description(&self) -> &str {
        self.description.as_ref()
    }
}

impl Display for CyclicImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for CyclicImportError {}

#[derive(Debug, Default, Clone)]
pub struct UnsupportedUriError {
    uri: String,
}

impl UnsupportedUriError {
    pub fn new(uri: String) -> Self {
        Self { uri }
    }

    pub fn uri(&self) -> &str {
        self.uri.as_ref()
    }
}

impl Display for UnsupportedUriError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Unsupported URI as import target: {}", self.uri())?;
        writeln!(
            f,
            "Supported URI schemes are 'file://', 'http://', 'https://', 's3://' and 'git://'."
        )?;
        writeln!(f, "If you would like support for additional schemes please raise an issue or supply a Pull Request to the Banner github repository.")?;
        Ok(())
    }
}

impl Error for UnsupportedUriError {}

#[derive(Debug, Default)]
pub struct AggregatePipelineConstructionError {
    errors: Vec<Box<dyn Error + Send + Sync>>,
}

impl AggregatePipelineConstructionError {
    pub fn new(errors: Vec<Box<dyn Error + Send + Sync>>) -> Self {
        Self { errors }
    }

    pub fn add_error(&mut self, error: Box<dyn Error + Send + Sync>) {
        self.errors.push(error)
    }
}

impl Display for AggregatePipelineConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for error in self.errors.iter() {
            writeln!(f, "{}", error)?;
        }
        Ok(())
    }
}

impl Error for AggregatePipelineConstructionError {}

#[derive(Debug, Default, Clone)]
pub struct CompositionError {
    uri: String,
}

impl CompositionError {
    pub fn new(uri: &Iri) -> Self {
        Self {
            uri: uri.as_str().to_string(),
        }
    }

    pub fn uri(&self) -> &str {
        self.uri.as_ref()
    }
}

impl Display for CompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error parsing uri: {}", self.uri())
    }
}

impl Error for CompositionError {}

#[cfg(test)]
mod build_pipeline_tests {
    use expect_test::{expect, Expect};
    use tracing_test::traced_test;

    use super::*;

    async fn check(code: &str, expect: Expect) {
        match build_and_validate_pipeline(code).await {
            Ok(ast) => {
                let actual = format!("{:#?}", ast);
                expect.assert_eq(&actual);
            }
            Err(e) => {
                println!("{:#?}", e);
                assert!(false)
            }
        }
    }

    #[traced_test]
    #[tokio::test]
    async fn can_parse_task_with_comment() {
        let code = r#######"
        // this task does the unit testing of the app
        task unit-test(image: rustl3rs/banner-rust-build:latest, execute: r#"/bin/bash -c"#) {
            r#####"bash
            echo testing, testing, 1, 2, 3!
            "#####
        }
        "#######;

        check(
            code,
            expect![[r#"
            Pipeline {
                tasks: [
                    TaskDefinition {
                        tags: [],
                        image: Image {
                            source: "rustl3rs/banner-rust-build:latest",
                            credentials: None,
                        },
                        command: [
                            "/bin/bash",
                            "-c",
                            "bash\n            echo testing, testing, 1, 2, 3!",
                        ],
                        inputs: [],
                        outputs: [],
                    },
                ],
                event_handlers: [],
            }"#]],
        )
        .await
    }

    // #[traced_test]
    // #[test]
    // fn can_parse_task_with_tag_attribute() {
    //     let code = r#######"
    //     [tag: banner.io/owner=me]
    //     [tag: banner.io/company=rustl3rs]
    //     task unit-test(image: rustl3rs/banner-rust-build, execute: r#"/bin/bash -c"#) {
    //         r#####"bash
    //         echo testing, testing, 1, 2, 3!
    //         "#####
    //     }
    //     "#######;

    //     check(code, expect![""])
    // }

    #[traced_test]
    #[tokio::test]
    async fn can_parse_uri() {
        let code = r#######"
        import file://../examples/single_task.ban
        import https://gist.githubusercontent.com/pms1969/464d6304014f9376be7e07b3ccf3a972/raw/a929e3feccd5384a00fe4e7ce6431f46dbb02951/cowsay.ban
        "#######;

        check(code, expect![[r#"
            Pipeline {
                tasks: [
                    TaskDefinition {
                        tags: [],
                        image: Image {
                            source: "alpine:latest",
                            credentials: None,
                        },
                        command: [
                            "sh",
                            "-c",
                            "bash\n    # this is a bash comment\n    echo rustl3rs herd!\n    # basically a no-op.\n    # But a good start to our testing.",
                        ],
                        inputs: [],
                        outputs: [],
                    },
                    TaskDefinition {
                        tags: [],
                        image: Image {
                            source: "kmcgivern/cowsay-alpine:latest",
                            credentials: None,
                        },
                        command: [
                            "",
                        ],
                        inputs: [],
                        outputs: [],
                    },
                ],
                event_handlers: [],
            }"#]]).await
    }

    #[traced_test]
    #[tokio::test]
    async fn can_parse_banner_pipeline() {
        let code = fs::read_to_string("../pipeline-assets/echo_task.ban")
            .expect("Should have been able to read the file");

        check(
            &code,
            expect![[r#"
            Pipeline {
                tasks: [
                    TaskDefinition {
                        tags: [],
                        image: Image {
                            source: "kmcgivern/cowsay-alpine:latest",
                            credentials: None,
                        },
                        command: [
                            "",
                        ],
                        inputs: [],
                        outputs: [],
                    },
                ],
                event_handlers: [],
            }"#]],
        )
        .await
    }
}

#[cfg(test)]
mod event_handler_creation_tests {
    use expect_test::{expect, Expect};
    use tracing_test::traced_test;

    use super::*;

    async fn check_pipeline(pipeline: ast::Pipeline, expect: Expect) {
        let actual = get_eventhandlers_for_pipeline(&pipeline.pipelines.first().unwrap());
        expect.assert_eq(&format!("{actual:?}"))
    }

    async fn check_job(pipeline: ast::Pipeline, expect: Expect) {
        let actual =
            get_eventhandlers_for_job(pipeline.pipelines.first(), &pipeline.jobs.first().unwrap());
        expect.assert_eq(&format!("{actual:?}"))
    }

    fn get_ast_for(code: &str) -> ast::Pipeline {
        code_to_ast(code)
    }

    #[traced_test]
    #[tokio::test]
    async fn test_multi_job_pipeline() {
        let ast = get_ast_for(
            r#"
            pipeline test [
                unit-test,
                build-artefacts,
                deploy-ci,
                deploy-qa,
                sit-test,
                deploy-prod,
            ]
            "#,
        );

        check_pipeline(ast, expect![[r#"[EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the pipeline: test; Last job was: deploy-prod," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "deploy-prod," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Pipeline,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(test)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "deploy-prod," }, Metadata { key: "banner.dev/description", value: "Trigger the start of the job: test/deploy-prod," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "sit-test," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Job,\n            )))\n            .with_pipeline_name(test)\n            .with_job_name(deploy-prod,)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the pipeline: test; Last job was: sit-test," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "sit-test," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Pipeline,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(test)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "sit-test," }, Metadata { key: "banner.dev/description", value: "Trigger the start of the job: test/sit-test," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "deploy-qa," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Job,\n            )))\n            .with_pipeline_name(test)\n            .with_job_name(sit-test,)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the pipeline: test; Last job was: deploy-qa," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "deploy-qa," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Pipeline,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(test)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "deploy-qa," }, Metadata { key: "banner.dev/description", value: "Trigger the start of the job: test/deploy-qa," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "deploy-ci," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Job,\n            )))\n            .with_pipeline_name(test)\n            .with_job_name(deploy-qa,)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the pipeline: test; Last job was: deploy-ci," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "deploy-ci," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Pipeline,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(test)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "deploy-ci," }, Metadata { key: "banner.dev/description", value: "Trigger the start of the job: test/deploy-ci," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "build-artefacts," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Job,\n            )))\n            .with_pipeline_name(test)\n            .with_job_name(deploy-ci,)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the pipeline: test; Last job was: build-artefacts," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "build-artefacts," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Pipeline,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(test)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "build-artefacts," }, Metadata { key: "banner.dev/description", value: "Trigger the start of the job: test/build-artefacts," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "unit-test," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Job,\n            )))\n            .with_pipeline_name(test)\n            .with_job_name(build-artefacts,)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the pipeline: test; Last job was: unit-test," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "unit-test," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Pipeline,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(test)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Trigger the start of the pipeline: test/unit-test," }], listen_for_events: [Event { type: System(Trigger(Pipeline)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Job,\n            )))\n            .with_pipeline_name(test)\n            .with_job_name(unit-test,)\n            .send_from(tx);\n        }\n        " }]"#]]).await;
    }

    #[traced_test]
    #[tokio::test]
    async fn test_single_job_pipeline() {
        let ast = get_ast_for(
            r#"
            pipeline test [
                unit-test,
            ]
            "#,
        );

        check_pipeline(ast, expect![[r#"[EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the pipeline: test; Last job was: unit-test," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "unit-test," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Pipeline,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(test)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Trigger the start of the pipeline: test/unit-test," }], listen_for_events: [Event { type: System(Trigger(Pipeline)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Job,\n            )))\n            .with_pipeline_name(test)\n            .with_job_name(unit-test,)\n            .send_from(tx);\n        }\n        " }]"#]]).await;
    }

    #[traced_test]
    #[tokio::test]
    async fn test_empty_pipeline() {
        let ast = get_ast_for(
            r#"
            pipeline test [
            ]
            "#,
        );

        check_pipeline(ast, expect![[r#"[EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Trigger the start of the pipeline: test" }], listen_for_events: [Event { type: System(Trigger(Pipeline)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::Log)\n            .with_log_message(\"Pipeline [test] has no jobs to trigger\".to_string())\n            .send_from(tx);\n        }\n        " }]"#]]).await;
    }

    #[traced_test]
    #[tokio::test]
    async fn test_two_job_pipeline() {
        let ast = get_ast_for(
            r#"
            pipeline test [
                unit-test,
                build-artefacts,
            ]
            "#,
        );

        check_pipeline(ast, expect![[r#"[EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the pipeline: test; Last job was: build-artefacts," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "build-artefacts," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Pipeline,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(test)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "build-artefacts," }, Metadata { key: "banner.dev/description", value: "Trigger the start of the job: test/build-artefacts," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "unit-test," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Job,\n            )))\n            .with_pipeline_name(test)\n            .with_job_name(build-artefacts,)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the pipeline: test; Last job was: unit-test," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/job", value: "unit-test," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Pipeline,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(test)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "test" }, Metadata { key: "banner.dev/description", value: "Trigger the start of the pipeline: test/unit-test," }], listen_for_events: [Event { type: System(Trigger(Pipeline)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "test" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Job,\n            )))\n            .with_pipeline_name(test)\n            .with_job_name(unit-test,)\n            .send_from(tx);\n        }\n        " }]"#]]).await;
    }

    #[traced_test]
    #[tokio::test]
    async fn test_empty_job() {
        let ast = get_ast_for(
            r#"
                job build [
                ]
                "#,
        );

        check_job(ast, expect![[r#"[EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }, Metadata { key: "banner.dev/description", value: "Trigger the start of empty job: _/build" }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::Log)\n            .with_log_message(\"Job [_/build] has no tasks to trigger\".to_string())\n            .send_from(tx);\n        }\n        " }]"#]]).await;
    }

    #[traced_test]
    #[tokio::test]
    async fn test_job_with_single_task() {
        let ast = get_ast_for(
            r#"
                    job build [
                        unit-test,
                    ]
                    "#,
        );

        check_job(ast, expect![[r#"[EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the job: _/build; Last task was: unit-test," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Job,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(_)\n            .with_job_name(build)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/description", value: "Trigger the start of the job: _/build/unit-test," }], listen_for_events: [Event { type: System(Trigger(Job)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Task,\n            )))\n            .with_pipeline_name(_)\n            .with_job_name(build)\n            .with_task_name(unit-test,)\n            .send_from(tx);\n        }\n        " }]"#]]).await;
    }

    #[traced_test]
    #[tokio::test]
    async fn test_job_with_multiple_tasks() {
        let ast = get_ast_for(
            r#"
                        job build [
                            unit-test,
                            build-docker,
                            publish-docker,
                        ]
                        "#,
        );

        check_job(ast, expect![[r#"[EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the job: _/build; Last task was: publish-docker," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Job,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(_)\n            .with_job_name(build)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }, Metadata { key: "banner.dev/task", value: "publish-docker," }, Metadata { key: "banner.dev/description", value: "Trigger the start of the task: _/build/publish-docker," }], listen_for_events: [Event { type: System(Done(Task, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }, Metadata { key: "banner.dev/task", value: "build-docker," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Task,\n            )))\n            .with_pipeline_name(_)\n            .with_job_name(build)\n            .with_task_name(publish-docker,)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the job: _/build; Last task was: build-docker," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Job,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(_)\n            .with_job_name(build)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }, Metadata { key: "banner.dev/task", value: "build-docker," }, Metadata { key: "banner.dev/description", value: "Trigger the start of the task: _/build/build-docker," }], listen_for_events: [Event { type: System(Done(Task, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }, Metadata { key: "banner.dev/task", value: "unit-test," }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Task,\n            )))\n            .with_pipeline_name(_)\n            .with_job_name(build)\n            .with_task_name(build-docker,)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }, Metadata { key: "banner.dev/description", value: "Signal the completion of the job: _/build; Last task was: unit-test," }], listen_for_events: [Event { type: System(Done(Job, Success)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Done(\n                SystemEventScope::Job,\n                SystemEventResult::Success,\n            )))\n            .with_pipeline_name(_)\n            .with_job_name(build)\n            .send_from(tx);\n        }\n        " }, EventHandler { tags: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/description", value: "Trigger the start of the job: _/build/unit-test," }], listen_for_events: [Event { type: System(Trigger(Job)), time_emitted: 0, metadata: [Metadata { key: "banner.dev/pipeline", value: "_" }, Metadata { key: "banner.dev/job", value: "build" }] }], script: "\n        pub fn main () {\n            Event::new(EventType::System(SystemEventType::Trigger(\n                SystemEventScope::Task,\n            )))\n            .with_pipeline_name(_)\n            .with_job_name(build)\n            .with_task_name(unit-test,)\n            .send_from(tx);\n        }\n        " }]"#]]).await;
    }
}
