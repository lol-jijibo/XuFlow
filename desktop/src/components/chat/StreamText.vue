<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from "vue";
import MarkdownIt from "markdown-it";
import hljs from "highlight.js";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useThemeStore } from "../../stores/theme";

const props = defineProps<{
  text: string;
  done: boolean;
}>();

const themeStore = useThemeStore();

/** 容器 DOM 引用，用于代理链接点击事件 */
const containerRef = ref<HTMLElement | null>(null);

/**
 * 拦截渲染内容中的链接点击。
 * http/https 链接通过系统默认浏览器打开，避免在 webview 内部跳转导致无法回退。
 */
function handleLinkClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  const anchor = target.closest("a");
  if (!anchor) return;

  const href = anchor.getAttribute("href");
  if (!href) return;

  // 仅拦截外部 http/https 链接，锚点跳转等保持 webview 内默认行为
  if (href.startsWith("http://") || href.startsWith("https://")) {
    event.preventDefault();
    openUrl(href).catch((err) => console.warn("[StreamText] 打开外部链接失败:", err));
  }
}

onMounted(() => {
  containerRef.value?.addEventListener("click", handleLinkClick);
});

onBeforeUnmount(() => {
  containerRef.value?.removeEventListener("click", handleLinkClick);
});

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
      '<span class="hljs-lang">shell</span>' +
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
    ref="containerRef"
    class="stream-text markdown-body"
    :class="{ dark: themeStore.isDark }"
    v-html="renderedHtml"
  />
</template>

<style scoped>
/* ── Base ── */
.markdown-body {
  line-height: 1.75;
  word-break: break-word;
  color: #d1d5db;
  font-size: 15px;
}

.markdown-body:not(.dark) {
  color: #374151;
}

/* ── Headings — bold, hierarchical ── */
.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4) {
  margin: 24px 0 10px;
  font-weight: 700;
  line-height: 1.3;
  color: #f1f5f9;
}

.markdown-body:not(.dark) :deep(h1),
.markdown-body:not(.dark) :deep(h2),
.markdown-body:not(.dark) :deep(h3),
.markdown-body:not(.dark) :deep(h4) {
  color: #111827;
}

.markdown-body :deep(h1) { font-size: 1.35em; }
.markdown-body :deep(h2) { font-size: 1.2em; }
.markdown-body :deep(h3) { font-size: 1.1em; }
.markdown-body :deep(h4) { font-size: 1em; }

/* First heading has no top margin */
.markdown-body :deep(h1:first-child),
.markdown-body :deep(h2:first-child),
.markdown-body :deep(h3:first-child) {
  margin-top: 0;
}

/* ── Paragraphs ── */
.markdown-body :deep(p) {
  margin: 0 0 12px;
  color: #d1d5db;
  line-height: 1.75;
}

.markdown-body:not(.dark) :deep(p) {
  color: #374151;
}

.markdown-body :deep(p:last-child) {
  margin-bottom: 0;
}

/* ── Lists — bullet points ── */
.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  padding-left: 22px;
  margin: 8px 0 14px;
}

.markdown-body :deep(li) {
  margin: 6px 0;
  color: #d1d5db;
  line-height: 1.7;
}

.markdown-body:not(.dark) :deep(li) {
  color: #4b5563;
}

.markdown-body :deep(li::marker) {
  color: #6b7280;
}

