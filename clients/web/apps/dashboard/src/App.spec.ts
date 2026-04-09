import {mount} from "@vue/test-utils";
import {describe, expect, it, vi} from "vitest";
import {nextTick, reactive, ref} from "vue";
import {createI18n} from "vue-i18n";

import App from "@/App.vue";
import type {
  DashboardOnboardingState,
  DashboardOnboardingStep,
  QuickPairState,
} from "@/composables/useConfig";
import {i18nConfig} from "@/i18n";
import {createAdminConfigForm} from "@/test/adminConfigFormFactory";

const mockedConfigState = vi.hoisted(() => ({
  current: null as ReturnType<typeof createMockConfig> | null,
}));

vi.mock("@/composables/useConfig", () => ({
  useConfig: () => {
    if (!mockedConfigState.current) {
      throw new Error("mock config not initialized");
    }
    return mockedConfigState.current;
  },
}));

vi.mock("@corvus/ui", async () => {
  const {defineComponent} = await import("vue");

  return {
    Button: defineComponent({
      name: "Button",
      props: {
        disabled: {
          type: Boolean,
          default: false,
        },
        variant: {
          type: String,
          default: "default",
        },
      },
      emits: ["click"],
      template:
          '<button :data-variant="variant" :disabled="disabled" @click="$emit(\'click\')"><slot /></button>',
    }),
    Input: defineComponent({
      name: "Input",
      props: {
        modelValue: {
          type: String,
          default: "",
        },
        type: {
          type: String,
          default: "text",
        },
      },
      emits: ["update:modelValue"],
      template:
          '<input :value="modelValue" :type="type" @input="$emit(\'update:modelValue\', $event.target.value)" />',
    }),
  };
});

vi.mock("@/components/memory/MemoryStats.vue", async () => {
  const {defineComponent} = await import("vue");

  return {
    default: defineComponent({
      name: "MemoryStatsStub",
      emits: ["select-category"],
      template:
          '<section data-testid="memory-stats"><button class="stats-select-category" @click="$emit(\'select-category\', \'Core\')">stats-select-category</button></section>',
    }),
  };
});

vi.mock("@/components/memory/MemoryFilters.vue", async () => {
  const {defineComponent} = await import("vue");

  return {
    default: defineComponent({
      name: "MemoryFiltersStub",
      props: {
        initialSessionId: {
          type: String,
          default: undefined,
        },
      },
      emits: ["update:category", "update:session-id", "update:search"],
      template:
          '<section data-testid="memory-filters">filters:{{ initialSessionId ?? "none" }}</section>',
    }),
  };
});

vi.mock("@/components/memory/MemoryList.vue", async () => {
  const {defineComponent} = await import("vue");

  return {
    default: defineComponent({
      name: "MemoryListStub",
      props: {
        categoryFilter: {
          type: String,
          default: undefined,
        },
        sessionIdFilter: {
          type: String,
          default: undefined,
        },
        searchFilter: {
          type: String,
          default: undefined,
        },
      },
      emits: ["select-category", "select-session", "open-explorer"],
      template: `
        <section data-testid="memory-list">
          <p class="memory-list-props">{{ categoryFilter ?? 'none' }}|{{ sessionIdFilter ?? 'none' }}|{{ searchFilter ?? 'none' }}</p>
          <button class="list-select-category" @click="$emit('select-category', 'Core')">list-select-category</button>
          <button class="list-select-session" @click="$emit('select-session', 'session-42')">list-select-session</button>
          <button class="list-open-explorer" @click="$emit('open-explorer', { category: 'Core', sessionId: 'session-42', entryId: 'memory-7' })">list-open-explorer</button>
        </section>
      `,
    }),
  };
});

vi.mock("@/components/memory/LocalMemoryExplorerPanel.vue", async () => {
  const {defineComponent} = await import("vue");

  return {
    default: defineComponent({
      name: "LocalMemoryExplorerPanelStub",
      props: {
        selection: {
          type: Object,
          default: () => ({}),
        },
      },
      emits: ["selection-change", "open-browse"],
      template: `
        <section data-testid="local-memory-explorer">
          <p class="explorer-selection">{{ JSON.stringify(selection) }}</p>
          <button class="explorer-open-browse" @click="$emit('open-browse', selection)">explorer-open-browse</button>
        </section>
      `,
    }),
  };
});

