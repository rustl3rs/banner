use std::{collections::HashSet, error::Error, fmt::Display, fs, path::PathBuf};

use banner_parser::{
    ast::{
        self, IdentifierListItem, IdentifierMarker, Import, JobSpecification,
        PipelineSpecification, TaskSpecification,
    },
    grammar::{BannerParser, Rule},
    image_ref::{self, ImageRefParser},
    FromPest, Iri, Parser,
};
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use log::trace;

use crate::{
    event_handler::EventHandler,
    event_handlers::{
        get_eventhandlers_for_job, get_eventhandlers_for_pipeline,
        get_eventhandlers_for_task_definition,
    },
    listen_for_events::matching_banner_metadata,
    pragma::{Pragma, PragmasBuilder},
    Event, Metadata, MountPoint, Tag, TaskDefinition, TaskResource, MATCHING_TAG,
};

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub tasks: Vec<TaskDefinition>,
    pub event_handlers: Vec<EventHandler>,
    pub pragmas: Vec<Pragma>,
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
}

pub async fn build_and_validate_pipeline(
    code: &str,
    pragma_builder: PragmasBuilder,
) -> Result<(Pipeline, Vec<PipelineSpecification>), Box<dyn Error + Send + Sync>> {
    let mut main_segment = code_to_ast(code);
    // try and gather all the errors in one place before returning them all.
    let mut errors: Vec<Box<dyn Error + Send + Sync>> = vec![];
    // we keep track of all imports in an attempt to prevent cyclic dependencies.
    // we'll also limit the depth we will travel at some point too.
    // TODO: limit the depth of the importing.
    let mut all_imports: HashSet<String> = HashSet::new();

    while !main_segment.imports.is_empty() {
        let imports = main_segment.imports.clone();

        // detect any cycles.
        let mut import_set: HashSet<String> =
            imports.iter().map(|import| import.uri.clone()).collect();
        let isc = import_set.clone();
        let cyclic_uris: HashSet<_> = all_imports.intersection(&isc).collect();
        // remove cycles from the imports and keep going
        if !cyclic_uris.is_empty() {
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

    if !errors.is_empty() {
        let error = AggregatePipelineConstructionError::new(errors);
        return Err(Box::new(error));
    }

    post_process(&mut main_segment)?;
    let specifications = main_segment.pipelines.clone();
    let pipeline = ast_to_repr(main_segment, pragma_builder);
    Ok((pipeline, specifications))
}

fn post_process(ast: &mut ast::Pipeline) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 1. validate the docker image references in task definitions and add the `latest` tag to any image ref
    //    that doesn't have an explicit tag or digest
    for task in ast.tasks.iter_mut() {
        if task.image.starts_with("${") {
            continue;
        }

        let mut tree = ImageRefParser::parse(image_ref::Rule::reference, &task.image)?;
        let image_ref = match image_ref::ImageRef::from_pest(&mut tree) {
            Ok(tree) => tree,
            Err(e) => {
                trace!("ERROR = {:#?}", e);
                panic!(
                    r#"Failed to parse image reference "{}", {:?}"#,
                    task.image, e
                );
            }
        };
        match (image_ref.tag, image_ref.digest) {
            (None, None) => task.image = format!("{}:latest", task.image), // no tag or digest, so add latest
            (None, Some(_)) | (Some(_), None) | (Some(_), Some(_)) => {} // do nothing, there is already a tag or digest associated to the image.
        };
    }

    // 2. validate the docker references in image definitions and add the `latest` tag to any image ref
    //    that doesn't have an explicit tag or digest
    for image_def in ast.images.iter_mut() {
        let mut tree = ImageRefParser::parse(image_ref::Rule::reference, &image_def.image.name)?;
        let image_ref = match image_ref::ImageRef::from_pest(&mut tree) {
            Ok(tree) => tree,
            Err(e) => {
                trace!("ERROR = {:#?}", e);
                panic!(
                    r#"Failed to parse image reference "{:?}", {:?}"#,
                    image_def.image, e
                );
            }
        };
        match (image_ref.tag, image_ref.digest) {
            (None, None) => image_def.image.name = format!("{}:latest", image_def.image.name), // no tag or digest, so add latest
            (None, Some(_)) | (Some(_), None) | (Some(_), Some(_)) => {} // do nothing, there is already a tag or digest associated to the image.
        };
    }

    // 3. Create a job for any job macro defined in the pipeline directive
    // After that is done, remove any reference to a IdentifierWithMarkers from the pipelines list of jobs.
    for pipeline in ast.pipelines.iter() {
        for job in pipeline.iter_jobs() {
            match job {
                IdentifierListItem::Identifier(job, markers) => {
                    if markers.contains(&IdentifierMarker::JobMacro) {
                        let job_spec = JobSpecification {
                            name: job.clone(),
                            tasks: vec![IdentifierListItem::Identifier(job.clone(), vec![])],
                        };
                        ast.jobs.push(job_spec);
                    }
                }
                _ => (),
            }
        }
    }

    // 4. annotate all tasks with their task, job and pipeline names
    // A task can be reused in multiple jobs, so we need to create a new task definition for each
    // task described in the job descriptions and annotate them with the job and pipeline names.
    // While this is particularly wasteful on memory, it makes it easier to reason about the
    // tasks and their relationships to jobs and pipelines. It also allows for diversions in the
    // future where we might want to add additional information to the task definitions, or allow for
    // "inheritance" of tasks from other tasks. I can imagine a scenario where multiple tasks are
    // essentially the same, but with minor differences. Rather than copy and paste the task, we
    // can simply create a new task that inherits from the original task and then override the
    // differences.
    // This is a tradeoff between memory and flexibility.
    let bare_tasks = ast.tasks.clone();
    let mut unused_tasks = ast.tasks.clone();
    let mut annotated_tasks: Vec<TaskSpecification> = vec![];
    for job in ast.jobs.iter() {
        // println!("--------> ANNOTATE job = {:#?}", job.name);
        for task in job.all_tasks().into_iter() {
            let bare_task = bare_tasks
                .iter()
                .find(|t| t.name == task)
                .expect("Failed to find task");
            let mut annotated_task = bare_task.clone();
            unused_tasks.retain(|t| t != bare_task);

            // add the task tag
            let task_tag = Tag::new_banner_task(&task);
            annotated_task.tags.push(ast::Tag {
                key: task_tag.key().to_string(),
                value: task_tag.value().to_string(),
            });

            // add the job tag
            let job_tag = Tag::new_banner_job(&job.name);
            annotated_task.tags.push(ast::Tag {
                key: job_tag.key().to_string(),
                value: job_tag.value().to_string(),
            });

            let pipeline_name = ast.pipelines.iter().find_map(|pipeline| {
                match ident_list_contains_item(&pipeline.jobs, &job.name) {
                    true => Some(&pipeline.name),
                    false => None,
                }
            });

            // add the pipeline tag
            let pipeline_name = match pipeline_name {
                Some(pipeline) => pipeline,
                None => "_",
            };
            let pipeline_tag = Tag::new_banner_pipeline(pipeline_name);
            annotated_task.tags.push(ast::Tag {
                key: pipeline_tag.key().to_string(),
                value: pipeline_tag.value().to_string(),
            });

            annotated_tasks.push(annotated_task);
        }
    }
    ast.tasks = annotated_tasks;
    ast.tasks.extend(unused_tasks);

    Ok(())
}

fn ident_list_contains_item(list: &[IdentifierListItem], item: &str) -> bool {
    for ident in list.iter() {
        match ident {
            IdentifierListItem::Identifier(id, _) => {
                if id == item {
                    return true;
                }
            }
            IdentifierListItem::SequentialList(list) => {
                if ident_list_contains_item(list, item) {
                    return true;
                }
            }
            IdentifierListItem::ParallelList(list) => {
                if ident_list_contains_item(list, item) {
                    return true;
                }
            }
        }
    }
    false
}

fn ast_to_repr(ast: ast::Pipeline, pragma_builder: PragmasBuilder) -> Pipeline {
    let pragmas = pragma_builder.build_from(&ast.pragmas);

    let tasks: Vec<TaskDefinition> = ast
        .tasks
        .iter()
        .map(|task| {
            let mut task_def = TaskDefinition::from(task);
            tracing::trace!("task = {:?}", task);
            if task.image.contains("${") {
                // do variable substitution on images
                let replacement = ast
                    .images
                    .iter()
                    .find(|image| format!("${{{}}}", image.name) == task.image)
                    .unwrap();
                // replace the image
                task_def.set_image(replacement.image.clone().into());
                // add in all env vars defined on the image ref
                replacement.image.envs.iter().for_each(|env| {
                    let env = TaskResource::EnvVar(env.key.clone(), env.value.as_str().to_string());
                    task_def.append_inputs(env);
                });
                // add all the volumes defined on the image ref
                replacement.image.mounts.iter().for_each(|volume| {
                    tracing::trace!("volume = {:#?}", volume);
                    let volume = TaskResource::Mount(MountPoint {
                        host_path: match volume.source.clone() {
                            ast::MountSource::EngineSupplied(dir) => {
                                crate::HostPath::EngineInit(dir)
                            }
                            ast::MountSource::Identifier(dir) => {
                                crate::HostPath::EngineFromTask(dir.clone())
                            }
                            ast::MountSource::StringLiteral(dir) => {
                                crate::HostPath::Path(dir.clone())
                            }
                        },
                        container_path: volume.destination.as_str().to_string(),
                    });
                    task_def.append_inputs(volume);
                });
            }
            task_def
        })
        .collect();

    // Must convert tasks, jobs and pipelines for now.
    // In future must also support free floating `on_event`
    let task_events: Vec<EventHandler> = tasks
        .iter()
        .flat_map(get_eventhandlers_for_task_definition)
        .collect();

    let job_events: Vec<EventHandler> = ast
        .jobs
        .iter()
        .flat_map(|job| {
            let pipelines: Vec<&PipelineSpecification> = ast
                .pipelines
                .iter()
                .filter(|pipeline| ident_list_contains_item(&pipeline.jobs, &job.name))
                .collect();
            if !pipelines.is_empty() {
                pipelines
                    .iter()
                    .flat_map(|pipeline| {
                        let veh: Vec<EventHandler> =
                            get_eventhandlers_for_job(Some(*pipeline), job);
                        veh
                    })
                    .collect::<Vec<EventHandler>>()
            } else {
                let veh: Vec<EventHandler> = get_eventhandlers_for_job(None, job);
                veh
            }
        })
        .collect();

    let pipeline_events: Vec<EventHandler> = ast
        .pipelines
        .iter()
        .flat_map(|pipeline| {
            let veh: Vec<EventHandler> = get_eventhandlers_for_pipeline(pipeline);
            veh
        })
        .collect();

    let event_handlers = pipeline_events
        .into_iter()
        .chain(job_events)
        .chain(task_events)
        .collect();

    Pipeline {
        tasks,
        event_handlers,
        pragmas,
    }
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

        try_code_to_ast(code, uri)
    } else {
        let client = hyper::Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
        let resp = client.get(uri.as_str().parse()?).await?;
        let body = hyper::body::to_bytes(resp.into_body()).await?;
        let code = std::str::from_utf8(&body).unwrap();

        try_code_to_ast(code, uri)
    }
}

