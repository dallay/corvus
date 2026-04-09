<script lang="ts" setup>
import type {LocalMemoryTimelineGroup} from "@/types/admin-sessions";

const props = defineProps<{
  groups: LocalMemoryTimelineGroup[];
  activeSessionId?: string;
  activeCategory?: string;
}>();

const emit = defineEmits<{
  "select-session": [sessionId?: string];
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function selectSession(sessionId?: string | null) {
  emit("select-session", sessionId ?? undefined);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function isSelected(sessionId?: string | null) {
  return props.activeSessionId === (sessionId ?? undefined);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function visibleEntries(group: LocalMemoryTimelineGroup) {
  if (!props.activeCategory) {
    return group.entries;
  }

  return group.entries.filter((entry) => entry.category === props.activeCategory);
}
</script>

<template>
  <section class="timeline-panel">
    <header class="panel-header">
      <div>
        <p class="eyebrow">Local Memory Timeline</p>
        <h3>Chronological session lanes</h3>
      </div>
    </header>

    <div class="timeline-groups">
      <article
          v-for="group in groups"
          :key="group.label"
          :class="{ 'timeline-group-active': activeSessionId === group.sessionId }"
          class="timeline-group"
          data-testid="timeline-group"
      >
        <button
            :aria-pressed="isSelected(group.sessionId)"
            class="timeline-group-button"
            type="button"
            @click="selectSession(group.sessionId)"
        >
          <span class="timeline-group-label">{{ group.label }}</span>
          <span class="timeline-group-meta">{{ group.entryCount }} entries</span>
        </button>

        <ol class="timeline-entry-list">
          <li
              v-for="entry in visibleEntries(group)"
              :key="entry.id"
              class="timeline-entry"
              data-testid="timeline-entry"
          >
            <span class="timeline-entry-time">{{ entry.timestamp }}</span>
            <span class="timeline-entry-key">{{ entry.key }}</span>
            <span class="timeline-entry-category">{{ entry.category }}</span>
          </li>
        </ol>
      </article>
    </div>
  </section>
</template>

<style scoped>
.timeline-panel {
  display: grid;
  gap: 12px;
}

.panel-header h3,
.eyebrow {
  margin: 0;
}

.eyebrow {
  color: var(--color-text-secondary);
  font-size: 11px;
  text-transform: uppercase;
}

.timeline-groups {
  display: grid;
  gap: 10px;
}

.timeline-group {
  border: 1px solid var(--color-border);
  border-radius: 12px;
  padding: 10px;
}

.timeline-group-active {
  border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-border));
}

.timeline-group-button {
  align-items: center;
  background: transparent;
  border: 0;
  color: inherit;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  padding: 0 0 8px;
  width: 100%;
}

.timeline-entry-list {
  display: grid;
  gap: 6px;
  list-style: none;
  margin: 0;
  padding: 0;
}

.timeline-entry {
  background: color-mix(in srgb, var(--color-bg-secondary) 80%, transparent);
  border-radius: 8px;
  display: grid;
  font-size: 12px;
  gap: 8px;
  grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr) auto;
  padding: 8px;
}

.timeline-entry-time,
.timeline-group-meta,
.timeline-entry-category {
  color: var(--color-text-secondary);
}

.timeline-entry-key {
  font-family: "JetBrains Mono", monospace;
}
</style>
