import { translations } from "@corvus/locales";
import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import SessionSidebar from "@/components/SessionSidebar.vue";

function createI18nPlugin() {
  return createI18n({
    legacy: false,
    locale: "en",
    fallbackLocale: "en",
    messages: { en: translations.en },
  });
}

const sampleSessions = [
  {
    id: "sess-1",
    started_at: "2026-03-28T10:00:00Z",
    ended_at: null,
    message_count: 5,
    last_activity: "2026-03-28T11:00:00Z",
  },
  {
    id: "sess-2",
    started_at: "2026-03-27T08:00:00Z",
    ended_at: "2026-03-27T15:00:00Z",
    message_count: 12,
    last_activity: "2026-03-27T14:00:00Z",
  },
  {
    id: "sess-3",
    started_at: "2026-03-26T06:00:00Z",
    ended_at: null,
    message_count: 3,
    last_activity: "2026-03-26T09:00:00Z",
  },
];

function mountSidebar(props?: Record<string, unknown>) {
  return mount(SessionSidebar, {
    props: {
      sessions: sampleSessions,
      currentSessionId: "sess-1",
      collapsed: false,
      ...props,
    },
    global: {
      plugins: [createI18nPlugin()],
    },
  });
}

describe("SessionSidebar", () => {
  it("renders session list with message count", () => {
    const wrapper = mountSidebar();

    const items = wrapper.findAll(".session-sidebar-item");
    expect(items.length).toBe(3);

    // Session IDs are rendered via truncateId
    expect(wrapper.text()).toContain("sess-1");
    expect(wrapper.text()).toContain("sess-2");
    expect(wrapper.text()).toContain("sess-3");

    // Each item renders a meta span with the i18n message count key
    const metas = wrapper.findAll(".session-item-meta");
    expect(metas.length).toBe(3);
  });

  it("highlights current session", () => {
    const wrapper = mountSidebar();

    const activeItem = wrapper.find(".session-sidebar-item--active");
    expect(activeItem.exists()).toBe(true);
    expect(activeItem.text()).toContain("sess-1");
    expect(activeItem.attributes("aria-current")).toBe("true");
  });

  it("emits switch-session event on session click", async () => {
    const wrapper = mountSidebar();

    const nonActiveItem = wrapper.find('[data-testid="session-item-sess-2"]');
    expect(nonActiveItem.exists()).toBe(true);
    await nonActiveItem.trigger("click");

    const emitted = wrapper.emitted("switch-session");
    expect(emitted).toHaveLength(1);
    expect(emitted?.[0]?.[0]).toBe("sess-2");
  });

  it("emits new-chat event when New Chat button is clicked", async () => {
    const wrapper = mountSidebar();

    await wrapper.find(".session-sidebar-new-chat").trigger("click");

    expect(wrapper.emitted("new-chat")).toHaveLength(1);
  });

  it("collapses sidebar and hides session list", () => {
    const wrapper = mountSidebar({ collapsed: true });

    expect(wrapper.find(".session-sidebar--collapsed").exists()).toBe(true);
    expect(wrapper.find(".session-sidebar-list").exists()).toBe(false);
    expect(wrapper.find(".session-sidebar-new-chat").exists()).toBe(false);
  });

  it("emits toggle-collapse when toggle button is clicked", async () => {
    const wrapper = mountSidebar();

    await wrapper.find(".session-sidebar-toggle").trigger("click");

    expect(wrapper.emitted("toggle-collapse")).toHaveLength(1);
  });

  it("shows empty state when no sessions", () => {
    const wrapper = mountSidebar({ sessions: [] });

    expect(wrapper.find(".session-sidebar-empty").exists()).toBe(true);
    expect(wrapper.find(".session-sidebar-item").exists()).toBe(false);
  });

  it("still shows New Chat button when no sessions", () => {
    const wrapper = mountSidebar({ sessions: [] });

    expect(wrapper.find(".session-sidebar-new-chat").exists()).toBe(true);
  });
});
