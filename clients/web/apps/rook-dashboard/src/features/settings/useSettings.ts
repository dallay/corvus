import { computed, ref } from "vue";

import type { RookApi } from "@/lib/api/client";
import type { SettingsView } from "@/lib/api/types";

function cloneSettings(settings: SettingsView): SettingsView {
  return {
    gateway_port: settings.gateway_port,
    default_routing_policy: {
      strategy: settings.default_routing_policy.strategy,
      max_retries: settings.default_routing_policy.max_retries,
      cooldown_seconds: settings.default_routing_policy.cooldown_seconds,
    },
    log_json: settings.log_json,
    log_level: settings.log_level,
  };
}

export function useSettings(client: RookApi) {
  const current = ref<SettingsView | null>(null);
  const draft = ref<SettingsView | null>(null);
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const saveError = ref<string | null>(null);
  const saveSuccess = ref<string | null>(null);

  const isDirty = computed(() => JSON.stringify(current.value) !== JSON.stringify(draft.value));

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;

    try {
      const nextSettings = await client.getSettings();
      current.value = cloneSettings(nextSettings);
      draft.value = cloneSettings(nextSettings);
    } catch (loadError) {
      current.value = null;
      draft.value = null;
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  async function save(): Promise<void> {
    if (!draft.value) {
      return;
    }

    saving.value = true;
    saveError.value = null;
    saveSuccess.value = null;

    try {
      const persisted = await client.updateSettings(cloneSettings(draft.value));
      current.value = cloneSettings(persisted);
      draft.value = cloneSettings(persisted);
      saveSuccess.value = "Settings saved.";
    } catch (mutationError) {
      saveError.value =
        mutationError instanceof Error ? mutationError.message : String(mutationError);
    } finally {
      saving.value = false;
    }
  }

  function resetDraft(): void {
    if (!current.value) {
      return;
    }

    draft.value = cloneSettings(current.value);
    saveError.value = null;
    saveSuccess.value = null;
  }

  return {
    current,
    draft,
    loading,
    saving,
    error,
    saveError,
    saveSuccess,
    isDirty,
    load,
    save,
    resetDraft,
  };
}
