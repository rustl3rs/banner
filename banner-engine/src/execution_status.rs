use std::fmt::Display;

use strum_macros::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, EnumString)]
#[non_exhaustive]
pub enum ExecutionStatus {
    Pending,
    Running,
    Success,
    Failed,
}

impl Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Pending => write!(f, "Pending"),
            ExecutionStatus::Running => write!(f, "Running"),
            ExecutionStatus::Success => write!(f, "Success"),
            ExecutionStatus::Failed => write!(f, "Failed"),
        }
    }
}
