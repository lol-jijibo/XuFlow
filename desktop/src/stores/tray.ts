import { defineStore } from "pinia";
import { ref } from "vue";

export const useTrayStore = defineStore("tray", () => {
  const visible = ref(true);

  // TODO: integrate with Tauri tray API
  function hide() {
    visible.value = false;
  }

  function show() {
    visible.value = true;
  }

  function toggle() {
    visible.value = !visible.value;
  }

  return { visible, hide, show, toggle };
});
