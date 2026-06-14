import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { SelectOption } from "naive-ui";

export const useConfigStore = defineStore("config", () => {
  const activeModelId = ref("deepseek-chat");
  const apiKey = ref("");

  const modelOptions = computed<SelectOption[]>(() => [
    { label: "DeepSeek Chat", value: "deepseek-chat" },
    { label: "DeepSeek Reasoner", value: "deepseek-reasoner" },
    { label: "火山方舟 DeepSeek V3", value: "volc-ds-v3" },
    { label: "火山方舟 DeepSeek R1", value: "volc-ds-r1" },
  ]);

  function setApiKey(key: string) {
    apiKey.value = key;
  }

  return { activeModelId, apiKey, modelOptions, setApiKey };
});
