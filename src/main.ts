import { createApp } from "vue";
import App from "./App.vue";
import "./assets/style.css";
import "@fortawesome/fontawesome-free/css/all.min.css";
import { initPlatform } from "./utils/platform";

// Initialize platform detection before mounting app
initPlatform();

createApp(App).mount("#app");
