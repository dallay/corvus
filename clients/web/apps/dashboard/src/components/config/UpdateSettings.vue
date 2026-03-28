<script setup lang="ts">
import type { AdminConfigView } from "@/types/admin-config";

defineProps<{
  config: AdminConfigView;
}>();
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.updates") }}</h2>
    <div class="grid">
      <label>
        <span>{{ $t("updates.currentVersion") }}</span>
        <p data-testid="updates_current_version">
          {{ config.updates?.status?.current_version ?? "—" }}
        </p>
      </label>
      <label>
        <span>{{ $t("updates.latestVersion") }}</span>
        <p data-testid="updates_latest_version">
          {{ config.updates?.status?.latest_version ?? $t("updates.unknown") }}
        </p>
      </label>
      <label>
        <span>{{ $t("updates.updateAvailable") }}</span>
        <p data-testid="updates_update_available">
          {{ config.updates?.status?.update_available ? $t("updates.yes") : $t("updates.no") }}
        </p>
      </label>
      <label>
        <span>{{ $t("updates.autoInstallEnabled") }}</span>
        <p data-testid="updates_auto_install_enabled">
          {{ config.updates?.auto_install_enabled ? $t("updates.yes") : $t("updates.no") }}
        </p>
      </label>
      <label>
        <span>{{ $t("updates.restartPolicy") }}</span>
        <p data-testid="updates_restart_policy">
          {{ config.updates?.restart_policy ?? "—" }}
        </p>
      </label>
      <label>
        <span>{{ $t("updates.lastCheckOutcome") }}</span>
        <p data-testid="updates_last_check_outcome">
          {{ config.updates?.status?.last_check_outcome ?? $t("updates.never") }}
        </p>
      </label>
      <label>
        <span>{{ $t("updates.effectiveInstallMethod") }}</span>
        <p data-testid="updates_effective_install_method">
          {{ config.updates?.status?.effective_install_method ?? "—" }}
        </p>
      </label>
    </div>
  </section>
</template>
