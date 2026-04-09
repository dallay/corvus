<script setup lang="ts">
import type { LocalMemoryTimelineGroup } from "@/types/admin-sessions";

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
        class="timeline-group"
        :class="{ 'timeline-group-active': activeSessionId === group.sessionId }"
        data-testid="timeline-group"
      >
        <button class="timeline-group-button" type="button" @click="selectSession(group.sessionId)">
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
  font-size: 11px;
  text-transform: uppercase;
  color: var(--color-text-secondary);
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
  display: flex;
  width: 100%;
  justify-content: space-between;
  align-items: center;
  background: transparent;
  border: 0;
  padding: 0 0 8px;
  cursor: pointer;
  color: inherit;
}

.timeline-entry-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 6px;
}

.timeline-entry {
  display: grid;
  grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr) auto;
  gap: 8px;
  font-size: 12px;
  padding: 8px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--color-bg-secondary) 80%, transparent);
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