fn load_file(uri: &Iri) -> Result<ast::Pipeline, Box<dyn Error + Send + Sync>> {
    let file = PathBuf::from(format!(
        "{}{}",
        uri.authority().unwrap(),
        uri.path().as_str()
    ));
    let pipeline = fs::read_to_string(file).expect("Should have been able to read the file");
    try_code_to_ast(&pipeline, uri)
}

// This should be infallible.
// The pipelines that get passed in should already have undergone transformation and validation
fn code_to_ast(code: &str) -> ast::Pipeline {
    let parsed = BannerParser::parse(Rule::pipeline_definition, code);
    tracing::trace!("Code tree: {:#?}", parsed);
    match parsed {
        Ok(mut parse_tree) => match ast::Pipeline::from_pest(&mut parse_tree) {
            Ok(tree) => tree,
            Err(e) => {
                println!("{:#?}", e);
                panic!("Creating the AST failed");
            }
        },
        Err(e) => {
            println!("{:#?}", e);
            panic!("Parsing of the pipeline failed");
        }
    }
}

fn try_code_to_ast(code: &str, uri: &Iri) -> Result<ast::Pipeline, Box<dyn Error + Sync + Send>> {
    let mut parse_tree = BannerParser::parse(Rule::pipeline_definition, code).unwrap();
    match ast::Pipeline::from_pest(&mut parse_tree) {
        Ok(tree) => Ok(tree),
        Err(e) => {
            trace!("ERROR parsing/ingesting URI: {uri}\n{:#?}", e);
            let error = CompositionError::new(uri);
            Err(Box::new(error))
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
        match build_and_validate_pipeline(code, PragmasBuilder::new()).await {
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
            expect![[r####"
                (
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
                                    "bash\n            echo testing, testing, 1, 2, 3!\n            ",
                                ],
                                inputs: [],
                                outputs: [],
                            },
                        ],
                        event_handlers: [
                            {
                                listen_for_events:
                                    ListenForEvent { type: System(Only(Trigger(Only(Task)))), metadata: [banner.dev/pipeline: _, banner.dev/job: _, banner.dev/task: _] },
                                tags:
                                    banner.dev/description: Execute the task: _,
                                script: ###"
                                    pub async fn main (engine, event) {
                                        engine.execute_task_name_in_scope("", "_", "_", "_").await;
                                    }
                                "###
                            },
                        ],
                        pragmas: [],
                    },
                    [],
                )"####]],
        )
        .await
    }

    #[traced_test]
    #[tokio::test]
    async fn can_parse_uri() {
        let code = r#######"
        import file://../examples/single_task.ban
        import https://gist.githubusercontent.com/pms1969/464d6304014f9376be7e07b3ccf3a972/raw/a929e3feccd5384a00fe4e7ce6431f46dbb02951/cowsay.ban
        "#######;

        check(code, expect![[r####"
            (
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
                                "bash\n    # this is a bash comment\n    echo rustl3rs herd!\n    # basically a no-op.\n    # But a good start to our testing.\n    ",
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
                    event_handlers: [
                        {
                            listen_for_events:
                                ListenForEvent { type: System(Only(Trigger(Only(Task)))), metadata: [banner.dev/pipeline: _, banner.dev/job: _, banner.dev/task: _] },
                            tags:
                                banner.dev/description: Execute the task: _,
                            script: ###"
                                pub async fn main (engine, event) {
                                    engine.execute_task_name_in_scope("", "_", "_", "_").await;
                                }
                            "###
                        },
                        {
                            listen_for_events:
                                ListenForEvent { type: System(Only(Trigger(Only(Task)))), metadata: [banner.dev/pipeline: _, banner.dev/job: _, banner.dev/task: _] },
                            tags:
                                banner.dev/description: Execute the task: _,
                            script: ###"
                                pub async fn main (engine, event) {
                                    engine.execute_task_name_in_scope("", "_", "_", "_").await;
                                }
                            "###
                        },
                    ],
                    pragmas: [],
                },
                [],
            )"####]]).await
    }

    #[traced_test]
    #[tokio::test]
    async fn can_parse_banner_pipeline() {
        let code = fs::read_to_string("../test-pipelines/echo_task.ban")
            .expect("Should have been able to read the file");

        check(
            &code,
            expect![[r####"
                (
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
                        event_handlers: [
                            {
                                listen_for_events:
                                    ListenForEvent { type: System(Only(Trigger(Only(Task)))), metadata: [banner.dev/pipeline: _, banner.dev/job: _, banner.dev/task: _] },
                                tags:
                                    banner.dev/description: Execute the task: _,
                                script: ###"
                                    pub async fn main (engine, event) {
                                        engine.execute_task_name_in_scope("", "_", "_", "_").await;
                                    }
                                "###
                            },
                        ],
                        pragmas: [],
                    },
                    [],
                )"####]],
        )
        .await
    }

    #[traced_test]
    #[tokio::test]
    async fn can_parse_job_macro() {
        let code = r#######"
                task unit-test(image: alpine, execute: "/bin/sh -c") {
                    r#"
                    // this is a comment
                    echo -n "testing, testing, 1, 2, 3!"
                    "#
                }
                
                pipeline test [
                    unit-test!,
                ]
            "#######;

        check(&code, expect![[r####"
            (
                Pipeline {
                    tasks: [
                        TaskDefinition {
                            tags: [
                                banner.dev/task: unit-test,
                                banner.dev/job: unit-test,
                                banner.dev/pipeline: test,
                            ],
                            image: Image {
                                source: "alpine:latest",
                                credentials: None,
                            },
                            command: [
                                "/bin/sh",
                                "-c",
                                "\n                    // this is a comment\n                    echo -n \"testing, testing, 1, 2, 3!\"\n                    ",
                            ],
                            inputs: [],
                            outputs: [],
                        },
                    ],
                    event_handlers: [
                        {
                            listen_for_events:
                                ListenForEvent { type: System(Only(Trigger(Only(Pipeline)))), metadata: [banner.dev/pipeline: test] },
                            tags:
                                banner.dev/pipeline: test,
                                banner.dev/description: Trigger the start of the pipeline: test/unit-test!,
                            script: ###"
                                pub async fn main (engine, event) {
                                    engine.trigger_job("test", "unit-test").await;
                                }
                            "###
                        },
                        {
                            listen_for_events:
                                ListenForEvent { type: System(Only(Done(Only(Job), Any))), metadata: [banner.dev/pipeline: test, banner.dev/job: unit-test] },
                            tags:
                                banner.dev/pipeline: test,
                                banner.dev/description: Signal the completion of the pipeline: test; Last job was: unit-test!,
                            script: ###"
                                pub async fn main (engine, event) {
                                    engine.pipeline_complete(event).await;
                                }
                            "###
                        },
                        {
                            listen_for_events:
                                ListenForEvent { type: System(Only(Done(Only(Task), Any))), metadata: [banner.dev/pipeline: test, banner.dev/job: unit-test, banner.dev/task: unit-test] },
                            tags:
                                banner.dev/pipeline: test,
                                banner.dev/job: unit-test,
                                banner.dev/description: Signal the completion of the job: test/unit-test; Last task was: unit-test,
                            script: ###"
                                pub async fn main (engine, event) {
                                    engine.job_complete(event).await;
                                }
                            "###
                        },
                        {
                            listen_for_events:
                                ListenForEvent { type: System(Only(Trigger(Only(Job)))), metadata: [banner.dev/pipeline: test, banner.dev/job: unit-test] },
                            tags:
                                banner.dev/pipeline: test,
                                banner.dev/job: unit-test,
                                banner.dev/job: unit-test,
                                banner.dev/description: Trigger the start of the job: test/unit-test/unit-test,
                            script: ###"
                                pub async fn main (engine, event) {
                                    engine.trigger_task("test", "unit-test", "unit-test").await;
                                }
                            "###
                        },
                        {
                            listen_for_events:
                                ListenForEvent { type: System(Only(Trigger(Only(Task)))), metadata: [banner.dev/pipeline: test, banner.dev/job: unit-test, banner.dev/task: unit-test] },
                            tags:
                                banner.dev/task: unit-test,
                                banner.dev/job: unit-test,
                                banner.dev/pipeline: test,
                                banner.dev/description: Execute the task: unit-test,
                            script: ###"
                                pub async fn main (engine, event) {
                                    engine.execute_task_name_in_scope("", "test", "unit-test", "unit-test").await;
                                }
                            "###
                        },
                    ],
                    pragmas: [],
                },
                [
                    PipelineSpecification {
                        name: "test",
                        jobs: [
                            Identifier(
                                "unit-test",
                                [
                                    JobMacro,
                                ],
                            ),
                        ],
                    },
                ],
            )"####]]).await
    }
}

#[cfg(test)]
mod event_handler_creation_tests {
    use expect_test::{expect, Expect};
    use tracing_test::traced_test;

    use super::*;

    async fn check_all(pipeline: ast::Pipeline, expect: Expect) {
        let actual = ast_to_repr(pipeline, PragmasBuilder::new());
        expect.assert_eq(&format!("{actual:?}"))
    }

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
        let ast = code_to_ast(code);
        tracing::debug!("AST: {:#?}", ast);
        ast
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

        check_pipeline(ast, expect![[r####"
            [{
                listen_for_events:
                    ListenForEvent { type: System(Only(Trigger(Only(Pipeline)))), metadata: [banner.dev/pipeline: test] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/description: Trigger the start of the pipeline: test/unit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test", "unit-test").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Any))), metadata: [banner.dev/pipeline: test, banner.dev/job: deploy-prod] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/description: Signal the completion of the pipeline: test; Last job was: deploy-prod,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.pipeline_complete(event).await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Only(Success)))), metadata: [banner.dev/pipeline: test, banner.dev/job: unit-test] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/job: build-artefacts,
                    banner.dev/description: Trigger the start of the single job: test/build-artefacts,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test", "build-artefacts").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Only(Success)))), metadata: [banner.dev/pipeline: test, banner.dev/job: build-artefacts] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/job: deploy-ci,
                    banner.dev/description: Trigger the start of the single job: test/deploy-ci,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test", "deploy-ci").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Only(Success)))), metadata: [banner.dev/pipeline: test, banner.dev/job: deploy-ci] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/job: deploy-qa,
                    banner.dev/description: Trigger the start of the single job: test/deploy-qa,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test", "deploy-qa").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Only(Success)))), metadata: [banner.dev/pipeline: test, banner.dev/job: deploy-qa] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/job: sit-test,
                    banner.dev/description: Trigger the start of the single job: test/sit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test", "sit-test").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Only(Success)))), metadata: [banner.dev/pipeline: test, banner.dev/job: sit-test] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/job: deploy-prod,
                    banner.dev/description: Trigger the start of the single job: test/deploy-prod,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test", "deploy-prod").await;
                    }
                "###
            }]"####]]).await;
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

        check_pipeline(ast, expect![[r####"
            [{
                listen_for_events:
                    ListenForEvent { type: System(Only(Trigger(Only(Pipeline)))), metadata: [banner.dev/pipeline: test] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/description: Trigger the start of the pipeline: test/unit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test", "unit-test").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Any))), metadata: [banner.dev/pipeline: test, banner.dev/job: unit-test] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/description: Signal the completion of the pipeline: test; Last job was: unit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.pipeline_complete(event).await;
                    }
                "###
            }]"####]]).await;
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

        check_pipeline(ast, expect![[r####"
            [{
                listen_for_events:
                    ListenForEvent { type: System(Only(Trigger(Only(Pipeline)))), metadata: [banner.dev/pipeline: test] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/description: Trigger the start of the pipeline: test/unit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test", "unit-test").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Any))), metadata: [banner.dev/pipeline: test, banner.dev/job: build-artefacts] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/description: Signal the completion of the pipeline: test; Last job was: build-artefacts,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.pipeline_complete(event).await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Only(Success)))), metadata: [banner.dev/pipeline: test, banner.dev/job: unit-test] },
                tags:
                    banner.dev/pipeline: test,
                    banner.dev/job: build-artefacts,
                    banner.dev/description: Trigger the start of the single job: test/build-artefacts,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test", "build-artefacts").await;
                    }
                "###
            }]"####]]).await;
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

        check_job(ast, expect![[r####"
            [{
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Task), Any))), metadata: [banner.dev/pipeline: _, banner.dev/job: build, banner.dev/task: unit-test] },
                tags:
                    banner.dev/pipeline: _,
                    banner.dev/job: build,
                    banner.dev/description: Signal the completion of the job: _/build; Last task was: unit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.job_complete(event).await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Trigger(Only(Job)))), metadata: [banner.dev/pipeline: _, banner.dev/job: build] },
                tags:
                    banner.dev/pipeline: _,
                    banner.dev/job: build,
                    banner.dev/job: unit-test,
                    banner.dev/description: Trigger the start of the job: _/build/unit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_task("_", "build", "unit-test").await;
                    }
                "###
            }]"####]]).await;
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

        check_job(ast, expect![[r####"
            [{
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Task), Any))), metadata: [banner.dev/pipeline: _, banner.dev/job: build, banner.dev/task: publish-docker] },
                tags:
                    banner.dev/pipeline: _,
                    banner.dev/job: build,
                    banner.dev/description: Signal the completion of the job: _/build; Last task was: publish-docker,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.job_complete(event).await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Task), Only(Success)))), metadata: [banner.dev/pipeline: _, banner.dev/job: build, banner.dev/task: build-docker] },
                tags:
                    banner.dev/pipeline: _,
                    banner.dev/job: build,
                    banner.dev/task: publish-docker,
                    banner.dev/description: Trigger the start of the task: _/build/publish-docker,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_task("_", "build", "publish-docker").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Task), Only(Success)))), metadata: [banner.dev/pipeline: _, banner.dev/job: build, banner.dev/task: unit-test] },
                tags:
                    banner.dev/pipeline: _,
                    banner.dev/job: build,
                    banner.dev/task: build-docker,
                    banner.dev/description: Trigger the start of the task: _/build/build-docker,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_task("_", "build", "build-docker").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Trigger(Only(Job)))), metadata: [banner.dev/pipeline: _, banner.dev/job: build] },
                tags:
                    banner.dev/pipeline: _,
                    banner.dev/job: build,
                    banner.dev/job: unit-test,
                    banner.dev/description: Trigger the start of the job: _/build/unit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_task("_", "build", "unit-test").await;
                    }
                "###
            }]"####]]).await;
    }

    #[traced_test]
    #[tokio::test]
    async fn test_simple_pipeline_with_job_and_task() {
        let ast = get_ast_for(
            r##"
            task unit-test(image: alpine:latest, execute: r#"sh -c"#) {
                r#"bash
                # this is a bash comment
                echo rustl3rs herd!
                # basically a no-op.
                # But a good start to our testing.
                "#
            }

            job build [
                unit-test,
            ]

            pipeline test_simple_pipeline_with_job_and_task [
                build,
            ]
            "##,
        );

        check_all(ast, expect![[r####"
            Pipeline { tasks: [TaskDefinition { tags: [], image: Image { source: "alpine:latest", credentials: None }, command: ["sh", "-c", "bash\n                # this is a bash comment\n                echo rustl3rs herd!\n                # basically a no-op.\n                # But a good start to our testing.\n                "], inputs: [], outputs: [] }], event_handlers: [{
                listen_for_events:
                    ListenForEvent { type: System(Only(Trigger(Only(Pipeline)))), metadata: [banner.dev/pipeline: test_simple_pipeline_with_job_and_task] },
                tags:
                    banner.dev/pipeline: test_simple_pipeline_with_job_and_task,
                    banner.dev/description: Trigger the start of the pipeline: test_simple_pipeline_with_job_and_task/build,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_job("test_simple_pipeline_with_job_and_task", "build").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Job), Any))), metadata: [banner.dev/pipeline: test_simple_pipeline_with_job_and_task, banner.dev/job: build] },
                tags:
                    banner.dev/pipeline: test_simple_pipeline_with_job_and_task,
                    banner.dev/description: Signal the completion of the pipeline: test_simple_pipeline_with_job_and_task; Last job was: build,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.pipeline_complete(event).await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Done(Only(Task), Any))), metadata: [banner.dev/pipeline: test_simple_pipeline_with_job_and_task, banner.dev/job: build, banner.dev/task: unit-test] },
                tags:
                    banner.dev/pipeline: test_simple_pipeline_with_job_and_task,
                    banner.dev/job: build,
                    banner.dev/description: Signal the completion of the job: test_simple_pipeline_with_job_and_task/build; Last task was: unit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.job_complete(event).await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Trigger(Only(Job)))), metadata: [banner.dev/pipeline: test_simple_pipeline_with_job_and_task, banner.dev/job: build] },
                tags:
                    banner.dev/pipeline: test_simple_pipeline_with_job_and_task,
                    banner.dev/job: build,
                    banner.dev/job: unit-test,
                    banner.dev/description: Trigger the start of the job: test_simple_pipeline_with_job_and_task/build/unit-test,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.trigger_task("test_simple_pipeline_with_job_and_task", "build", "unit-test").await;
                    }
                "###
            }, {
                listen_for_events:
                    ListenForEvent { type: System(Only(Trigger(Only(Task)))), metadata: [banner.dev/pipeline: _, banner.dev/job: _, banner.dev/task: _] },
                tags:
                    banner.dev/description: Execute the task: _,
                script: ###"
                    pub async fn main (engine, event) {
                        engine.execute_task_name_in_scope("", "_", "_", "_").await;
                    }
                "###
            }], pragmas: [] }"####]]).await;
    }
}
