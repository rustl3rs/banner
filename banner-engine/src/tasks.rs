use banner_parser::ast::Task;
use log::debug;

use crate::{Metadata, TASK_TAG};

pub type Tag = Metadata;
#[derive(Debug)]
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
#[derive(Debug)]
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

#[derive(Debug)]
pub enum TaskResource {
    Semver(semver::Version),
    Counter(u128),
    Path(String),
    EnvVar(String),
    Secret(String),
}

#[derive(Debug)]
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
            // .chain(Some(Tag::new_banner_task(&task.name))) // Add task tag
            // .chain(Some(Tag::new_banner_job(job))) // Add job tag
            // .chain(Some(Tag::new_banner_pipeline(pipeline))) // Add pipeline tag
            .collect();
        let image = Image::new(task.image.clone(), None);
        let mut command: Vec<String> = task
            .command
            .as_str()
            .split_whitespace()
            .map(|s| s.into())
            .collect();
        command.push(task.script.clone());
        let td = Self::new(tags, image, command, vec![], vec![]);
        td
    }
}
