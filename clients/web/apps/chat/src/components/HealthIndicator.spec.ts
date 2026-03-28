import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import HealthIndicator from "@/components/HealthIndicator.vue";
import { i18nConfig } from "@/i18n";

const testI18n = createI18n(i18nConfig);
const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

function mountIndicator(props = {}) {
  return mount(HealthIndicator, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
      ...props,
    },
    global: {
      plugins: [testI18n],
    },
  });
}

describe("HealthIndicator", () => {
  it("shows connected on successful health check", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ status: "ok" }), { status: 200 })
    );

    const wrapper = mountIndicator();
    await flushPromises();

    expect(wrapper.get('[data-testid="health-status"]').classes()).toContain("connected");
    expect(fetchMock).toHaveBeenCalledWith("http://localhost:3000/health", {
      method: "GET",
      headers: { Authorization: "Bearer test-token" },
    });
  });

  it("shows disconnected on health check failure", async () => {
    fetchMock.mockRejectedValueOnce(new Error("Network error"));

    const wrapper = mountIndicator();
    await flushPromises();

    expect(wrapper.get('[data-testid="health-status"]').classes()).toContain("disconnected");
  });

  it("shows disconnected on non-OK HTTP response", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ status: "error" }), { status: 500 })
    );

    const wrapper = mountIndicator();
    await flushPromises();

    expect(wrapper.get('[data-testid="health-status"]').classes()).toContain("disconnected");
  });

  it("strips trailing slashes from gateway URL", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ status: "ok" }), { status: 200 })
    );

    mountIndicator({ gatewayUrl: "http://localhost:3000/" });
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledWith("http://localhost:3000/health", {
      method: "GET",
      headers: { Authorization: "Bearer test-token" },
    });
  });
});
