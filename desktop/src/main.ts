import { createApp } from "vue";
import { createRouter, createWebHashHistory } from "vue-router";
import { createPinia } from "pinia";
import App from "./App.vue";
import { routes } from "./router";
import "highlight.js/styles/github-dark-dimmed.css";

const app = createApp(App);

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

app.use(router);
app.use(createPinia());

app.config.errorHandler = (err, _instance, info) => {
  console.error("[Vue error]", err, info);
};

app.mount("#app");
console.log("[main] Vue app mounted successfully");
