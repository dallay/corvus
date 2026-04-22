import { computed, ref } from "vue";

import type { RookApi } from "@/lib/api/client";
import type { CreateRouteRequest, PoolView, RouteView, UpdateRouteRequest } from "@/lib/api/types";

export interface RouteFormInput {
  logical_model: string;
  target_pool_id: string;
  fallback_route_id: string | null;
  capability_constraints: string[];
}

export function availableFallbackRoutes(routes: RouteView[], currentRouteId: string | null) {
  return routes.filter((route) => route.id !== currentRouteId);
}

export function useRoutes(client: RookApi) {
  const routes = ref<RouteView[]>([]);
  const pools = ref<PoolView[]>([]);
  const detail = ref<RouteView | null>(null);
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const actionError = ref<string | null>(null);

  const poolsById = computed(() => new Map(pools.value.map((pool) => [pool.id, pool])));
  const fallbackRouteOptions = computed(() =>
    availableFallbackRoutes(routes.value, detail.value?.id ?? null)
  );

  async function load() {
    loading.value = true;
    error.value = null;

    try {
      const [nextRoutes, nextPools] = await Promise.all([client.listRoutes(), client.listPools()]);
      routes.value = nextRoutes;
      pools.value = nextPools;

      if (detail.value) {
        detail.value = routes.value.find((route) => route.id === detail.value?.id) ?? null;
      }
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  async function openDetail(routeId: string) {
    detail.value = await client.getRoute(routeId);
  }

  async function create(input: CreateRouteRequest) {
    saving.value = true;
    actionError.value = null;

    try {
      await client.createRoute({
        ...input,
        capability_constraints: input.capability_constraints ?? [],
      });
      await load();
    } catch (createError) {
      actionError.value = createError instanceof Error ? createError.message : String(createError);
    } finally {
      saving.value = false;
    }
  }

  async function update(routeId: string, input: UpdateRouteRequest) {
    saving.value = true;
    actionError.value = null;

    try {
      await client.updateRoute(routeId, {
        ...input,
        capability_constraints: input.capability_constraints ?? [],
      });
      await load();
      detail.value = await client.getRoute(routeId);
    } catch (updateError) {
      actionError.value = updateError instanceof Error ? updateError.message : String(updateError);
    } finally {
      saving.value = false;
    }
  }

  async function remove(routeId: string) {
    saving.value = true;
    actionError.value = null;

    try {
      await client.deleteRoute(routeId);
      await load();
      if (detail.value?.id === routeId) {
        detail.value = null;
      }
    } catch (removeError) {
      actionError.value = removeError instanceof Error ? removeError.message : String(removeError);
    } finally {
      saving.value = false;
    }
  }

  return {
    actionError,
    create,
    detail,
    error,
    fallbackRouteOptions,
    load,
    loading,
    openDetail,
    pools,
    poolsById,
    remove,
    routes,
    saving,
    update,
  };
}
