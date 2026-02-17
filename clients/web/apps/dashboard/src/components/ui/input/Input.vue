<script setup lang="ts">
import { computed } from "vue";

import { cn } from "@/lib/utils";

const props = defineProps<{
  class?: string;
  modelValue?: string;
  placeholder?: string;
  type?: string;
}>();

const _emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const _classes = computed(() =>
  cn(
    "flex h-10 w-full rounded-md border border-slate-200 bg-white px-3 py-2 text-sm ring-offset-white file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-500 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
    props.class
  )
);
</script>

<template>
  <input
    :class="_classes"
    :type="type ?? 'text'"
    :value="modelValue"
    :placeholder="placeholder"
    @input="_emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>
