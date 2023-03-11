mod tag_missing_error;

use async_trait::async_trait;
use backon::ConstantBuilder;
use backon::Retryable;
use banner_engine::validate_pipeline;
use banner_engine::{Engine, ExecutionResult, TaskDefinition};
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogsOptions, RemoveContainerOptions,
    StartContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::Docker;
use cap_tempfile::{ambient_authority, TempDir, TempFile};
use futures_util::stream::TryStreamExt;
use std::error::Error;
use std::fs;
use std::marker::{Send, Sync};
use std::path::PathBuf;
use std::time::Duration;
use tag_missing_error::TagMissingError;

#[derive(Debug)]
pub struct LocalEngine {
    pipelines: Vec<PathBuf>,
}

impl LocalEngine {
    pub fn new() -> Self {
        Self { pipelines: vec![] }
    }

    pub fn with_pipeline_from_file(
        &mut self,
        filepath: PathBuf,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let pipeline =
            fs::read_to_string(&filepath).expect("Should have been able to read the file");
        match validate_pipeline(pipeline) {
            Ok(()) => {
                self.pipelines.push(filepath);
                ()
            }
            Err(e) => {
                let f = filepath.to_str().unwrap();
                eprintln!("Error parsing pipeline from file: {f}.\n\n{e}")
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Engine for LocalEngine {
    // I tried making this return an aggregate error in the case where access to the temp
    // directory and the availability of docker was missing.  This unfortunately, didn't
    // really add much real value to the function, and obscured it enough that I just thought
    // it was a bad thing.
    async fn confirm_requirements(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        check_access_to_temp().await?;
        check_availability_of_docker().await?;
        Ok(())
    }

    async fn initialise(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    // ! TODO: if no label is given, need to add `:latest`
    async fn execute(
        &self,
        task: &TaskDefinition,
    ) -> Result<ExecutionResult, Box<dyn Error + Send + Sync>> {
        let docker = Docker::connect_with_local_defaults()?;

        // construct our container name
        let pipeline_name = get_task_tag_value(&task, "pipeline")?;
        let job_name = get_task_tag_value(&task, "job")?;
        let task_name = get_task_tag_value(&task, "task")?;
        let container_name = format!("banner_{pipeline_name}_{job_name}_{task_name}");

        // TODO: don't pull if the image is already here?
        // ensure the image is pulled locally.
        let cio = CreateImageOptions {
            from_image: task.image().source(),
            ..Default::default()
        };
        let mut ci_logs = docker.create_image(Some(cio), None, None);
        loop {
            if let Some(log) = ci_logs.try_next().await? {
                if let Some(status) = log.status {
                    log::info!(target: "task_log", "{task_name}: {}", status);
                }
                continue;
            } else {
                break;
            }
        }

        // create our container
        let options = Some(CreateContainerOptions {
            name: &container_name,
            platform: None,
        });

        // one must first create the container before starting it.
        let commands: Vec<&str> = task.command().iter().map(|s| s.as_ref()).collect();
        let config = Config {
            image: Some(task.image().source()),
            cmd: Some(commands),

            ..Default::default()
        };
        docker.create_container(options, config).await?;

        // start the just created container retry if something happend.... just twice tho.
        // it's likely that the image isn't present, so just go grab it.
        // let docker = Docker::connect_with_local_defaults()?;
        let start = || async {
            docker
                .start_container(&container_name, None::<StartContainerOptions<String>>)
                .await
        };

        let backoff = ConstantBuilder::default()
            .with_delay(Duration::from_secs(2))
            .with_max_times(2);

        match start.retry(&backoff).await {
            Ok(_) => (),
            Err(e) => {
                let _ = remove_container(&container_name).await;
                return Err(Box::new(e));
            }
        };

        stream_logs_from_container_to_stdout(&container_name, &task_name).await?;

        remove_container(&container_name).await?;
        Ok(ExecutionResult::Success(vec![]))
    }

    async fn get_pipelines(&self) -> Vec<String> {
        let pipelines: Vec<String> = self
            .pipelines
            .iter()
            .map(|fb| fs::read_to_string(&fb).expect("Should have been able to read the file"))
            .collect();
        pipelines
    }
}

async fn stream_logs_from_container_to_stdout(
    container_name: &str,
    task_name: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults().unwrap();

    let options = Some(LogsOptions::<String> {
        follow: true,
        stdout: true,
        timestamps: true,
        stderr: true,
        ..Default::default()
    });

    let mut logs = docker.logs(container_name, options);
    loop {
        if let Some(log) = logs.try_next().await? {
            log::info!(target: "task_log", "{task_name}: {log}");
            continue;
        } else {
            return Ok(());
        }
    }
}

async fn remove_container(container_name: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults().unwrap();
    let options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });
    Ok(docker.remove_container(&container_name, options).await?)
}

async fn check_availability_of_docker() -> Result<(), Box<dyn Error + Send + Sync>> {
    // check for the existence of docker
    let docker = Docker::connect_with_local_defaults()?;
    docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?;
    Ok(())
}

async fn check_access_to_temp() -> Result<(), Box<dyn Error + Send + Sync>> {
    let dir = TempDir::new(ambient_authority())?;
    let _file = TempFile::new(&dir)?;
    Ok(())
}

fn get_task_tag_value<'a>(
    task: &'a TaskDefinition,
    key: &str,
) -> Result<&'a str, Box<dyn Error + Send + Sync>> {
    match task
        .tags()
        .iter()
        .filter(|tag| tag.key() == format!("banner.io/{key}"))
        .find_map(|tag| Some(tag.value()))
    {
        Some(value) => Ok(value),
        None => {
            let err = TagMissingError::new(format!(
                "Expected banner.io tag not present on task: banner.io/{key}"
            ));
            Err(Box::new(err))
        }
    }
}
