use super::service::SessionCommandService;
use super::types::{CommandContext, SessionCommandResult, SessionSlashCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCommandSpec {
    pub name: &'static str,
}

const SUPPORTED_COMMANDS: [SessionCommandSpec; 4] = [
    SessionCommandSpec { name: "/resume" },
    SessionCommandSpec { name: "/suspend" },
    SessionCommandSpec { name: "/tldr" },
    SessionCommandSpec { name: "/compact" },
];

pub fn supported_commands() -> &'static [SessionCommandSpec] {
    &SUPPORTED_COMMANDS
}

pub async fn dispatch(
    service: &SessionCommandService<'_>,
    context: CommandContext<'_>,
) -> Result<SessionCommandResult, super::types::SessionCommandError> {
    match context.command {
        SessionSlashCommand::Resume { target, .. } => {
            service
                .handle_resume(context.session_id, target.as_deref())
                .await
        }
        SessionSlashCommand::Suspend => service.handle_suspend(context.session_id).await,
        SessionSlashCommand::Tldr => service.handle_tldr(context.session_id).await,
        SessionSlashCommand::Compact { args } => {
            service.handle_compact(context.session_id, &args).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_supported_commands() {
        let names = supported_commands()
            .iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["/resume", "/suspend", "/tldr", "/compact"]);
    }
}
