<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { AdminConfigView } from "@/types/admin-config";

defineProps<{
  config: AdminConfigView;
}>();

const { t } = useI18n();

function formatToggle(value: boolean | undefined): string {
  if (value === true) {
    return t("updates.yes");
  }

  if (value === false) {
    return t("updates.no");
  }

  return t("updates.unknown");
}
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.updates") }}</h2>
    <div class="grid">
      <div class="display-field">
        <span>{{ $t("updates.currentVersion") }}</span>
        <p data-testid="updates_current_version">
          {{ config.updates?.status?.current_version ?? "—" }}
        </p>
      </div>
      <div class="display-field">
        <span>{{ $t("updates.latestVersion") }}</span>
        <p data-testid="updates_latest_version">
          {{ config.updates?.status?.latest_version ?? $t("updates.unknown") }}
        </p>
      </div>
      <div class="display-field">
        <span>{{ $t("updates.updateAvailable") }}</span>
        <p data-testid="updates_update_available">
          {{ formatToggle(config.updates?.status?.update_available) }}
        </p>
      </div>
      <div class="display-field">
        <span>{{ $t("updates.autoInstallEnabled") }}</span>
        <p data-testid="updates_auto_install_enabled">
          {{ formatToggle(config.updates?.auto_install_enabled) }}
        </p>
      </div>
      <div class="display-field">
        <span>{{ $t("updates.restartPolicy") }}</span>
        <p data-testid="updates_restart_policy">
          {{ config.updates?.restart_policy ?? "—" }}
        </p>
      </div>
      <div class="display-field">
        <span>{{ $t("updates.lastCheckOutcome") }}</span>
        <p data-testid="updates_last_check_outcome">
          {{ config.updates?.status?.last_check_outcome ?? $t("updates.never") }}
        </p>
      </div>
      <div class="display-field">
        <span>{{ $t("updates.effectiveInstallMethod") }}</span>
        <p data-testid="updates_effective_install_method">
          {{ config.updates?.status?.effective_install_method ?? "—" }}
        </p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.display-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.display-field p,
.display-field span {
  margin: 0;
}
</style>
