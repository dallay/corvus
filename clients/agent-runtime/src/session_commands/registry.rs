use super::parser::SessionCommandParser;
use super::service::SessionCommandService;
use super::types::{
    CommandContext, RawSlashInvocation, SessionCommandError, SessionCommandResult,
    SlashCommandArgumentShape, SlashCommandDescriptor, SlashCommandHandler,
    SlashCommandRegistration, SlashCommandRequirements, SlashInvocation, SlashRegistryError,
};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

pub struct SlashCommandRegistry {
    registrations: Vec<SlashCommandRegistration>,
    by_canonical_name: BTreeMap<&'static str, usize>,
    by_alias: BTreeMap<&'static str, usize>,
}

impl Default for SlashCommandRegistry {
    fn default() -> Self {
        Self::empty()
    }
}

impl SlashCommandRegistry {
    pub fn empty() -> Self {
        Self {
            registrations: Vec::new(),
            by_canonical_name: BTreeMap::new(),
            by_alias: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        registration: SlashCommandRegistration,
    ) -> Result<(), SlashRegistryError> {
        validate_name(registration.descriptor.canonical_name)?;

        if registration.descriptor.description.trim().is_empty() {
            return Err(SlashRegistryError::EmptyDescription {
                canonical_name: registration.descriptor.canonical_name.to_string(),
            });
        }

        if self
            .by_canonical_name
            .contains_key(registration.descriptor.canonical_name)
        {
            return Err(SlashRegistryError::DuplicateCanonicalName {
                canonical_name: registration.descriptor.canonical_name.to_string(),
            });
        }

        for alias in registration.descriptor.aliases {
            validate_name(alias)?;

            if *alias == registration.descriptor.canonical_name {
                return Err(SlashRegistryError::AliasCollidesWithCanonical {
                    alias: alias.to_string(),
                    canonical_name: registration.descriptor.canonical_name.to_string(),
                });
            }

            if let Some(existing_index) = self.by_alias.get(alias) {
                return Err(SlashRegistryError::DuplicateAlias {
                    alias: alias.to_string(),
                    existing_canonical_name: self.registrations[*existing_index]
                        .descriptor
                        .canonical_name
                        .to_string(),
                });
            }

            if self.by_canonical_name.contains_key(alias) {
                return Err(SlashRegistryError::AliasCollidesWithCanonical {
                    alias: alias.to_string(),
                    canonical_name: alias.to_string(),
                });
            }
        }

        // Check canonical name doesn't conflict with existing alias
        if let Some(existing_index) = self.by_alias.get(registration.descriptor.canonical_name) {
            return Err(SlashRegistryError::AliasCollidesWithCanonical {
                alias: registration.descriptor.canonical_name.to_string(),
                canonical_name: self.registrations[*existing_index]
                    .descriptor
                    .canonical_name
                    .to_string(),
            });
        }

        // Detect duplicate aliases inside the same descriptor
        let mut seen_aliases = std::collections::HashSet::new();
        for alias in registration.descriptor.aliases {
            if !seen_aliases.insert(alias.to_string()) {
                return Err(SlashRegistryError::DuplicateAlias {
                    alias: alias.to_string(),
                    existing_canonical_name: registration.descriptor.canonical_name.to_string(),
                });
            }
        }

        let index = self.registrations.len();
        self.by_canonical_name
            .insert(registration.descriptor.canonical_name, index);
        for alias in registration.descriptor.aliases {
            self.by_alias.insert(alias, index);
        }
        self.registrations.push(registration);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&SlashCommandDescriptor> {
        self.resolve_registration(name)
            .map(|registration| &registration.descriptor)
    }

    pub fn recognizes(&self, prompt: &str) -> bool {
        SessionCommandParser::parse(prompt)
            .and_then(|raw| self.get(&raw.invoked_name))
            .is_some()
    }

    fn iter(&self) -> impl Iterator<Item = &SlashCommandDescriptor> {
        self.registrations
            .iter()
            .map(|registration| &registration.descriptor)
    }

    pub async fn dispatch(
        &self,
        service: &SessionCommandService<'_>,
        context: CommandContext<'_>,
        prompt: &str,
    ) -> Option<Result<SessionCommandResult, SessionCommandError>> {
        let raw = SessionCommandParser::parse(prompt)?;
        let registration = self.resolve_registration(&raw.invoked_name)?;
        let invocation = match validate_invocation(&registration.descriptor, raw) {
            Ok(invocation) => invocation,
            Err(error) => return Some(Err(error)),
        };

        Some(
            registration
                .handler
                .handle(service, context, invocation)
                .await,
        )
    }

    fn resolve_registration(&self, name: &str) -> Option<&SlashCommandRegistration> {
        self.by_canonical_name
            .get(name)
            .or_else(|| self.by_alias.get(name))
            .and_then(|index| self.registrations.get(*index))
    }
}

pub fn default_registry() -> &'static SlashCommandRegistry {
    static REGISTRY: OnceLock<SlashCommandRegistry> = OnceLock::new();
    REGISTRY.get_or_init(build_default_registry)
}

fn build_default_registry() -> SlashCommandRegistry {
    let mut registry = SlashCommandRegistry::empty();
    for registration in built_in_registrations() {
        if let Err(error) = registry.register(registration) {
            panic!("invalid built-in slash command registry: {error:?}");
        }
    }
    registry
}

fn built_in_registrations() -> [SlashCommandRegistration; 4] {
    [
        SlashCommandRegistration {
            descriptor: SlashCommandDescriptor {
                canonical_name: "/resume",
                aliases: &["/continue"],
                description: "Resume a suspended session or list resumable sessions.",
                argument_shape: SlashCommandArgumentShape::OptionalTargetThenText,
                requirements: SlashCommandRequirements {
                    capability_tags: vec!["session-lifecycle"],
                    permission_tags: vec!["verifiable-caller-identity"],
                    backend_tags: vec!["sqlite"],
                },
            },
            handler: Arc::new(ResumeHandler),
        },
        SlashCommandRegistration {
            descriptor: SlashCommandDescriptor {
                canonical_name: "/suspend",
                aliases: &[],
                description: "Suspend the current session using the latest compact snapshot.",
                argument_shape: SlashCommandArgumentShape::None,
                requirements: SlashCommandRequirements {
                    capability_tags: vec!["session-lifecycle"],
                    permission_tags: vec![],
                    backend_tags: vec!["sqlite"],
                },
            },
            handler: Arc::new(SuspendHandler),
        },
        SlashCommandRegistration {
            descriptor: SlashCommandDescriptor {
                canonical_name: "/tldr",
                aliases: &[],
                description: "Persist a concise session summary snapshot.",
                argument_shape: SlashCommandArgumentShape::None,
                requirements: SlashCommandRequirements {
                    capability_tags: vec!["session-summary"],
                    permission_tags: vec![],
                    backend_tags: vec!["sqlite"],
                },
            },
            handler: Arc::new(TldrHandler),
        },
        SlashCommandRegistration {
            descriptor: SlashCommandDescriptor {
                canonical_name: "/compact",
                aliases: &[],
                description: "Create a resume-capable compact snapshot for the current session.",
                argument_shape: SlashCommandArgumentShape::OptionalText,
                requirements: SlashCommandRequirements {
                    capability_tags: vec!["session-summary"],
                    permission_tags: vec![],
                    backend_tags: vec!["sqlite"],
                },
            },
            handler: Arc::new(CompactHandler),
        },
    ]
}

struct ResumeHandler;
struct SuspendHandler;
struct TldrHandler;
struct CompactHandler;

#[async_trait::async_trait]
impl SlashCommandHandler for ResumeHandler {
    async fn handle(
        &self,
        service: &SessionCommandService<'_>,
        context: CommandContext<'_>,
        invocation: SlashInvocation,
    ) -> Result<SessionCommandResult, SessionCommandError> {
        service
            .handle_resume(
                context.session_id,
                invocation.primary_target.as_deref(),
                context.caller_token_hash,
            )
            .await
    }
}

#[async_trait::async_trait]
impl SlashCommandHandler for SuspendHandler {
    async fn handle(
        &self,
        service: &SessionCommandService<'_>,
        context: CommandContext<'_>,
        _invocation: SlashInvocation,
    ) -> Result<SessionCommandResult, SessionCommandError> {
        service.handle_suspend(context.session_id).await
    }
}

#[async_trait::async_trait]
impl SlashCommandHandler for TldrHandler {
    async fn handle(
        &self,
        service: &SessionCommandService<'_>,
        context: CommandContext<'_>,
        _invocation: SlashInvocation,
    ) -> Result<SessionCommandResult, SessionCommandError> {
        service.handle_tldr(context.session_id).await
    }
}

#[async_trait::async_trait]
impl SlashCommandHandler for CompactHandler {
    async fn handle(
        &self,
        service: &SessionCommandService<'_>,
        context: CommandContext<'_>,
        invocation: SlashInvocation,
    ) -> Result<SessionCommandResult, SessionCommandError> {
        service
            .handle_compact(context.session_id, &invocation.raw_args)
            .await
    }
}

fn validate_name(name: &str) -> Result<(), SlashRegistryError> {
    let chars = name.chars().collect::<Vec<_>>();
    let valid = chars.len() >= 2
        && chars[0] == '/'
        && chars[1].is_ascii_lowercase()
        && chars[1..].iter().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || *ch == '-'
        });

    if valid {
        Ok(())
    } else {
        Err(SlashRegistryError::InvalidName {
            name: name.to_string(),
        })
    }
}

