use crate::conductor::performers::{Performer, PerformerContext};
use crate::conductor::{PlannedStepForExecution, StepStatus, TaskDomain};
use anyhow::Result;
use async_trait::async_trait;

pub struct ResearchPerformer;

#[async_trait]
impl Performer for ResearchPerformer {
    fn domain(&self) -> TaskDomain {
        TaskDomain::Research
    }

    async fn execute(
        &self,
        _step: &PlannedStepForExecution,
        _ctx: &PerformerContext,
    ) -> Result<StepStatus> {
        Ok(StepStatus::Completed)
    }
}
