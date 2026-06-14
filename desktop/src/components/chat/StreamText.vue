<script setup lang="ts">
import { computed } from "vue";
import MarkdownIt from "markdown-it";
import hljs from "highlight.js";
import { useThemeStore } from "../../stores/theme";

const props = defineProps<{
  text: string;
  done: boolean;
}>();

const themeStore = useThemeStore();

const md: MarkdownIt = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
  highlight(str: string, lang: string): string {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return (
          '<pre class="hljs-code-block"><div class="hljs-header">' +
          `<span class="hljs-lang">${lang}</span>` +
          '<button class="hljs-copy-btn" data-code="' +
          encodeURIComponent(str) +
          '">复制</button></div>' +
          `<code class="hljs">${hljs.highlight(str, { language: lang, ignoreIllegals: true }).value}</code></pre>`
        );
      } catch {
        // fall through
      }
    }
    return (
      '<pre class="hljs-code-block"><div class="hljs-header">' +
      '<span class="hljs-lang">code</span>' +
      '<button class="hljs-copy-btn" data-code="' +
      encodeURIComponent(str) +
      '">复制</button></div>' +
      `<code class="hljs">${md.utils.escapeHtml(str)}</code></pre>`
    );
  },
});

const renderedHtml = computed(() => {
  try {
    return md.render(props.text);
  } catch {
    return md.utils.escapeHtml(props.text);
  }
});
</script>

<template>
  <div
    class="stream-text markdown-body"
    :class="{ dark: themeStore.isDark }"
    v-html="renderedHtml"
  />
</template>

<style scoped>
.markdown-body {
  line-height: 1.7;
  word-break: break-word;
}

/* Code blocks */
.markdown-body :deep(.hljs-code-block) {
  background: #1e1e2e;
  border-radius: 10px;
  margin: 12px 0;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.markdown-body :deep(.hljs-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 14px;
  background: #2d2d44;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.markdown-body :deep(.hljs-lang) {
  font-size: 12px;
  color: #94a3b8;
  font-family: "SF Mono", "Fira Code", monospace;
  font-weight: 500;
}

.markdown-body :deep(.hljs-copy-btn) {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #94a3b8;
  font-size: 11px;
  padding: 3px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.markdown-body :deep(.hljs-copy-btn:hover) {
  background: rgba(255, 255, 255, 0.15);
  color: #e2e8f0;
}

.markdown-body :deep(code.hljs) {
  display: block;
  padding: 14px;
  overflow-x: auto;
  font-size: 13px;
  line-height: 1.6;
  font-family: "SF Mono", "Fira Code", monospace;
}

/* Inline code */
.markdown-body :deep(code:not(.hljs)) {
  background: rgba(99, 102, 241, 0.1);
  padding: 2px 6px;
  border-radius: 5px;
  font-size: 0.9em;
  font-family: "SF Mono", "Fira Code", monospace;
  color: #6366f1;
}

.dark.markdown-body :deep(code:not(.hljs)) {
  background: rgba(99, 102, 241, 0.2);
  color: #818cf8;
}

/* Paragraphs */
.markdown-body :deep(p) {
  margin: 0 0 10px;
}

.markdown-body :deep(p:last-child) {
  margin-bottom: 0;
}

/* Lists */
.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  padding-left: 24px;
  margin: 6px 0;
}

.markdown-body :deep(li) {
  margin: 4px 0;
}

/* Blockquotes */
.markdown-body :deep(blockquote) {
  border-left: 3px solid #6366f1;
  margin: 10px 0;
  padding: 6px 14px;
  color: #64748b;
  background: rgba(99, 102, 241, 0.04);
  border-radius: 0 8px 8px 0;
}

.dark.markdown-body :deep(blockquote) {
  color: #94a3b8;
  background: rgba(99, 102, 241, 0.08);
}

/* Tables */
.markdown-body :deep(table) {
  border-collapse: collapse;
  margin: 10px 0;
  width: 100%;
  border-radius: 8px;
  overflow: hidden;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  border: 1px solid rgba(0, 0, 0, 0.08);
  padding: 8px 12px;
  text-align: left;
}

.markdown-body :deep(th) {
  background: #f8fafc;
  font-weight: 600;
}

.dark.markdown-body :deep(th),
.dark.markdown-body :deep(td) {
  border-color: rgba(255, 255, 255, 0.08);
}

.dark.markdown-body :deep(th) {
  background: #1e1e3f;
}

/* Links */
.markdown-body :deep(a) {
  color: #6366f1;
  text-decoration: none;
  border-bottom: 1px solid transparent;
  transition: border-color 0.2s ease;
}

.markdown-body :deep(a:hover) {
  border-bottom-color: #6366f1;
}

/* Headings */
.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4) {
  margin: 16px 0 8px;
  font-weight: 600;
}

.markdown-body :deep(h1) {
  font-size: 1.5em;
}

.markdown-body :deep(h2) {
  font-size: 1.3em;
}

.markdown-body :deep(h3) {
  font-size: 1.1em;
}

/* Horizontal rule */
.markdown-body :deep(hr) {
  border: none;
  border-top: 1px solid rgba(0, 0, 0, 0.1);
  margin: 16px 0;
}

.dark.markdown-body :deep(hr) {
  border-top-color: rgba(255, 255, 255, 0.1);
}
</style>
