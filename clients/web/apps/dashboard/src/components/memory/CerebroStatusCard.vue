<script setup lang="ts">
import type { AdminCerebroStatusResponse, CerebroToolName } from "@/types/admin-sessions";

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const props = defineProps<{
  status: AdminCerebroStatusResponse | null;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const orderedTools: CerebroToolName[] = [
  "mem_search",
  "mem_get_observation",
  "mem_timeline",
  "mem_stats",
  "mem_save",
  "mem_update",
  "mem_delete",
  "mem_context",
  "mem_session_start",
  "mem_session_end",
  "mem_session_summary",
  "mem_save_prompt",
];
</script>

<template>
  <section class="cerebro-status-card">
    <header class="card-header">
      <div>
        <p class="eyebrow">Cerebro Memory</p>
        <h4>Status</h4>
      </div>
      <span v-if="status" class="state-pill" :data-state="status.service_state">
        {{ status.service_state }}
      </span>
    </header>

    <p class="helper">
      Cerebro is an admin-only long-term memory enhancement. Local SQLite memory remains authoritative.
    </p>

    <ul v-if="status" class="tool-list">
      <li v-for="tool in orderedTools" :key="tool" class="tool-row">
        <span class="tool-name">{{ tool }}</span>
        <span class="state-pill" :data-state="status.tools[tool]?.state ?? 'unsupported'">
          {{ status.tools[tool]?.state ?? "unsupported" }}
        </span>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.cerebro-status-card {
  border: 1px solid var(--color-border);
  border-radius: 12px;
  padding: 14px;
  background: color-mix(in srgb, var(--color-bg-secondary) 85%, transparent);
}

.card-header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
}

.eyebrow {
  margin: 0 0 4px;
  font-size: 11px;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.card-header h4 {
  margin: 0;
}

.tool-list {
  list-style: none;
  margin: 12px 0 0;
  padding: 0;
  display: grid;
  gap: 8px;
}

.tool-row {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  align-items: center;
}

.tool-name {
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}

.state-pill {
  border-radius: 999px;
  padding: 2px 8px;
  font-size: 11px;
  text-transform: uppercase;
  border: 1px solid var(--color-border);
}

.state-pill[data-state="available"] {
  color: #22c55e;
}

.state-pill[data-state="unconfigured"],
.state-pill[data-state="unsupported"],
.state-pill[data-state="not_implemented"] {
  color: #f59e0b;
}

.state-pill[data-state="unreachable"] {
  color: #ef4444;
}
</style>
