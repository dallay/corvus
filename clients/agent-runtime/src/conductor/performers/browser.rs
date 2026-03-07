use crate::conductor::performers::{Performer, PerformerContext};
use crate::conductor::{PlannedStepForExecution, StepStatus, TaskDomain};
use anyhow::Result;
use async_trait::async_trait;

pub struct BrowserPerformer;

#[async_trait]
impl Performer for BrowserPerformer {
    fn domain(&self) -> TaskDomain {
        TaskDomain::Browser
    }

    async fn execute(
        &self,
        _step: &PlannedStepForExecution,
        _ctx: &PerformerContext,
    ) -> Result<StepStatus> {
        Ok(StepStatus::Completed)
    }
}
