import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

import CerebroObservationDetail from "@/components/memory/CerebroObservationDetail.vue";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

describe("CerebroObservationDetail", () => {
  it("renders relationship and ontology insight payloads", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          state: "available",
          observation: {
            memory_id: "mem-42",
            relationships: [{ type: "linked_to", target: "mem-77" }],
            ontology: { topic: "preferences" },
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } }
      )
    );

    const wrapper = mount(CerebroObservationDetail, {
      props: {
        gatewayUrl: (path: string) => new URL(`http://localhost:3000/api${path}`).toString(),
        authHeaders: () => ({ Authorization: "Bearer token" }),
        selected: { memory_id: "mem-42", summary: "dark mode" },
      },
    });

    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain("Relationship Insights");
    expect(wrapper.text()).toContain("linked_to");
    expect(wrapper.text()).toContain("preferences");
  });
});
