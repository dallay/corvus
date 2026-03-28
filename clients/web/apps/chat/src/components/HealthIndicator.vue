<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

const POLL_INTERVAL_MS = 30_000;

const status = ref<"connected" | "disconnected">("disconnected");
let intervalId: ReturnType<typeof setInterval> | null = null;

async function checkHealth(): Promise<void> {
  try {
    const headers: Record<string, string> = {};
    if (props.bearerToken) {
      headers.Authorization = `Bearer ${props.bearerToken}`;
    }

    const response = await fetch(`${props.gatewayUrl}/health`, {
      method: "GET",
      headers,
    });
    status.value = response.ok ? "connected" : "disconnected";
  } catch {
    status.value = "disconnected";
  }
}

onMounted(() => {
  checkHealth();
  intervalId = setInterval(checkHealth, POLL_INTERVAL_MS);
});

onUnmounted(() => {
  if (intervalId !== null) {
    clearInterval(intervalId);
    intervalId = null;
  }
});
</script>

<template>
  <div class="health-indicator" data-testid="health-status" :class="status">
    <span class="health-dot" />
    <span class="health-label">{{
      status === "connected" ? $t("chat.connected") : $t("chat.disconnected")
    }}</span>
  </div>
</template>

<style scoped>
.health-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--color-text-muted);
}

.health-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.health-indicator.connected .health-dot {
  background: #22c55e;
  box-shadow: 0 0 6px rgb(34 197 94 / 40%);
}

.health-indicator.disconnected .health-dot {
  background: #ef4444;
  box-shadow: 0 0 6px rgb(239 68 68 / 40%);
}
</style>
