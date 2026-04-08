import { createApp } from "vue";

import App from "./App.vue";
import { i18n } from "./i18n";
import "@fontsource-variable/space-grotesk";
import "@fontsource/space-mono/400.css";
import "@fontsource/space-mono/700.css";
import "@fontsource/doto/400.css";
import "@fontsource/doto/700.css";
import "./style.css";

createApp(App).use(i18n).mount("#app");
