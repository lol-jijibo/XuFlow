<script setup lang="ts">
import type { DiffFile } from "../../utils/diffParser";
import ReviewDiffHunk from "./ReviewDiffHunk.vue";

// 单文件 diff 展开视图，渲染该文件的所有 hunk

defineProps<{
  file: DiffFile;
}>();
</script>

<template>
  <div class="diff-item">
    <div class="diff-item-body">
      <div
        v-for="(hunk, hIdx) in file.hunks"
        :key="hIdx"
        class="diff-hunk-wrapper"
      >
        <ReviewDiffHunk :hunk="hunk" :language="file.language" :filePath="file.path" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.diff-item {
  /* 文件 diff 容器 */
}

.diff-item-body {
  padding: 0;
}

.diff-hunk-wrapper {
  /* 每个 hunk 之间有空隙 */
}
</style>