/* ── Code blocks — terminal aesthetic ── */
.markdown-body :deep(.hljs-code-block) {
  background: #1E1E1E;
  border-radius: 8px;
  margin: 14px 0;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.markdown-body:not(.dark) :deep(.hljs-code-block) {
  background: #1a1a1a;
  border-color: rgba(0, 0, 0, 0.1);
}

.markdown-body :deep(.hljs-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 7px 14px;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.markdown-body :deep(.hljs-lang) {
  font-size: 11px;
  color: #6b7280;
  font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.markdown-body :deep(.hljs-copy-btn) {
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: #888;
  font-size: 11px;
  padding: 3px 10px;
  border-radius: 5px;
  cursor: pointer;
  transition: all 0.15s ease;
  font-family: -apple-system, BlinkMacSystemFont, sans-serif;
}

.markdown-body :deep(.hljs-copy-btn:hover) {
  background: rgba(255, 255, 255, 0.12);
  color: #ccc;
}

.markdown-body :deep(code.hljs) {
  display: block;
  padding: 14px 16px;
  overflow-x: auto;
  font-size: 13px;
  line-height: 1.65;
  font-family: "SF Mono", "Fira Code", "Cascadia Code", "JetBrains Mono", monospace;
  color: #60a5fa;  /* blue-400 for terminal feel */
  background: transparent;
}

/* Keyword highlights inside hljs */
.markdown-body :deep(.hljs-keyword) { color: #c084fc; }
.markdown-body :deep(.hljs-string)  { color: #34d399; }
.markdown-body :deep(.hljs-number)  { color: #fbbf24; }
.markdown-body :deep(.hljs-comment) { color: #6b7280; font-style: italic; }
.markdown-body :deep(.hljs-built_in){ color: #f9a8d4; }
.markdown-body :deep(.hljs-title)   { color: #93c5fd; }
.markdown-body :deep(.hljs-type)    { color: #fbbf24; }

/* ── Inline code ── */
.markdown-body :deep(code:not(.hljs)) {
  background: rgba(255, 255, 255, 0.08);
  padding: 2px 7px;
  border-radius: 5px;
  font-size: 0.88em;
  font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
  color: #93c5fd;
}

.markdown-body:not(.dark) :deep(code:not(.hljs)) {
  background: rgba(0, 0, 0, 0.06);
  color: #1e40af;
}

/* ── Blockquotes ── */
.markdown-body :deep(blockquote) {
  border-left: 3px solid #4b5563;
  margin: 12px 0;
  padding: 8px 16px;
  color: #9ca3af;
  background: rgba(255, 255, 255, 0.02);
  border-radius: 0 6px 6px 0;
  font-size: 14px;
}

.markdown-body:not(.dark) :deep(blockquote) {
  color: #6b7280;
  background: rgba(0, 0, 0, 0.02);
  border-left-color: #d1d5db;
}

/* ── Links ── */
.markdown-body :deep(a) {
  color: #60a5fa;
  text-decoration: none;
  border-bottom: 1px solid transparent;
  transition: border-color 0.15s ease;
  cursor: pointer;
}

.markdown-body :deep(a:hover) {
  border-bottom-color: #60a5fa;
}


/* ── Horizontal rule ── */
.markdown-body :deep(hr) {
  border: none;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  margin: 20px 0;
}

.markdown-body:not(.dark) :deep(hr) {
  border-top-color: rgba(0, 0, 0, 0.08);
}

/* ── Tables ── */
.markdown-body :deep(table) {
  border-collapse: collapse;
  margin: 12px 0;
  width: 100%;
  border-radius: 8px;
  overflow: hidden;
  font-size: 14px;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  border: 1px solid rgba(255, 255, 255, 0.06);
  padding: 8px 14px;
  text-align: left;
}

.markdown-body:not(.dark) :deep(th),
.markdown-body:not(.dark) :deep(td) {
  border-color: rgba(0, 0, 0, 0.08);
}

.markdown-body :deep(th) {
  background: rgba(255, 255, 255, 0.04);
  font-weight: 600;
  color: #e5e7eb;
}

.markdown-body:not(.dark) :deep(th) {
  background: #f3f4f6;
  color: #111827;
}

/* ── Strong & emphasis ── */
.markdown-body :deep(strong) {
  font-weight: 700;
  color: #f1f5f9;
}

.markdown-body:not(.dark) :deep(strong) {
  color: #111827;
}

.markdown-body :deep(em) {
  font-style: italic;
  color: #9ca3af;
}

/* ── Images ── */
.markdown-body :deep(img) {
  max-width: 100%;
  border-radius: 8px;
}
</style>
