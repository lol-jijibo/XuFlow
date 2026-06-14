<script setup lang="ts">
// Placeholder for tool call rendering
defineProps<{
  name: string;
  arguments: string;
  result?: string;
}>();
</script>

<template>
  <div class="tool-call-card">
    <div class="tool-header">Tool: {{ name }}</div>
    <div class="tool-body">
      <pre class="tool-args">{{ arguments }}</pre>
      <pre v-if="result" class="tool-result">{{ result }}</pre>
    </div>
  </div>
</template>

<style scoped>
.tool-call-card {
  margin: 8px 0;
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
  padding: 12px;
  background: var(--n-card-color);
}
.tool-header {
  font-weight: 600;
  font-size: 13px;
  color: var(--n-primary-color);
  margin-bottom: 8px;
}
.tool-args, .tool-result {
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}
</style>
