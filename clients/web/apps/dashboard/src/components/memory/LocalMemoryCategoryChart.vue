<script lang="ts" setup>
import type {LocalMemoryCategoryFacet} from "@/types/admin-sessions";

defineProps<{
  facets: LocalMemoryCategoryFacet[];
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const emit = defineEmits<{
  "select-category": [category: string];
  "clear-category": [];
}>();
</script>

<template>
  <section class="category-panel">
    <header class="panel-header">
      <div>
        <p class="eyebrow">Local categories</p>
        <h3>Category breakdown</h3>
      </div>
      <button
          v-if="facets.some((facet) => facet.isActive)"
          class="clear-category-focus"
          type="button"
          @click="emit('clear-category')"
      >
        Clear focus
      </button>
    </header>

    <div class="category-list">
      <button
          v-for="facet in facets"
          :key="facet.category"
          :class="{ 'category-bar-active': facet.isActive }"
          class="category-bar"
          type="button"
          @click="emit('select-category', facet.category)"
      >
        <span class="category-label">{{ facet.category }}</span>
        <span class="category-meta"
        >{{ facet.total }} · {{ facet.sessionCount }}
          {{ facet.sessionCount === 1 ? "session" : "sessions" }}</span
        >
      </button>
    </div>
  </section>
</template>

<style scoped>
.category-panel,
.category-list {
  display: grid;
  gap: 10px;
}

.panel-header {
  align-items: center;
  display: flex;
  gap: 12px;
  justify-content: space-between;
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

.category-bar,
.clear-category-focus {
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  color: inherit;
}

.category-bar {
  cursor: pointer;
  display: flex;
  gap: 12px;
  justify-content: space-between;
  padding: 10px 12px;
}

.category-bar-active {
  border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-border));
}

.category-meta {
  color: var(--color-text-secondary);
  font-size: 12px;
}

.clear-category-focus {
  cursor: pointer;
  padding: 6px 10px;
}
</style>