vi.mock("@/components/memory/CerebroSearchPanel.vue", async () => {
  const {defineComponent} = await import("vue");

  return {
    default: defineComponent({
      name: "CerebroSearchPanelStub",
      template: '<section data-testid="cerebro-search">Cerebro Search</section>',
    }),
  };
});

vi.mock("@/components/memory/CerebroObservationDetail.vue", async () => {
  const {defineComponent} = await import("vue");

  return {
    default: defineComponent({
      name: "CerebroObservationDetailStub",
      template: '<section data-testid="cerebro-detail">Cerebro Detail</section>',
    }),
  };
});

vi.mock("@/components/memory/CerebroTimelinePanel.vue", async () => {
  const {defineComponent} = await import("vue");

  return {
    default: defineComponent({
      name: "CerebroTimelinePanelStub",
      template: '<section data-testid="cerebro-timeline">Cerebro Timeline</section>',
    }),
  };
});

function createSectionModule(name: string) {
  return async () => {
    const {defineComponent} = await import("vue");

    return {
      default: defineComponent({
        name: `${name}-stub`,
        props: {
          modelValue: {
            type: Object,
            required: true,
          },
          disabled: {
            type: Boolean,
            default: false,
          },
          saving: {
            type: Boolean,
            default: false,
          },
        },
        emits: ["update:model-value", "save"],
        template: `
          <section :data-section="name">
            <span class="section-name">{{ name }}</span>
            <span class="section-disabled">{{ disabled }}</span>
            <span class="section-saving">{{ saving }}</span>
            <button class="emit-update" @click="$emit('update:model-value', { default_model: name + '-model' })">
              update
            </button>
            <button class="emit-save" @click="$emit('save')">save</button>
          </section>
        `,
        setup() {
          return {name};
        },
      }),
    };
  };
}

vi.mock("@/components/config/GeneralSettings.vue", createSectionModule("general"));
vi.mock("@/components/config/SecuritySettings.vue", createSectionModule("security"));
vi.mock("@/components/config/ObservabilitySettings.vue", createSectionModule("observability"));
vi.mock("@/components/config/RuntimeSettings.vue", createSectionModule("runtime"));
vi.mock("@/components/config/SchedulerSettings.vue", createSectionModule("scheduler"));
vi.mock("@/components/config/GatewaySettings.vue", createSectionModule("gateway"));
vi.mock("@/components/config/WebhookSettings.vue", createSectionModule("webhook"));

function createMockConfig(
    overrides: {
      quickPairState?: QuickPairState;
      canSave?: boolean;
      statusMessage?: string;
      errorMessage?: string;
      webhookSecretExists?: boolean;
      sectionSaving?: Partial<Record<string, boolean>>;
      onboardingState?: DashboardOnboardingState;
      onboardingSteps?: DashboardOnboardingStep[];
      isOperatorReady?: boolean;
    } = {}
) {
  const onboardingState =
      overrides.onboardingState ??
      ({
        surfaceId: "web_dashboard",
        state: "trust_pending",
        trustMode: "http_paired",
        transportMode: "http_gateway",
        recoveryKind: null,
        canRetry: false,
        canResume: false,
        persistsPairingCode: false,
        persistsBearerToken: false,
      } satisfies DashboardOnboardingState);
  const onboardingSteps =
      overrides.onboardingSteps ??
      ([
        {
          key: "runtime",
          titleKey: "onboarding.steps.runtime.title",
          descriptionKey: "onboarding.steps.runtime.description",
          status: "current",
        },
        {
          key: "trust",
          titleKey: "onboarding.steps.trust.title",
          descriptionKey: "onboarding.steps.trust.description",
          status: "pending",
        },
        {
          key: "connect",
          titleKey: "onboarding.steps.connect.title",
          descriptionKey: "onboarding.steps.connect.description",
          status: "pending",
        },
        {
          key: "ready",
          titleKey: "onboarding.steps.ready.title",
          descriptionKey: "onboarding.steps.ready.description",
          status: "pending",
        },
      ] satisfies DashboardOnboardingStep[]);

  return {
    baseUrl: ref("/api"),
    pairingCode: ref(""),
    bearerToken: ref("token"),
    loading: ref(false),
    statusMessage: ref(overrides.statusMessage ?? ""),
    errorMessage: ref(overrides.errorMessage ?? ""),
    quickPairState: ref<QuickPairState>(overrides.quickPairState ?? "idle"),
    form: reactive(
        createAdminConfigForm({
          webhook_secret_exists: overrides.webhookSecretExists ?? false,
        })
    ),
    canSave: ref(overrides.canSave ?? true),
    sectionSaving: reactive({
      general: false,
      security: false,
      observability: false,
      runtime: false,
      scheduler: false,
      gateway: false,
      webhook: false,
      "web-search": false,
      browser: false,
      composio: false,
      memory: false,
      "provider-pools": false,
      updates: false,
      ...overrides.sectionSaving,
    }),
    memoryBackendOptions: ref(["sqlite", "none"]),
    observabilityBackendOptions: ref(["none", "otel"]),
    runtimeKindOptions: ref(["native", "docker"]),
    autonomyLevelOptions: ref(["readonly", "supervised", "full"]),
    onboardingState: ref(onboardingState),
    onboardingSteps: ref(onboardingSteps),
    isOperatorReady: ref(overrides.isOperatorReady ?? false),
    pairGateway: vi.fn(async () => true),
    connectGateway: vi.fn(async () => true),
    saveSection: vi.fn(async () => undefined),
  };
}

