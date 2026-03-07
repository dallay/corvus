use crate::conductor::{TaskDomain, TaskOrigin, TaskPriority, TaskRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelRouteOutcome {
    Task(Box<TaskRequest>),
    ChatPassthrough,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliRouteOutcome {
    Task(Box<TaskRequest>),
    AgentPassthrough,
}

fn extract_explicit_task_description(message: &str) -> Option<String> {
    let trimmed = message.trim();
    let task_description = trimmed.strip_prefix("/task ")?.trim();
    if task_description.is_empty() {
        return None;
    }
    Some(task_description.to_string())
}

pub fn route_channel_message(
    conductor_enabled: bool,
    message: &str,
    channel_name: &str,
    channel_id: &str,
    sender: &str,
    thread_id: Option<&str>,
) -> ChannelRouteOutcome {
    if !conductor_enabled {
        return ChannelRouteOutcome::ChatPassthrough;
    }

    let Some(task_description) = extract_explicit_task_description(message) else {
        return ChannelRouteOutcome::ChatPassthrough;
    };

    ChannelRouteOutcome::Task(Box::new(TaskRequest {
        description: task_description,
        origin: TaskOrigin::Chat {
            channel_name: channel_name.to_string(),
            channel_id: channel_id.to_string(),
            sender: sender.to_string(),
            thread_id: thread_id.map(ToString::to_string),
        },
        priority: TaskPriority::Normal,
        context: None,
        workspace_hint: None,
        timeout_ms: None,
        tags: vec!["channel".to_string(), "task".to_string()],
        domain: TaskDomain::Composite,
    }))
}

pub fn route_cli_message(conductor_enabled: bool, message: &str) -> CliRouteOutcome {
    if !conductor_enabled {
        return CliRouteOutcome::AgentPassthrough;
    }

    let Some(task_description) = extract_explicit_task_description(message) else {
        return CliRouteOutcome::AgentPassthrough;
    };

    CliRouteOutcome::Task(Box::new(TaskRequest {
        description: task_description,
        origin: TaskOrigin::Cli {
            working_dir: std::env::current_dir()
                .ok()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| ".".to_string()),
        },
        priority: TaskPriority::Normal,
        context: None,
        workspace_hint: None,
        timeout_ms: None,
        tags: vec!["cli".to_string(), "task".to_string()],
        domain: TaskDomain::Composite,
    }))
}
