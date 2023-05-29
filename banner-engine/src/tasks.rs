use banner_parser::ast::Task;
use log::debug;

use crate::{Metadata, TASK_TAG};

pub type Tag = Metadata;
#[derive(Debug, Clone)]
pub struct TaskDefinition {
    tags: Vec<Tag>,
    image: Image,
    command: Vec<String>,
    inputs: Vec<TaskResource>,
    outputs: Vec<TaskResource>,
}

impl TaskDefinition {
    pub fn new(
        tags: Vec<Tag>,
        image: Image,
        command: Vec<String>,
        inputs: Vec<TaskResource>,
        outputs: Vec<TaskResource>,
    ) -> Self {
        Self {
            tags,
            image,
            command,
            inputs,
            outputs,
        }
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

    pub fn tags(&self) -> &[Tag] {
        self.tags.as_ref()
    }

    pub(crate) fn set_image(&mut self, image: Image) {
        self.image = image;
    }

    pub(crate) fn append_inputs(&mut self, input: TaskResource) {
        self.inputs.push(input);
    }

    pub(crate) fn append_outputs(&mut self, output: TaskResource) {
        self.outputs.push(output);
    }

    pub fn env_vars(&self) -> Vec<EnvironmentVariable> {
        self.inputs
            .iter()
            .filter_map(|r| match r {
                TaskResource::EnvVar(key, value) => Some(EnvironmentVariable { key, value }),
                _ => None,
            })
            .collect()
    }

    pub fn mounts(&self) -> Vec<&MountPoint> {
        self.inputs
            .iter()
            .filter_map(|r| match r {
                TaskResource::Mount(mp) => Some(mp),
                _ => None,
            })
            .collect()
    }

    pub fn inputs(&self) -> &[TaskResource] {
        self.inputs.as_ref()
    }

    pub fn outputs(&self) -> &[TaskResource] {
        self.outputs.as_ref()
    }

    pub fn command(&self) -> &[String] {
        self.command.as_ref()
    }

    pub fn get_name(&self) -> &str {
        debug!(target: "task_log", "Searching for name tag in: {:?}", self);
        self.tags
            .iter()
            .find_map(|tag| {
                debug!(target: "task_log", "tag: {:?}", tag);
                if tag.key() == TASK_TAG {
                    Some(tag.value())
                } else {
                    None
                }
            })
            .unwrap()
    }
}

pub type Uri = String;
#[derive(Debug, Clone)]
pub struct Image {
    // TBD
    source: Uri,
    credentials: Option<ImageRepositoryCredentials>,
}

impl Image {
    pub fn new(source: Uri, credentials: Option<ImageRepositoryCredentials>) -> Self {
        Self {
            source,
            credentials,
        }
    }

    pub fn source(&self) -> &str {
        self.source.as_ref()
    }

    pub fn credentials(&self) -> Option<&ImageRepositoryCredentials> {
        self.credentials.as_ref()
    }
}

impl From<banner_parser::ast::Image> for Image {
    fn from(value: banner_parser::ast::Image) -> Self {
        Self {
            source: value.name.clone(),
            credentials: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskResource {
    Semver(semver::Version),
    Counter(u128),
    Mount(MountPoint),
    EnvVar(String, String),
    Secret(String),
}

#[derive(Debug, Clone)]
pub enum ImageRepositoryCredentials {
    UserPass(String, String),
    DockerConfig(String),
}

impl From<&Task> for TaskDefinition {
    fn from(task: &Task) -> Self {
        let tags = task
            .tags
            .iter()
            .map(|t| Tag::new(&t.key, &t.value)) // Add all the tags described with the task
            .collect();
        let image = Image::new(task.image.clone(), None);
        let mut command: Vec<String> = task
            .command
            .as_str()
            .split_whitespace()
            .map(|s| s.into())
            .collect();
        command.push(task.script.clone().as_str().into());
        let td = Self::new(tags, image, command, vec![], vec![]);
        td
    }
}

#[derive(Debug, Clone)]
pub struct EnvironmentVariable<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

#[derive(Debug, Clone)]
pub struct MountPoint {
    pub host_path: HostPath,
    pub container_path: String,
}

#[derive(Debug, Clone)]
pub enum HostPath {
    Path(String),
    Volume(String),
    EngineInit(String),
    EngineFromTask(String),
}
