use async_trait::async_trait;
use banner_engine::{Engine, ExecutionResult, TaskDefinition};
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogsOptions, RemoveContainerOptions,
    StartContainerOptions,
};
use bollard::Docker;
use cap_tempfile::{ambient_authority, TempDir, TempFile};
use futures_util::stream::TryStreamExt;
use std::error::Error;
use std::fmt::Display;
use std::marker::{Send, Sync};

#[derive(Debug)]
pub struct LocalEngine {}

impl LocalEngine {
    pub fn new() -> Self {
        Self {}
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

        // start the just created container
        match docker
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await
        {
            Ok(_) => (),
            Err(e) => {
                remove_container(&container_name).await?;
                return Err(Box::new(e));
            }
        }

        stream_logs_from_container_to_stdout(&container_name, &task_name).await?;

        remove_container(&container_name).await?;
        Ok(ExecutionResult::Success(vec![]))
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
            log::info!("{task_name}: {log}");
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
            let err = TagMissingError::new(String::from(format!(
                "Expected banner.io tag not present on task: banner.io/{key}"
            )));
            Err(Box::new(err))
        }
    }
}

#[derive(Debug, Default)]
pub struct TagMissingError {
    description: String,
}

impl TagMissingError {
    pub fn new(description: String) -> Self {
        Self { description }
    }

    pub fn description(&self) -> &str {
        self.description.as_ref()
    }
}

impl Display for TagMissingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for TagMissingError {}