fn validate_invocation(
    descriptor: &SlashCommandDescriptor,
    raw: RawSlashInvocation,
) -> Result<SlashInvocation, SessionCommandError> {
    match descriptor.argument_shape {
        SlashCommandArgumentShape::None => {
            if raw.raw_args.is_empty() {
                Ok(SlashInvocation {
                    invoked_name: raw.invoked_name,
                    canonical_name: descriptor.canonical_name,
                    raw_args: String::new(),
                    primary_target: None,
                })
            } else {
                Err(SessionCommandError::InvalidArguments {
                    command: descriptor.canonical_name,
                    detail: "this command does not accept trailing arguments".to_string(),
                })
            }
        }
        SlashCommandArgumentShape::OptionalText => Ok(SlashInvocation {
            invoked_name: raw.invoked_name,
            canonical_name: descriptor.canonical_name,
            raw_args: raw.raw_args,
            primary_target: None,
        }),
        SlashCommandArgumentShape::OptionalTargetThenText => {
            let (primary_target, raw_args) =
                SessionCommandParser::split_primary_target(&raw.raw_args);
            Ok(SlashInvocation {
                invoked_name: raw.invoked_name,
                canonical_name: descriptor.canonical_name,
                raw_args,
                primary_target,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct NoopHandler;

    #[async_trait]
    impl SlashCommandHandler for NoopHandler {
        async fn handle(
            &self,
            _service: &SessionCommandService<'_>,
            _context: CommandContext<'_>,
            _invocation: SlashInvocation,
        ) -> Result<SessionCommandResult, SessionCommandError> {
            unreachable!("handler should not run in registry validation tests")
        }
    }

    fn registration(
        canonical_name: &'static str,
        aliases: &'static [&'static str],
        description: &'static str,
        argument_shape: SlashCommandArgumentShape,
    ) -> SlashCommandRegistration {
        SlashCommandRegistration {
            descriptor: SlashCommandDescriptor {
                canonical_name,
                aliases,
                description,
                argument_shape,
                requirements: SlashCommandRequirements::default(),
            },
            handler: Arc::new(NoopHandler),
        }
    }

    #[test]
    fn registry_rejects_invalid_names() {
        let mut registry = SlashCommandRegistry::empty();

        let error = registry
            .register(registration(
                "/Resume",
                &[],
                "resume a session",
                SlashCommandArgumentShape::OptionalTargetThenText,
            ))
            .unwrap_err();

        assert_eq!(
            error,
            SlashRegistryError::InvalidName {
                name: "/Resume".to_string(),
            }
        );
    }

    #[test]
    fn registry_rejects_empty_descriptions() {
        let mut registry = SlashCommandRegistry::empty();

        let error = registry
            .register(registration(
                "/resume",
                &[],
                "   ",
                SlashCommandArgumentShape::OptionalTargetThenText,
            ))
            .unwrap_err();

        assert_eq!(
            error,
            SlashRegistryError::EmptyDescription {
                canonical_name: "/resume".to_string(),
            }
        );
    }

    #[test]
    fn registry_rejects_duplicate_canonical_names() {
        let mut registry = SlashCommandRegistry::empty();
        registry
            .register(registration(
                "/resume",
                &[],
                "resume a session",
                SlashCommandArgumentShape::OptionalTargetThenText,
            ))
            .unwrap();

        let error = registry
            .register(registration(
                "/resume",
                &[],
                "resume again",
                SlashCommandArgumentShape::OptionalTargetThenText,
            ))
            .unwrap_err();

        assert_eq!(
            error,
            SlashRegistryError::DuplicateCanonicalName {
                canonical_name: "/resume".to_string(),
            }
        );
    }

    #[test]
    fn registry_rejects_duplicate_aliases() {
        let mut registry = SlashCommandRegistry::empty();
        registry
            .register(registration(
                "/resume",
                &["/continue"],
                "resume a session",
                SlashCommandArgumentShape::OptionalTargetThenText,
            ))
            .unwrap();

        let error = registry
            .register(registration(
                "/compact",
                &["/continue"],
                "compact a session",
                SlashCommandArgumentShape::OptionalText,
            ))
            .unwrap_err();

        assert_eq!(
            error,
            SlashRegistryError::DuplicateAlias {
                alias: "/continue".to_string(),
                existing_canonical_name: "/resume".to_string(),
            }
        );
    }

    #[test]
    fn registry_rejects_alias_collisions_with_canonical_names() {
        let mut registry = SlashCommandRegistry::empty();
        registry
            .register(registration(
                "/resume",
                &[],
                "resume a session",
                SlashCommandArgumentShape::OptionalTargetThenText,
            ))
            .unwrap();

        let error = registry
            .register(registration(
                "/compact",
                &["/resume"],
                "compact a session",
                SlashCommandArgumentShape::OptionalText,
            ))
            .unwrap_err();

        assert_eq!(
            error,
            SlashRegistryError::AliasCollidesWithCanonical {
                alias: "/resume".to_string(),
                canonical_name: "/resume".to_string(),
            }
        );
    }

    #[test]
    fn registry_supports_canonical_and_alias_lookup() {
        let mut registry = SlashCommandRegistry::empty();
        registry
            .register(registration(
                "/resume",
                &["/continue"],
                "resume a session",
                SlashCommandArgumentShape::OptionalTargetThenText,
            ))
            .unwrap();

        assert_eq!(
            registry
                .get("/resume")
                .map(|descriptor| descriptor.canonical_name),
            Some("/resume")
        );
        assert_eq!(
            registry
                .get("/continue")
                .map(|descriptor| descriptor.canonical_name),
            Some("/resume")
        );
    }

    #[test]
    fn default_registry_exposes_built_in_descriptors() {
        let registry = default_registry();
        let names = registry
            .iter()
            .map(|descriptor| descriptor.canonical_name)
            .collect::<Vec<_>>();
        let resume = registry
            .get("/resume")
            .expect("/resume descriptor should exist");

        assert_eq!(names, vec!["/resume", "/suspend", "/tldr", "/compact"]);
        assert_eq!(
            Some(resume.argument_shape.clone()),
            Some(SlashCommandArgumentShape::OptionalTargetThenText)
        );
        assert_eq!(
            resume.description,
            "Resume a suspended session or list resumable sessions."
        );
        assert_eq!(
            resume.requirements.capability_tags,
            vec!["session-lifecycle"]
        );
        assert_eq!(
            resume.requirements.permission_tags,
            vec!["verifiable-caller-identity"]
        );
        assert_eq!(resume.requirements.backend_tags, vec!["sqlite"]);

        let resume_alias = registry
            .get("/continue")
            .expect("/continue alias should resolve to /resume");
        assert_eq!(resume_alias.canonical_name, "/resume");

        assert_eq!(
            registry
                .get("/compact")
                .map(|descriptor| descriptor.argument_shape.clone()),
            Some(SlashCommandArgumentShape::OptionalText)
        );
    }

    #[tokio::test]
    async fn dispatch_validates_argument_shape_for_exact_commands() {
        let service = SessionCommandService::new(&RegistryMemory);

        let result = default_registry()
            .dispatch(
                &service,
                CommandContext {
                    session_id: "session-1",
                    caller_token_hash: None,
                },
                "/tldr extra args",
            )
            .await;

        assert!(matches!(
            result,
            Some(Err(SessionCommandError::InvalidArguments {
                command: "/tldr",
                ..
            }))
        ));
    }

    #[tokio::test]
    async fn dispatch_routes_built_ins_to_existing_service_behavior() {
        let memory = CountingRegistryMemory::default();
        let service = SessionCommandService::new(&memory);

        let result = default_registry()
            .dispatch(
                &service,
                CommandContext {
                    session_id: "session-1",
                    caller_token_hash: None,
                },
                "/compact keep latest goals",
            )
            .await
            .expect("built-in command should resolve")
            .unwrap_err();

        assert_eq!(
            result,
            SessionCommandError::UnsupportedBackend {
                backend: "none".to_string(),
            }
        );
        assert!(memory.name_calls.load(Ordering::SeqCst) >= 1);
    }

    #[tokio::test]
    async fn dispatch_preserves_resume_authorization_after_registry_lookup() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let memory = crate::memory::SqliteMemory::new(temp_dir.path()).unwrap();
        let service = SessionCommandService::new(&memory);

        let result = default_registry()
            .dispatch(
                &service,
                CommandContext {
                    session_id: "session-1",
                    caller_token_hash: None,
                },
                "/resume",
            )
            .await
            .expect("built-in command should resolve")
            .unwrap_err();

        assert_eq!(result, SessionCommandError::Unauthorized);
    }

    #[tokio::test]
    async fn dispatch_resolves_alias_to_canonical_handler() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let memory = crate::memory::SqliteMemory::new(temp_dir.path()).unwrap();
        let service = SessionCommandService::new(&memory);
        let mut registry = SlashCommandRegistry::empty();
        registry
            .register(SlashCommandRegistration {
                descriptor: SlashCommandDescriptor {
                    canonical_name: "/resume",
                    aliases: &["/continue"],
                    description: "resume a session",
                    argument_shape: SlashCommandArgumentShape::OptionalTargetThenText,
                    requirements: SlashCommandRequirements::default(),
                },
                handler: Arc::new(ResumeHandler),
            })
            .unwrap();

        let result = registry
            .dispatch(
                &service,
                CommandContext {
                    session_id: "session-1",
                    caller_token_hash: None,
                },
                "/continue",
            )
            .await
            .expect("alias should resolve to canonical command")
            .unwrap_err();

        assert_eq!(result, SessionCommandError::Unauthorized);
    }

    #[tokio::test]
    async fn dispatch_routes_suspend_via_registry() {
        let memory = CountingRegistryMemory::default();
        let service = SessionCommandService::new(&memory);

        let result = default_registry()
            .dispatch(
                &service,
                CommandContext {
                    session_id: "session-1",
                    caller_token_hash: None,
                },
                "/suspend",
            )
            .await
            .expect("built-in command should resolve")
            .unwrap_err();

        assert_eq!(
            result,
            SessionCommandError::UnsupportedBackend {
                backend: "none".to_string(),
            }
        );
        assert!(memory.name_calls.load(Ordering::SeqCst) >= 1);
    }

    struct RegistryMemory;

    #[async_trait]
    impl Memory for RegistryMemory {
        fn name(&self) -> &str {
            "none"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[derive(Default)]
    struct CountingRegistryMemory {
        name_calls: AtomicUsize,
    }

    #[async_trait]
    impl Memory for CountingRegistryMemory {
        fn name(&self) -> &str {
            self.name_calls.fetch_add(1, Ordering::SeqCst);
            "none"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }
}
