import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import SessionList from "@/components/sessions/SessionList.vue";
import { i18nConfig } from "@/i18n";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

function mockSessionsResponse(
  items: Array<{
    id: string;
    started_at: string;
    last_activity: string;
    message_count: number;
    status: "active" | "ended";
  }>,
  total?: number
) {
  fetchMock.mockResolvedValueOnce(
    new Response(
      JSON.stringify({
        sessions: items,
        total: total ?? items.length,
        limit: 50,
        offset: 0,
      }),
      { status: 200, headers: { "Content-Type": "application/json" } }
    )
  );
}

function mountSessionList(props?: Record<string, unknown>) {
  return mount(SessionList, {
    props: {
      gatewayUrl: (path: string) => new URL(`http://localhost:3000/api${path}`).toString(),
      authHeaders: () => ({ Authorization: "Bearer token" }),
      ...props,
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("SessionList", () => {
  it("renders table with session rows", async () => {
    mockSessionsResponse([
      {
        id: "s1",
        started_at: "2026-01-01",
        last_activity: "2026-01-02",
        message_count: 5,
        status: "active",
      },
      {
        id: "s2",
        started_at: "2026-01-03",
        last_activity: "2026-01-04",
        message_count: 3,
        status: "ended",
      },
    ]);

    const wrapper = mountSessionList();
    await flushPromises();

    expect(wrapper.find("table.session-table").exists()).toBe(true);
    expect(wrapper.findAll("tbody tr")).toHaveLength(2);
    expect(wrapper.text()).toContain("s1");
    expect(wrapper.text()).toContain("s2");
  });

  it("displays empty state message when no sessions", async () => {
    mockSessionsResponse([]);

    const wrapper = mountSessionList();
    await flushPromises();

    expect(wrapper.find("table").exists()).toBe(false);
    expect(wrapper.text()).toContain("No sessions found");
  });

  it("shows active/ended visual distinction via status badges", async () => {
    mockSessionsResponse([
      {
        id: "s1",
        started_at: "2026-01-01",
        last_activity: "2026-01-02",
        message_count: 5,
        status: "active",
      },
      {
        id: "s2",
        started_at: "2026-01-03",
        last_activity: "2026-01-04",
        message_count: 3,
        status: "ended",
      },
    ]);

    const wrapper = mountSessionList();
    await flushPromises();

    const badges = wrapper.findAll(".status-badge");
    expect(badges).toHaveLength(2);
    expect(badges[0]?.classes()).toContain("status-active");
    expect(badges[1]?.classes()).toContain("status-ended");
  });

  it("emits select event on row click", async () => {
    mockSessionsResponse([
      {
        id: "s1",
        started_at: "2026-01-01",
        last_activity: "2026-01-02",
        message_count: 5,
        status: "active",
      },
    ]);

    const wrapper = mountSessionList();
    await flushPromises();

    await wrapper.find('[data-testid="session-s1"] .select-btn').trigger("click");
    const emitted = wrapper.emitted("select");
    expect(emitted).toHaveLength(1);
    expect(emitted?.[0]?.[0]).toEqual(expect.objectContaining({ id: "s1" }));
  });

  it("shows pagination when total exceeds page size", async () => {
    mockSessionsResponse(
      Array.from({ length: 25 }, (_, i) => ({
        id: `s${i}`,
        started_at: "2026-01-01",
        last_activity: "2026-01-02",
        message_count: 1,
        status: "active" as const,
      })),
      60
    );

    const wrapper = mountSessionList();
    await flushPromises();

    expect(wrapper.find(".pagination").exists()).toBe(true);
    expect(wrapper.text()).toContain("Page");
    expect(wrapper.text()).toContain("60");
  });

  it("does not show pagination when sessions fit in one page", async () => {
    mockSessionsResponse([
      {
        id: "s1",
        started_at: "2026-01-01",
        last_activity: "2026-01-02",
        message_count: 5,
        status: "active",
      },
    ]);

    const wrapper = mountSessionList();
    await flushPromises();

    expect(wrapper.find(".pagination").exists()).toBe(false);
  });

  it("allows changing page size from the pagination controls", async () => {
    mockSessionsResponse(
      Array.from({ length: 25 }, (_, i) => ({
        id: `s${i}`,
        started_at: "2026-01-01",
        last_activity: "2026-01-02",
        message_count: 1,
        status: "active" as const,
      })),
      60
    );

    const wrapper = mountSessionList();
    await flushPromises();

    mockSessionsResponse([], 60);
    await wrapper.find(".page-size-select").setValue("50");
    await flushPromises();

    const [url] = fetchMock.mock.calls[1] ?? [];
    const parsed = new URL(url as string);
    expect(parsed.searchParams.get("limit")).toBe("50");
    expect(wrapper.text()).toContain("Rows per page");
  });
});