function mountApp(config = createMockConfig()) {
  mockedConfigState.current = config;
  const i18n = createI18n(i18nConfig);

  return {
    config,
    wrapper: mount(App, {
      global: {
        plugins: [i18n],
      },
    }),
  };
}

describe("Dashboard App", () => {
  it("renders auth controls, config sections, and webhook helper state", () => {
    const {wrapper} = mountApp(
        createMockConfig({
          webhookSecretExists: true,
        })
    );

    expect(wrapper.text()).toContain("Configuración segura del gateway");
    expect(wrapper.text()).toContain("Onboarding del dashboard");
    expect(wrapper.text()).toContain("Runtime disponible");
    expect(wrapper.text()).toContain("Secret actual: configurado");
    expect(wrapper.findAll("input")).toHaveLength(12);
    expect(wrapper.findAll("[data-section]")).toHaveLength(7);
    expect(wrapper.findAll(".onboarding-step")).toHaveLength(4);
  });

  it("shows quick-pair progress states and hides auth controls while connecting", async () => {
    const {wrapper, config} = mountApp(createMockConfig({quickPairState: "validating"}));

    expect(wrapper.text()).toContain("Enlace detectado, validando");
    expect(wrapper.findAll("input")).toHaveLength(9);

    config.quickPairState.value = "connecting";
    await nextTick();

    expect(wrapper.text()).toContain("Conectando al gateway");
    expect(wrapper.findAll("input")).toHaveLength(9);
  });

  it("shows quick-pair failures and delegates auth actions", async () => {
    const {wrapper, config} = mountApp(
        createMockConfig({
          quickPairState: "failed",
          errorMessage: "No se pudo conectar",
          onboardingState: {
            surfaceId: "web_dashboard",
            state: "blocked",
            trustMode: "http_paired",
            transportMode: "http_gateway",
            recoveryKind: "paired_but_not_connected",
            canRetry: true,
            canResume: true,
            persistsPairingCode: false,
            persistsBearerToken: true,
          },
          onboardingSteps: [
            {
              key: "runtime",
              titleKey: "onboarding.steps.runtime.title",
              descriptionKey: "onboarding.steps.runtime.description",
              status: "complete",
            },
            {
              key: "trust",
              titleKey: "onboarding.steps.trust.title",
              descriptionKey: "onboarding.steps.trust.description",
              status: "complete",
            },
            {
              key: "connect",
              titleKey: "onboarding.steps.connect.title",
              descriptionKey: "onboarding.steps.connect.description",
              status: "blocked",
            },
            {
              key: "ready",
              titleKey: "onboarding.steps.ready.title",
              descriptionKey: "onboarding.steps.ready.description",
              status: "pending",
            },
          ],
        })
    );

    expect(wrapper.text()).toContain("Falló el emparejamiento rápido");
    expect(wrapper.text()).toContain("Emparejado pero sin conexión");
    expect(wrapper.text()).toContain("No se pudo conectar");

    const buttons = wrapper.findAll("button");
    await buttons[0]?.trigger("click");
    await buttons[1]?.trigger("click");

    expect(config.pairGateway).toHaveBeenCalledOnce();
    expect(config.connectGateway).toHaveBeenCalledOnce();
  });

  it("propagates child section updates and save events", async () => {
    const {wrapper, config} = mountApp(
        createMockConfig({
          canSave: false,
          sectionSaving: {general: true, webhook: true},
        })
    );

    const generalSection = wrapper.find('[data-section="general"]');
    const webhookSection = wrapper.find('[data-section="webhook"]');

    expect(generalSection.text()).toContain("true");
    expect(webhookSection.text()).toContain("true");

    await generalSection.find(".emit-update").trigger("click");
    await generalSection.find(".emit-save").trigger("click");
    await webhookSection.find(".emit-save").trigger("click");

    expect(config.form.default_model).toBe("general-model");
    expect(config.saveSection).toHaveBeenNthCalledWith(1, "general");
    expect(config.saveSection).toHaveBeenNthCalledWith(2, "webhook");
  });

  it("renders operator-ready completion copy when the dashboard is ready", () => {
    const {wrapper} = mountApp(
        createMockConfig({
          isOperatorReady: true,
          onboardingState: {
            surfaceId: "web_dashboard",
            state: "ready",
            trustMode: "http_paired",
            transportMode: "http_gateway",
            recoveryKind: null,
            canRetry: false,
            canResume: true,
            persistsPairingCode: false,
            persistsBearerToken: true,
          },
          onboardingSteps: [
            {
              key: "runtime",
              titleKey: "onboarding.steps.runtime.title",
              descriptionKey: "onboarding.steps.runtime.description",
              status: "complete",
            },
            {
              key: "trust",
              titleKey: "onboarding.steps.trust.title",
              descriptionKey: "onboarding.steps.trust.description",
              status: "complete",
            },
            {
              key: "connect",
              titleKey: "onboarding.steps.connect.title",
              descriptionKey: "onboarding.steps.connect.description",
              status: "complete",
            },
            {
              key: "ready",
              titleKey: "onboarding.steps.ready.title",
              descriptionKey: "onboarding.steps.ready.description",
              status: "complete",
            },
          ],
        })
    );

    expect(wrapper.text()).toContain("Listo para operar");
    expect(wrapper.text()).toContain("dashboard completó el emparejamiento");
  });

  it("switches to the local explorer from browse drill-ins and preserves local filters", async () => {
    const {wrapper} = mountApp(createMockConfig({isOperatorReady: true}));

    await wrapper.find('[data-testid="nav-memory"]').trigger("click");
    await wrapper.find(".list-open-explorer").trigger("click");

    expect(wrapper.find("[data-testid='local-memory-explorer']").exists()).toBe(true);
    expect(wrapper.text()).toContain('"sessionId":"session-42"');
    expect(wrapper.text()).toContain('"category":"Core"');

    await wrapper.find(".explorer-open-browse").trigger("click");

    expect(wrapper.find("[data-testid='memory-list']").exists()).toBe(true);
    expect(wrapper.find(".memory-list-props").text()).toContain("Core|session-42|none");
  });

  it("keeps the local explorer visibly separate from Cerebro memory mode", async () => {
    const {wrapper} = mountApp(createMockConfig({isOperatorReady: true}));

    await wrapper.find('[data-testid="nav-memory"]').trigger("click");
    await wrapper.find(".stats-select-category").trigger("click");

    expect(wrapper.find("[data-testid='local-memory-explorer']").exists()).toBe(true);
    expect(wrapper.find("[data-testid='cerebro-search']").exists()).toBe(false);

    await wrapper.find('[data-testid="memory-mode-cerebro"]').trigger("click");

    expect(wrapper.find("[data-testid='cerebro-search']").exists()).toBe(true);
    expect(wrapper.find("[data-testid='local-memory-explorer']").exists()).toBe(false);
  });
});
