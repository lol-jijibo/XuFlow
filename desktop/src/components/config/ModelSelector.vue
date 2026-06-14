<script setup lang="ts">
import { NSelect } from "naive-ui";
import { useConfigStore } from "../../stores/config";

const store = useConfigStore();
</script>

<template>
  <div class="model-selector">
    <NSelect
      v-model:value="store.activeModelId"
      :options="store.modelOptions"
      placeholder="选择模型"
      size="small"
      style="width: 180px"
      class="model-select"
    />
  </div>
</template>

<style scoped>
.model-selector {
  display: flex;
  align-items: center;
  gap: 8px;
}

.model-select {
  --n-border-radius: 8px;
}
</style>
