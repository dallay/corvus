#[cfg(test)]
mod tests {
    use crate::security::policy::{SecurityPolicy, CommandRiskLevel, AutonomyLevel};

    #[test]
    fn test_risk_levels() {
        let p = SecurityPolicy::default();

        // Current behavior (potentially weak)
        println!("npm run test risk: {:?}", p.command_risk_level("npm run test"));
        println!("cargo build risk: {:?}", p.command_risk_level("cargo build"));
        println!("cargo run risk: {:?}", p.command_risk_level("cargo run"));

        // Check if npm config is allowed
        println!("npm config allowed: {}", p.is_command_allowed("npm config set editor malicious"));
    }
}
