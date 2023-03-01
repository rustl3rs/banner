pub type TaskMetadata = Metadata;
pub type JobMetadata = Metadata;
pub type PipelineMetadata = Metadata;

// Metadata became a way of evolving the usage; without having to care
// too much about blowing up the implementations between versions.
// It might work.. It might not... Truth of the matter is, I don't know
// all the stuff I might want to pass around about an event yet.
#[derive(Debug, Clone)]
pub struct Metadata {
    key: String,
    value: String,
}

impl Metadata {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }

    pub fn key(&self) -> &str {
        self.key.as_ref()
    }

    pub fn value(&self) -> &str {
        self.value.as_ref()
    }
}
