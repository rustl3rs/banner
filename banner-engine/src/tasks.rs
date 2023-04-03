use banner_parser::ast::Task;

use crate::Metadata;

const TASK_TAG_NAME_KEY: &str = "task.banner.io/name";

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
        self.tags
            .iter()
            .find_map(|tag| {
                if tag.key() == TASK_TAG_NAME_KEY {
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
            .map(|t| Tag::new(&t.key, &t.value))
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
