<script setup lang="ts">
import type { LocalMemoryCategoryFacet } from "@/types/admin-sessions";

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
        type="button"
        class="clear-category-focus"
        @click="emit('clear-category')"
      >
        Clear focus
      </button>
    </header>

    <div class="category-list">
      <button
        v-for="facet in facets"
        :key="facet.category"
        type="button"
        class="category-bar"
        :class="{ 'category-bar-active': facet.isActive }"
        @click="emit('select-category', facet.category)"
      >
        <span class="category-label">{{ facet.category }}</span>
        <span class="category-meta">{{ facet.total }} · {{ facet.sessionCount }} sessions</span>
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
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
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

.category-bar,
.clear-category-focus {
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: transparent;
  color: inherit;
}

.category-bar {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  cursor: pointer;
}

.category-bar-active {
  border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-border));
}

.category-meta {
  color: var(--color-text-secondary);
  font-size: 12px;
}

.clear-category-focus {
  padding: 6px 10px;
  cursor: pointer;
}
</style>
