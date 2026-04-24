import { ref } from "vue";

import type { RookApi } from "@/lib/api/client";
import type { UsageStatusView } from "@/lib/api/types";

export function useUsage(client: RookApi) {
  const usage = ref<UsageStatusView | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;

    try {
      usage.value = await client.getUsage();
    } catch (loadError) {
      usage.value = null;
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  return {
    usage,
    loading,
    error,
    load,
  };
}
