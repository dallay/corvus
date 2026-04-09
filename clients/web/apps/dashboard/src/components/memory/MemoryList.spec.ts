import {flushPromises, mount} from "@vue/test-utils";
import {beforeEach, describe, expect, it, vi} from "vitest";
import {createI18n} from "vue-i18n";

import MemoryList from "@/components/memory/MemoryList.vue";
import {i18nConfig} from "@/i18n";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

function mockMemoryResponse(
    items: Array<{
      id: string;
      key: string;
      content: string;
      category: string;
      timestamp: string;
      session_id?: string | null;
    }>,
    total?: number
) {
  fetchMock.mockResolvedValueOnce(
      new Response(
          JSON.stringify({
            entries: items,
            total: total ?? items.length,
            limit: 50,
            offset: 0,
          }),
          {status: 200, headers: {"Content-Type": "application/json"}}
      )
  );
}

function mountMemoryList(props?: Record<string, unknown>) {
  return mount(MemoryList, {
    attachTo: document.body,
    props: {
      gatewayUrl: (path: string) => new URL(`http://localhost:3000/api${path}`).toString(),
      authHeaders: () => ({Authorization: "Bearer token"}),
      ...props,
    },
    global: {
      plugins: [createI18n({...i18nConfig, locale: "en"})],
    },
  });
}

const sampleEntries = [
  {
    id: "m1",
    key: "fact-1",
    content: "The quick brown fox jumps over the lazy dog",
    category: "Core",
    timestamp: "2026-01-01T10:00:00Z",
    session_id: "s1",
  },
  {
    id: "m2",
    key: "fact-2",
    content: "Some other content here",
    category: "Conversation",
    timestamp: "2026-01-02T10:00:00Z",
    session_id: null,
  },
];

describe("MemoryList", () => {
  it("renders memory entries with key, category, and content preview", async () => {
    mockMemoryResponse(sampleEntries);

    const wrapper = mountMemoryList();
    await flushPromises();

    expect(wrapper.find("table.memory-table").exists()).toBe(true);
    expect(wrapper.findAll("tbody tr")).toHaveLength(2);
    expect(wrapper.text()).toContain("fact-1");
    expect(wrapper.text()).toContain("Core");
    expect(wrapper.text()).toContain("fact-2");
    expect(wrapper.text()).toContain("Conversation");
  });

  it("shows empty state when no entries", async () => {
    mockMemoryResponse([]);

    const wrapper = mountMemoryList();
    await flushPromises();

    expect(wrapper.find("table").exists()).toBe(false);
    expect(wrapper.text()).toContain("No memory entries found");
  });

  it("shows pagination when total exceeds page size", async () => {
    mockMemoryResponse(
        Array.from({length: 25}, (_, i) => ({
          id: `m${i}`,
          key: `key-${i}`,
          content: "content",
          category: "Core",
          timestamp: "2026-01-01",
          session_id: null,
        })),
        60
    );

    const wrapper = mountMemoryList();
    await flushPromises();

    expect(wrapper.find(".pagination").exists()).toBe(true);
    expect(wrapper.text()).toContain("60");
  });

  it("shows confirmation dialog when delete button is clicked", async () => {
    mockMemoryResponse(sampleEntries);

    const wrapper = mountMemoryList();
    await flushPromises();

    const deleteBtn = wrapper.find('[aria-label="Delete fact-1"]');
    expect(deleteBtn.exists()).toBe(true);

    await deleteBtn.trigger("click");

    expect(wrapper.find(".confirm-dialog").exists()).toBe(true);
    expect(wrapper.find(".confirm-dialog").attributes("aria-modal")).toBe("true");
    expect(wrapper.find(".confirm-dialog").attributes("aria-labelledby")).toBe(
        "memory-delete-title"
    );
    expect(wrapper.find(".confirm-dialog").attributes("aria-describedby")).toBe(
        "memory-delete-description"
    );
    expect(wrapper.text()).toContain("fact-1");
  });

  it("sends DELETE request when deletion is confirmed", async () => {
    mockMemoryResponse(sampleEntries);

    const wrapper = mountMemoryList();
    await flushPromises();

    await wrapper.find('[aria-label="Delete fact-1"]').trigger("click");

    // Mock DELETE response
    fetchMock.mockResolvedValueOnce(new Response(null, {status: 200}));
    // Mock reload response after delete
    const remainingEntry = sampleEntries[1];
    expect(remainingEntry).toBeDefined();
    mockMemoryResponse(remainingEntry ? [remainingEntry] : [], 1);

    await wrapper.find(".confirm-yes").trigger("click");
    await flushPromises();

    const deleteCall = fetchMock.mock.calls.find(
        (entry) => (entry[1]?.method ?? "GET") === "DELETE"
    );
    expect(deleteCall).toBeDefined();
    expect(deleteCall?.[0]).toContain("/web/admin/memory/fact-1");
  });

  it("keeps entry when deletion is cancelled", async () => {
    mockMemoryResponse(sampleEntries);

    const wrapper = mountMemoryList();
    await flushPromises();

    const deleteButton = wrapper.find('[aria-label="Delete fact-1"]');
    const deleteElement = deleteButton.element as HTMLButtonElement;
    deleteElement.focus();

    await deleteButton.trigger("click");
    expect(wrapper.find(".confirm-dialog").exists()).toBe(true);

    await wrapper.find(".confirm-no").trigger("click");
    await flushPromises();

    expect(wrapper.find(".confirm-dialog").exists()).toBe(false);
    expect(document.activeElement).toBe(deleteElement);
    // No DELETE call should have been made (only the initial fetch)
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it.each([
    ["sessionIdFilter", "session-42", "session_id"],
    ["categoryFilter", "Core", "category"],
    ["searchFilter", "vector", "q"],
  ])("refetches when %s changes", async (propName: string, propValue: string, queryParam: string) => {
    mockMemoryResponse(sampleEntries);

    const wrapper = mountMemoryList();
    await flushPromises();

    mockMemoryResponse(sampleEntries);
    await wrapper.setProps({[propName]: propValue});
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(2);

    const [url] = fetchMock.mock.calls[1] ?? [];
    const parsed = new URL(url as string);
    expect(parsed.searchParams.get(queryParam)).toBe(propValue);
  });

  it("emits category and session drill-in events while preserving browse list behavior", async () => {
    mockMemoryResponse(sampleEntries);

    const wrapper = mountMemoryList();
    await flushPromises();

    await wrapper.find("button.category-badge").trigger("click");
    await wrapper.find("button.session-link").trigger("click");
    await wrapper.find("button.explore-btn").trigger("click");

    expect(wrapper.emitted("select-category")).toEqual([["Core"]]);
    expect(wrapper.emitted("select-session")).toEqual([["s1"]]);
    expect(wrapper.emitted("open-explorer")).toEqual([
      [
        {
          category: "Core",
          sessionId: "s1",
          entryId: "m1",
        },
      ],
    ]);
    expect(wrapper.find("table.memory-table").exists()).toBe(true);
  });

  it("renders entries without a session as non-actionable", async () => {
    mockMemoryResponse(sampleEntries);

    const wrapper = mountMemoryList();
    await flushPromises();

    const sessionButtons = wrapper.findAll("button.session-link");
    expect(sessionButtons[1]?.attributes("disabled")).toBeDefined();
    expect(sessionButtons[1]?.attributes("aria-label")).toBe("No session");

    await sessionButtons[1]?.trigger("click");

    expect(wrapper.emitted("select-session")).toBeUndefined();
  });
});
