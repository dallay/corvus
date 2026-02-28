# Sentinnel Journal

Critical learnings only. Do not log routine work.

## Entry Format

```markdown
## YYYY-MM-DD - [Title]
**Learning:** [Insight]
**Action:** [How to apply next time]
```

## 2025-02-13 - Inconsistent Tool Security Enforcement
**Learning:** Tools that interact with the internet (like WebSearchTool) can easily be overlooked when standardizing security policy enforcement, leading to rate-limit bypasses and autonomy violations.
**Action:** Always verify that any new or existing tool that performs external actions or consumes resources is integrated with `Arc<SecurityPolicy>` and calls `can_act()` and `record_action()`.
