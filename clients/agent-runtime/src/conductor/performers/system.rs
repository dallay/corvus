use crate::conductor::performers::{Performer, PerformerContext};
use crate::conductor::{PlannedStepForExecution, StepStatus, TaskDomain};
use anyhow::Result;
use async_trait::async_trait;

pub struct SystemPerformer;

#[async_trait]
impl Performer for SystemPerformer {
    fn domain(&self) -> TaskDomain {
        TaskDomain::System
    }

    async fn execute(
        &self,
        step: &PlannedStepForExecution,
        ctx: &PerformerContext,
    ) -> Result<StepStatus> {
        if let Err(error) = ctx.sandbox.run_wrapped(&step.command).await {
            return Ok(StepStatus::Failed {
                error: format!("sandbox_required:{error}"),
            });
        }
        Ok(StepStatus::Completed)
    }
}
