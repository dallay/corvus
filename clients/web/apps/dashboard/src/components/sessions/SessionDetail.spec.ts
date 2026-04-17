import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import SessionDetail from "@/components/sessions/SessionDetail.vue";
import { i18nConfig } from "@/i18n";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

function mockDetailResponse(detail: Record<string, unknown>) {
  fetchMock
    .mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          session: detail,
          memory_summary: (detail.memory_summary as Record<string, unknown> | undefined) ?? {},
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }
      )
    )
    .mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          service_state: "available",
          tools: {
            mem_search: { state: "available" },
            mem_get_observation: { state: "available" },
            mem_timeline: { state: "available" },
            mem_stats: { state: "available" },
            mem_save: { state: "available" },
            mem_update: { state: "available" },
            mem_delete: { state: "available" },
            mem_save_prompt: { state: "not_implemented" },
            mem_session_start: { state: "not_implemented" },
            mem_session_end: { state: "not_implemented" },
            mem_session_summary: { state: "not_implemented" },
            mem_context: { state: "available" },
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } }
      )
    );
}

function deferredResponse(detail: Record<string, unknown>) {
  let resolve: ((value: Response) => void) | undefined;
  const promise = new Promise<Response>((innerResolve) => {
    resolve = innerResolve;
  });

  return {
    promise,
    resolve: () =>
      resolve?.(
        new Response(JSON.stringify(detail), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      ),
  };
}

function mountDetail(sessionId: string) {
  return mount(SessionDetail, {
    props: {
      gatewayUrl: (path: string) => new URL(`http://localhost:3000/api${path}`).toString(),
      authHeaders: () => ({ Authorization: "Bearer token" }),
      sessionId,
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("SessionDetail", () => {
  it("renders session metadata fields for an active session", async () => {
    mockDetailResponse({
      id: "abc-123",
      started_at: "2026-01-01T10:00:00Z",
      ended_at: null,
      status: "active",
      message_count: 15,
      last_activity: "2026-01-02T12:00:00Z",
      memory_summary: { Conversation: 4, Core: 2 },
    });

    const wrapper = mountDetail("abc-123");
    await flushPromises();

    expect(wrapper.text()).toContain("abc-123");
    expect(wrapper.text()).toContain("15");
    expect(wrapper.text()).toContain("Active");
    expect(wrapper.find(".status-active").exists()).toBe(true);
  });

  it("renders ended session with ended_at timestamp", async () => {
    mockDetailResponse({
      id: "old-session",
      started_at: "2026-03-27T10:00:00Z",
      ended_at: "2026-03-27T18:00:00Z",
      status: "ended",
      message_count: 30,
      last_activity: "2026-03-27T17:50:00Z",
      memory_summary: {},
    });

    const wrapper = mountDetail("old-session");
    await flushPromises();

    expect(wrapper.text()).toContain("2026-03-27T18:00:00Z");
    expect(wrapper.text()).toContain("Ended");
    expect(wrapper.find(".status-ended").exists()).toBe(true);
  });

  it("displays memory summary with category counts", async () => {
    mockDetailResponse({
      id: "abc-123",
      started_at: "2026-01-01",
      ended_at: null,
      status: "active",
      message_count: 15,
      last_activity: "2026-01-02",
      memory_summary: { Conversation: 4, Core: 2 },
    });

    const wrapper = mountDetail("abc-123");
    await flushPromises();

    expect(wrapper.text()).toContain("Memory Summary");
    expect(wrapper.text()).toContain("Conversation");
    expect(wrapper.text()).toContain("4");
    expect(wrapper.text()).toContain("Core");
    expect(wrapper.text()).toContain("2");
    expect(wrapper.text()).toContain("Session Enhancements");
    expect(wrapper.text()).toContain("not_implemented");
  });

  it("shows 0 entries message when memory summary is empty", async () => {
    mockDetailResponse({
      id: "empty-session",
      started_at: "2026-01-01",
      ended_at: null,
      status: "active",
      message_count: 0,
      last_activity: "2026-01-01",
      memory_summary: {},
    });

    const wrapper = mountDetail("empty-session");
    await flushPromises();

    expect(wrapper.text()).toContain("0 entries");
  });

  it("emits close event when close button is clicked", async () => {
    mockDetailResponse({
      id: "abc-123",
      started_at: "2026-01-01",
      ended_at: null,
      status: "active",
      message_count: 0,
      last_activity: "2026-01-01",
      memory_summary: {},
    });

    const wrapper = mountDetail("abc-123");
    await flushPromises();

    await wrapper.find(".close-btn").trigger("click");
    expect(wrapper.emitted("close")).toHaveLength(1);
  });

  it("applies minimum target class to the close control", async () => {
    mockDetailResponse({
      id: "abc-123",
      started_at: "2026-01-01",
      ended_at: null,
      status: "active",
      message_count: 0,
      last_activity: "2026-01-01",
      memory_summary: {},
    });

    const wrapper = mountDetail("abc-123");
    await flushPromises();

    expect(wrapper.find(".close-btn").classes()).toContain("touch-target");
  });

  it("emits view-memory event when View Memory Entries button is clicked", async () => {
    mockDetailResponse({
      id: "abc-123",
      started_at: "2026-01-01",
      ended_at: null,
      status: "active",
      message_count: 0,
      last_activity: "2026-01-01",
      memory_summary: {},
    });

    const wrapper = mountDetail("abc-123");
    await flushPromises();

    await wrapper.find(".view-memory-btn").trigger("click");
    const emitted = wrapper.emitted("view-memory");
    expect(emitted).toHaveLength(1);
    expect(emitted?.[0]?.[0]).toBe("abc-123");
  });

  it("keeps the newest session detail when requests resolve out of order", async () => {
    const firstResponse = deferredResponse({
      session: {
        id: "older-session",
        started_at: "2026-01-01",
        ended_at: null,
        status: "active",
        message_count: 1,
        last_activity: "2026-01-02",
        metadata: { source: "first" },
      },
      memory_summary: { Conversation: 1 },
    });
    const secondResponse = deferredResponse({
      session: {
        id: "newer-session",
        started_at: "2026-01-03",
        ended_at: null,
        status: "active",
        message_count: 2,
        last_activity: "2026-01-04",
        metadata: { source: "second" },
      },
      memory_summary: { Core: 2 },
    });

    fetchMock
      .mockImplementationOnce(() => firstResponse.promise)
      .mockImplementationOnce(() =>
        Promise.resolve(
          new Response(
            JSON.stringify({
              service_state: "available",
              tools: {
                mem_search: { state: "available" },
                mem_get_observation: { state: "available" },
                mem_timeline: { state: "available" },
                mem_stats: { state: "available" },
                mem_save: { state: "available" },
                mem_update: { state: "available" },
                mem_delete: { state: "available" },
                mem_save_prompt: { state: "not_implemented" },
                mem_session_start: { state: "not_implemented" },
                mem_session_end: { state: "not_implemented" },
                mem_session_summary: { state: "not_implemented" },
                mem_context: { state: "available" },
              },
            }),
            { status: 200, headers: { "Content-Type": "application/json" } }
          )
        )
      )
      .mockImplementationOnce(() => secondResponse.promise)
      .mockImplementationOnce(() =>
        Promise.resolve(
          new Response(
            JSON.stringify({
              service_state: "available",
              tools: {
                mem_search: { state: "available" },
                mem_get_observation: { state: "available" },
                mem_timeline: { state: "available" },
                mem_stats: { state: "available" },
                mem_save: { state: "available" },
                mem_update: { state: "available" },
                mem_delete: { state: "available" },
                mem_save_prompt: { state: "not_implemented" },
                mem_session_start: { state: "not_implemented" },
                mem_session_end: { state: "not_implemented" },
                mem_session_summary: { state: "not_implemented" },
                mem_context: { state: "available" },
              },
            }),
            { status: 200, headers: { "Content-Type": "application/json" } }
          )
        )
      );

    const wrapper = mountDetail("older-session");
    await wrapper.setProps({ sessionId: "newer-session" });

    secondResponse.resolve();
    await flushPromises();
    expect(wrapper.text()).toContain("newer-session");
    expect(wrapper.text()).toContain("2");

    firstResponse.resolve();
    await flushPromises();
    expect(wrapper.text()).toContain("newer-session");
    expect(wrapper.text()).not.toContain("older-session");
  });
});
