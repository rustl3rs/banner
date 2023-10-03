use std::sync::{Arc, RwLock};

use super::job::Status;

#[derive(Debug, Clone)]
pub(crate) enum IdentifierListItem {
    Identifier(Metadata),
    SequentialList(Vec<IdentifierListItem>),
    ParallelList(Vec<IdentifierListItem>),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct JobSpecification {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub tasks: Vec<IdentifierListItem>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PipelineSpecification {
    pub name: String,
    pub jobs: Vec<IdentifierListItem>,
}

#[derive(Debug, Clone)]
pub(crate) struct Metadata {
    pub(crate) name: String,
    postion: Arc<RwLock<Option<(u16, u16)>>>,
    status: Arc<RwLock<Status>>,
}

impl Metadata {
    pub(crate) fn new(name: String) -> Self {
        Metadata {
            name,
            postion: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(Status::Pending)),
        }
    }

    pub(crate) fn set_position(&self, x: u16, y: u16) {
        let mut pg = self.postion.write().unwrap();
        *pg = Some((x, y));
    }

    pub(crate) fn set_status(&self, status: Status) {
        let mut sg = self.status.write().unwrap();
        *sg = status;
    }

    pub(crate) fn get_position(&self) -> Option<(u16, u16)> {
        *self.postion.read().unwrap()
    }

    pub(crate) fn get_status(&self) -> Status {
        self.status.read().unwrap().clone()
    }
}

impl From<&banner_engine::PipelineSpecification> for PipelineSpecification {
    fn from(pipeline: &banner_engine::PipelineSpecification) -> Self {
        PipelineSpecification {
            name: pipeline.name.clone(),
            jobs: pipeline.jobs.iter().map(|j| j.into()).collect(),
        }
    }
}

impl From<&banner_engine::JobSpecification> for JobSpecification {
    fn from(job: &banner_engine::JobSpecification) -> Self {
        JobSpecification {
            name: job.name.clone(),
            tasks: job.tasks.iter().map(|t| t.into()).collect(),
        }
    }
}

impl From<&banner_engine::IdentifierListItem> for IdentifierListItem {
    fn from(item: &banner_engine::IdentifierListItem) -> Self {
        match item {
            banner_engine::IdentifierListItem::Identifier(i, _) => {
                IdentifierListItem::Identifier(Metadata::new(i.clone()))
            }
            banner_engine::IdentifierListItem::SequentialList(list) => {
                IdentifierListItem::SequentialList(list.iter().map(|i| i.into()).collect())
            }
            banner_engine::IdentifierListItem::ParallelList(list) => {
                IdentifierListItem::ParallelList(list.iter().map(|i| i.into()).collect())
            }
        }
    }
}
