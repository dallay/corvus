use crate::config::ConductorConfig;

pub fn resolve_tick_interval_ms(config: &ConductorConfig, conductor_markdown: Option<&str>) -> u64 {
    if let Some(front_matter_value) = conductor_markdown.and_then(front_matter_tick_interval_ms) {
        return front_matter_value;
    }

    config.tick_interval_ms
}

fn front_matter_tick_interval_ms(markdown: &str) -> Option<u64> {
    let mut lines = markdown.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(value) = parse_tick_interval_line(trimmed) {
            return Some(value);
        }
    }
    None
}

fn parse_tick_interval_line(line: &str) -> Option<u64> {
    let (key, value) = line.split_once(':')?;
    if key.trim() != "tick_interval_ms" {
        return None;
    }
    value.trim().parse::<u64>().ok()
}
