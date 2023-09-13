mod tag_missing_error;

use async_trait::async_trait;
use backon::ConstantBuilder;
use backon::Retryable;
use banner_engine::HostPath;
use banner_engine::PipelineSpecification;
use banner_engine::{
    build_and_validate_pipeline, Engine, ExecutionResult, Pipeline, TaskDefinition, JOB_TAG,
    PIPELINE_TAG, TASK_TAG,
};
use bollard::container::InspectContainerOptions;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogsOptions, RemoveContainerOptions,
    StartContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::service::HostConfig;
use bollard::service::Mount;
use bollard::Docker;
use cap_tempfile::{ambient_authority, TempDir, TempFile};
use futures_util::stream::TryStreamExt;
use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::marker::{Send, Sync};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use tag_missing_error::TagMissingError;

// #[derive(Debug)]
// pub enum StateDirectory {
//     TempDir(TempDir),
//     Path(PathBuf),
// }

#[derive(Debug)]
pub struct LocalEngine {
    pipelines: Vec<Pipeline>,
    specifications: Vec<PipelineSpecification>,
    directories: Arc<RwLock<HashMap<String, String>>>,
    state_dir: PathBuf,
    state: Arc<RwLock<HashMap<String, String>>>,
}

impl LocalEngine {
    pub fn new() -> Self {
        Self {
            pipelines: vec![],
            specifications: vec![],
            directories: Arc::new(RwLock::new(HashMap::new())),
            state_dir: {
                let dir = Alphanumeric.sample_string(&mut rand::thread_rng(), 8);
                let path = PathBuf::from("/tmp/banner").join(dir);
                log::info!(target: "task_log", "Creating state directory: {:?}", path);
                std::fs::create_dir_all(path.as_path()).unwrap();
                path
            },
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn with_pipeline_from_file(
        &mut self,
        filepath: PathBuf,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let pipeline =
            fs::read_to_string(&filepath).expect("Should have been able to read the file");
        match build_and_validate_pipeline(&pipeline).await {
            Ok((pipeline, mut specifications)) => {
                self.pipelines.push(pipeline);
                self.specifications.append(&mut specifications);
                ()
            }
            Err(e) => {
                let f = filepath.to_str().unwrap();
                eprintln!("Error parsing pipeline from file: {f}.\n\n{e}")
            }
        }

        Ok(())
    }

    fn get_dir_for_mount_source(
        &self,
        host_path: &HostPath,
        pipeline_name: &str,
        job_name: &str,
        task_name: &str,
    ) -> (Option<String>, Option<bollard::service::MountTypeEnum>) {
        match host_path {
            HostPath::Path(dir) => (
                Some(dir.to_string()),
                Some(bollard::service::MountTypeEnum::BIND),
            ),
            HostPath::Volume(_name) => todo!(),
            HostPath::EngineInit(name) => {
                let key = format!("{}.{}.{}.{}", pipeline_name, job_name, task_name, name);
                let mut path = PathBuf::new();
                path.push(&self.state_dir);
                path.push(&key);
                std::fs::create_dir(path.as_path()).unwrap();
                let dir = path.to_str().unwrap();
                self.directories
                    .write()
                    .unwrap()
                    .insert(key, dir.to_string());
                (
                    Some(dir.to_string()),
                    Some(bollard::service::MountTypeEnum::BIND),
                )
            }
            HostPath::EngineFromTask(task) => {
                let dirs = self.directories.read().unwrap();
                let dir = dirs.get(task).expect("Should have found the directory");
                (
                    Some(dir.to_string()),
                    Some(bollard::service::MountTypeEnum::BIND),
                )
            }
        }
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
        let pipeline_name = get_task_tag_value(&task, PIPELINE_TAG)?;
        let job_name = get_task_tag_value(&task, JOB_TAG)?;
        let task_name = get_task_tag_value(&task, TASK_TAG)?;
        let container_name = format!("banner_{pipeline_name}_{job_name}_{task_name}");
        log::info!(target: "task_log", "Starting container: {container_name}");

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
        let commands: Vec<&str> = task
            .command()
            .iter()
            .filter_map(|s| if s.is_empty() { None } else { Some(s.as_ref()) })
            .collect();
        log::info!(target: "task_log", "{commands:?}");
        let mut env_vars = vec![
            format!("BANNER_PIPELINE={}", pipeline_name),
            format!("BANNER_JOB={}", job_name),
            format!("BANNER_TASK={}", task_name),
        ];
        env_vars.extend(
            task.env_vars()
                .iter()
                .map(|env_var| format!("{}={}", env_var.key, env_var.value)),
        );

        // create a host config to mount any volumes requested in the task.
        let mounts: Vec<Mount> = task
            .mounts()
            .into_iter()
            .map(|mount| {
                let (source, mount_type) = self.get_dir_for_mount_source(
                    &mount.host_path,
                    &pipeline_name,
                    &job_name,
                    &task_name,
                );
                let mut m = Mount::default();
                m.target = Some(mount.container_path.clone());
                m.source = source;
                m.typ = mount_type;
                m.read_only = Some(false);
                m
            })
            .collect();
        let host_config = HostConfig {
            mounts: Some(mounts),
            ..Default::default()
        };
        log::trace!(target: "task_log", "host_config: {host_config:?}");
        let config = Config {
            image: Some(task.image().source()),
            cmd: Some(commands),

            env: Some(env_vars.iter().map(|s| s.as_ref()).collect()),
            host_config: Some(host_config),
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

        // TODO: spawn this and a metrics task.
        //       metrics task to gather CPU/Memory and Network usage of the container and make them available for prometheus? emit as metrics events.
        stream_logs_from_container_to_stdout(&container_name, &task_name).await?;

        // get the container status so we can get it's exit code.
        let inspect_options = InspectContainerOptions { size: false };
        let inspect_result = docker
            .inspect_container(&container_name, Some(inspect_options))
            .await?;

        // clean up.
        remove_container(&container_name).await?;

        // handle the container exit code.
        match inspect_result.state.unwrap().exit_code.unwrap() {
            0 => Ok(ExecutionResult::Success(vec![])),
            _ => Ok(ExecutionResult::Failed(vec![])),
        }
    }

    // TODO: fix the scope pipeline and job usage.
    async fn execute_task_name_in_scope(
        &self,
        _scope_name: &str,
        _pipeline_name: &str,
        _job_name: &str,
        task_name: &str,
    ) -> Result<ExecutionResult, Box<dyn Error + Send + Sync>> {
        let pipelines = self.pipelines.clone();
        let task_definition: &TaskDefinition = get_task_definition(&pipelines, task_name);
        self.execute(task_definition).await
    }

    fn get_pipelines(&self) -> Vec<&banner_engine::Pipeline> {
        self.pipelines.iter().collect()
    }

    fn get_pipeline_specification(&self) -> &Vec<PipelineSpecification> {
        &self.specifications
    }

    /// Returns a map of key/value pairs that represent the state of the engine for a specific pipeline run.
    fn get_state_for_id(&self, key: &str) -> Option<String> {
        let hm = self.state.read().unwrap();
        let result = hm.get(key);
        match result {
            Some(val) => Some(val.to_string()),
            None => None,
        }
    }

    /// Returns a value from state based on the key.
    fn set_state_for_id(
        &self,
        key: &str,
        value: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!(target: "task_log", "set_state_for_id: {key} {value}");
        let mut hm = self.state.write().unwrap();
        hm.insert(key.to_string(), value);
        Ok(())
    }
}

fn get_task_definition<'a>(pipelines: &'a Vec<Pipeline>, task_name: &'a str) -> &'a TaskDefinition {
    pipelines
        .iter()
        .find_map(|pipeline| {
            Some(
                pipeline
                    .tasks
                    .iter()
                    .find(|task| (*task).get_name() == task_name)
                    .unwrap(),
            )
        })
        .unwrap()
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
    println!("Checking availability of docker...");
    // check for the existence of docker
    let docker = Docker::connect_with_local_defaults()?;
    let result = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await;
    match result {
        Ok(_) => {
            println!("Docker is available.");
            Ok(())
        }
        Err(e) => {
            println!("Docker is not available.");
            Err(Box::new(e))
        }
    }
}

async fn check_access_to_temp() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Checking access to temp...");
    let dir = TempDir::new(ambient_authority())?;
    let _file = TempFile::new(&dir)?;
    println!("Access to temp is good.");
    Ok(())
}

fn get_task_tag_value<'a>(
    task: &'a TaskDefinition,
    key: &str,
) -> Result<&'a str, Box<dyn Error + Send + Sync>> {
    log::trace!(target: "task_log", "Looking for {key} in {:?}", task.tags());
    match task.tags().iter().find_map(|tag| {
        if tag.key() == key {
            Some(tag.value())
        } else {
            None
        }
    }) {
        Some(value) => {
            log::trace!(target: "task_log", "Found: {value}");
            Ok(value)
        }
        None => {
            let err = TagMissingError::new(format!("Expected tag not present on task: {key}",));
            log::warn!(target: "task_log", "{err:?}");
            Ok("_")
        }
    }
}
