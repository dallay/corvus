import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { defineComponent, nextTick, reactive, ref } from "vue";
import { createI18n } from "vue-i18n";

import App from "@/App.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";
import type { QuickPairState } from "@/composables/useConfig";

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
  const { defineComponent } = await import("vue");

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

function createSectionModule(name: string) {
  return async () => {
    const { defineComponent } = await import("vue");

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
          return { name };
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

function createMockConfig(overrides: {
  quickPairState?: QuickPairState;
  canSave?: boolean;
  statusMessage?: string;
  errorMessage?: string;
  webhookSecretExists?: boolean;
  sectionSaving?: Partial<Record<string, boolean>>;
} = {}) {
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
      ...overrides.sectionSaving,
    }),
    memoryBackendOptions: ref(["sqlite", "none"]),
    observabilityBackendOptions: ref(["none", "otel"]),
    runtimeKindOptions: ref(["native", "docker"]),
    autonomyLevelOptions: ref(["readonly", "supervised", "full"]),
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
    const { wrapper } = mountApp(
      createMockConfig({
        webhookSecretExists: true,
      })
    );

    expect(wrapper.text()).toContain("Configuración segura del gateway");
    expect(wrapper.text()).toContain("Autenticación");
    expect(wrapper.text()).toContain("Secret actual: configurado");
    expect(wrapper.findAll("input")).toHaveLength(3);
    expect(wrapper.findAll("[data-section]")).toHaveLength(7);
  });

  it("shows quick-pair progress states and hides auth controls while connecting", async () => {
    const { wrapper, config } = mountApp(createMockConfig({ quickPairState: "validating" }));

    expect(wrapper.text()).toContain("Enlace detectado, validando");
    expect(wrapper.findAll("input")).toHaveLength(0);

    config.quickPairState.value = "connecting";
    await nextTick();

    expect(wrapper.text()).toContain("Conectando al gateway");
    expect(wrapper.findAll("input")).toHaveLength(0);
  });

  it("shows quick-pair failures and delegates auth actions", async () => {
    const { wrapper, config } = mountApp(
      createMockConfig({
        quickPairState: "failed",
        errorMessage: "No se pudo conectar",
      })
    );

    expect(wrapper.text()).toContain("Falló el emparejamiento rápido");
    expect(wrapper.text()).toContain("No se pudo conectar");

    const buttons = wrapper.findAll("button");
    await buttons[0]?.trigger("click");
    await buttons[1]?.trigger("click");

    expect(config.pairGateway).toHaveBeenCalledOnce();
    expect(config.connectGateway).toHaveBeenCalledOnce();
  });

  it("propagates child section updates and save events", async () => {
    const { wrapper, config } = mountApp(
      createMockConfig({
        canSave: false,
        sectionSaving: { general: true, webhook: true },
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
});
