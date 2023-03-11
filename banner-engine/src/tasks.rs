use crate::Metadata;

pub type Tag = Metadata;
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
}

pub type Uri = String;
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

pub enum TaskResource {
    Semver(semver::Version),
    Counter(u128),
    Path(String),
    EnvVar(String),
    Secret(String),
}

pub enum ImageRepositoryCredentials {
    UserPass(String, String),
    DockerConfig(String),
}
