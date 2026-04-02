<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import type { SessionListItem } from "@/types/chat";

const props = defineProps<{
  sessions: SessionListItem[];
  currentSessionId: string;
  collapsed: boolean;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const emit = defineEmits<{
  "switch-session": [id: string];
  "new-chat": [];
  "toggle-collapse": [];
}>();

const { t } = useI18n();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const sortedSessions = computed(() =>
  [...props.sessions].sort(
    (a, b) => new Date(b.last_activity).getTime() - new Date(a.last_activity).getTime()
  )
);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function formatRelativeTime(isoDate: string): string {
  const now = Date.now();
  const then = new Date(isoDate).getTime();
  const diffMs = now - then;

  if (Number.isNaN(diffMs) || diffMs < 0) {
    return isoDate;
  }

  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 1) return t("session.justNow");
  if (minutes < 60) return t("session.minutesAgo", { count: minutes });

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t("session.hoursAgo", { count: hours });

  const days = Math.floor(hours / 24);
  return t("session.daysAgo", { count: days });
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function truncateId(id: string): string {
  return id.length > 8 ? `${id.slice(0, 8)}...` : id;
}
</script>

<template>
  <aside
    class="session-sidebar"
    :class="{ 'session-sidebar--collapsed': collapsed }"
    :aria-label="t('session.sidebarLabel')"
  >
    <div class="session-sidebar-header">
      <template v-if="!collapsed">
        <span class="session-sidebar-title">{{ t("session.history") }}</span>
      </template>
      <button
        class="session-sidebar-toggle"
        :aria-label="collapsed ? t('session.expand') : t('session.collapse')"
        @click="emit('toggle-collapse')"
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <polyline v-if="collapsed" points="9 18 15 12 9 6" />
          <polyline v-else points="15 18 9 12 15 6" />
        </svg>
      </button>
    </div>

    <template v-if="!collapsed">
      <button
        class="session-sidebar-new-chat"
        @click="emit('new-chat')"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
        {{ t("chat.newChat") }}
      </button>

      <ul class="session-sidebar-list">
        <li
          v-for="session in sortedSessions"
          :key="session.id"
        >
          <button
            :data-testid="`session-item-${session.id}`"
            class="session-sidebar-item"
            :class="{ 'session-sidebar-item--active': session.id === currentSessionId }"
            :aria-current="session.id === currentSessionId ? 'true' : undefined"
            @click="emit('switch-session', session.id)"
          >
            <span class="session-item-id">{{ truncateId(session.id) }}</span>
            <span class="session-item-meta">
              {{ formatRelativeTime(session.last_activity) }}
              · {{ t("session.messageCount", { count: session.message_count }) }}
            </span>
          </button>
        </li>
      </ul>

      <div
        v-if="sortedSessions.length === 0"
        class="session-sidebar-empty"
      >
        {{ t("session.noHistory") }}
      </div>
    </template>
  </aside>
</template>

<style scoped>
.session-sidebar {
  display: flex;
  flex-direction: column;
  width: 240px;
  border-right: 1px solid var(--color-border);
  background: color-mix(in srgb, var(--color-bg-secondary) 92%, transparent);
  overflow: hidden;
  transition: width 0.2s ease;
}

.session-sidebar--collapsed {
  width: 44px;
}

.session-sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px;
  border-bottom: 1px solid var(--color-border);
  min-height: 44px;
}

.session-sidebar--collapsed .session-sidebar-header {
  justify-content: center;
}

.session-sidebar-title {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-text-muted);
}

.session-sidebar-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.2s;
  flex-shrink: 0;
}

.session-sidebar-toggle:hover {
  color: var(--color-text-primary);
  background: var(--color-surface-glass-hover);
}

.session-sidebar-new-chat {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 12px;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px dashed var(--color-border);
  background: transparent;
  color: var(--color-text-secondary);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.2s;
}

.session-sidebar-new-chat:hover {
  color: var(--color-accent);
  border-color: var(--color-border-accent);
  background: var(--color-surface-glass-hover);
}

.session-sidebar-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px;
  list-style: none;
  margin: 0;
}

.session-sidebar-item {
  display: flex;
  flex-direction: column;
  width: 100%;
  gap: 2px;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  text-align: left;
  cursor: pointer;
  transition: all 0.2s;
  font-family: inherit;
}

.session-sidebar-item:hover {
  background: var(--color-surface-glass-hover);
}

.session-sidebar-item--active {
  background: var(--color-surface-glass);
  border-color: var(--color-border-accent);
}

.session-item-id {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-sidebar-item--active .session-item-id {
  color: var(--color-accent);
}

.session-item-meta {
  font-size: 11px;
  color: var(--color-text-muted);
}

.session-sidebar-empty {
  padding: 16px 12px;
  font-size: 12px;
  color: var(--color-text-muted);
  text-align: center;
}
</style>
