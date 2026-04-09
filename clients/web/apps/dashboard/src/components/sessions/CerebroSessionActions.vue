<script setup lang="ts">
import { useAdmin } from "@/composables/useAdmin";
import type { AdminCerebroStatusResponse, CerebroToolName } from "@/types/admin-sessions";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
  sessionId: string;
  status: AdminCerebroStatusResponse | null;
}>();

const admin = useAdmin(props.gatewayUrl, props.authHeaders);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const sessionTools: Array<{ tool: CerebroToolName; label: string }> = [
  { tool: "mem_session_start", label: "Session Start" },
  { tool: "mem_session_end", label: "Session End" },
  { tool: "mem_session_summary", label: "Session Summary" },
  { tool: "mem_context", label: "Context Lookup" },
];

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function invoke(tool: CerebroToolName) {
  if (tool === "mem_context") {
    await admin.invokeCerebroContext({ session_id: props.sessionId, limit: 5 });
    return;
  }
  await admin.invokeCerebroSessionAction(
    tool as "mem_session_start" | "mem_session_end" | "mem_session_summary",
    props.sessionId
  );
}
</script>

<template>
  <section class="cerebro-session-card">
    <div class="header-row">
      <div>
        <p class="eyebrow">Cerebro Memory</p>
        <h4>Session Enhancements</h4>
      </div>
      <span class="helper">Local session facts stay primary.</span>
    </div>

    <ul class="action-list">
      <li v-for="entry in sessionTools" :key="entry.tool" class="action-row">
        <div>
          <strong>{{ entry.label }}</strong>
          <p class="helper">{{ status?.tools[entry.tool]?.message ?? status?.tools[entry.tool]?.state }}</p>
        </div>
        <button :disabled="status?.tools[entry.tool]?.state !== 'available'" @click="invoke(entry.tool)">
          {{ status?.tools[entry.tool]?.state === "available" ? "Run" : status?.tools[entry.tool]?.state }}
        </button>
      </li>
    </ul>

    <div
      v-if="admin.cerebroLastAction.value && 'state' in admin.cerebroLastAction.value && admin.cerebroLastAction.value.state !== 'available'"
      class="helper"
    >
      {{ admin.cerebroLastAction.value.message }}
    </div>
    <pre
      v-else-if="admin.cerebroLastAction.value && 'data' in admin.cerebroLastAction.value"
      class="payload"
    >{{ JSON.stringify(admin.cerebroLastAction.value.data, null, 2) }}</pre>
  </section>
</template>

<style scoped>
.cerebro-session-card {
  margin-top: 16px;
  border: 1px solid var(--color-border);
  border-radius: 12px;
  padding: 14px;
}

.header-row {
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

.header-row h4 {
  margin: 0;
}

.action-list {
  list-style: none;
  margin: 12px 0 0;
  padding: 0;
  display: grid;
  gap: 10px;
}

.action-row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
}

.action-row p {
  margin: 4px 0 0;
}

.payload {
  margin: 12px 0 0;
  padding: 10px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-bg-secondary) 85%, transparent);
  overflow: auto;
  font-size: 12px;
}
</style>
