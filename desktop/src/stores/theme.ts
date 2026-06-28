import { defineStore } from "pinia";
import { ref, computed, watch } from "vue";
import { darkTheme, type GlobalThemeOverrides } from "naive-ui";

// ── Theme variant ──
export type ThemeVariant = "sunset" | "dawn" | "system";
export type ThemeMode = "light" | "dark";

// ── Appearance settings persisted in localStorage ──
const APPEARANCE_KEY = "xuflow-appearance";

function loadAppearance(): {
  variant: ThemeVariant;
  uiFontSize: number;
  codeFontSize: number;
  contrast: number;
} {
  try {
    const raw = localStorage.getItem(APPEARANCE_KEY);
    if (raw) {
      const data = JSON.parse(raw);
      return {
        variant: data.variant ?? "system",
        uiFontSize: data.uiFontSize ?? 14,
        codeFontSize: data.codeFontSize ?? 13,
        contrast: data.contrast ?? 100,
      };
    }
  } catch {
    // ignore
  }
  return { variant: "system", uiFontSize: 14, codeFontSize: 13, contrast: 100 };
}

function saveAppearance(s: {
  variant: ThemeVariant;
  uiFontSize: number;
  codeFontSize: number;
  contrast: number;
}) {
  try {
    localStorage.setItem(APPEARANCE_KEY, JSON.stringify(s));
  } catch {
    // ignore
  }
}

// ── Resolve system preference ──
function systemPrefersDark(): boolean {
  if (typeof window === "undefined") return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

// ── Store ──
export const useThemeStore = defineStore("theme", () => {
  // Load persisted
  const saved = loadAppearance();

  // ── Reactive state ──
  const variant = ref<ThemeVariant>(saved.variant);
  const uiFontSize = ref<number>(saved.uiFontSize);
  const codeFontSize = ref<number>(saved.codeFontSize);
  const contrast = ref<number>(saved.contrast);

  // Resolved effective light/dark mode
  const mode = ref<ThemeMode>(
    variant.value === "system"
      ? systemPrefersDark()
        ? "dark"
        : "light"
      : variant.value === "sunset"
        ? "dark"
        : "light"
  );

  const isDark = computed(() => mode.value === "dark");

  // Listen for system preference changes when variant is "system"
  if (typeof window !== "undefined") {
    window
      .matchMedia("(prefers-color-scheme: dark)")
      .addEventListener("change", (e) => {
        if (variant.value === "system") {
          mode.value = e.matches ? "dark" : "light";
          applyTheme();
        }
      });
  }

  // ── Persist helpers ──
  function persist() {
    saveAppearance({
      variant: variant.value,
      uiFontSize: uiFontSize.value,
      codeFontSize: codeFontSize.value,
      contrast: contrast.value,
    });
  }

  // Auto-persist on any change
  watch([variant, uiFontSize, codeFontSize, contrast], () => persist(), {
    deep: false,
  });

  // ── Contrast-aware theme overrides ──
  const theme = computed(() => (isDark.value ? darkTheme : null));

  const themeOverrides = computed<GlobalThemeOverrides>(() => {
    if (isDark.value) {
      return {
        common: {
          primaryColor: "#6b7280",
          primaryColorHover: "#9ca3af",
          primaryColorPressed: "#4b5563",
          primaryColorSuppl: "rgba(107,114,128,0.15)",
          borderRadius: "8px",
          borderRadiusSmall: "6px",
          fontFamily:
            '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif',
        },
        Card: {
          borderRadius: "12px",
        },
        Input: {
          borderRadius: "10px",
        },
        Button: {
          borderRadius: "8px",
        },
      };
    }
    return {
      common: {
        primaryColor: "#4b5563",
        primaryColorHover: "#6b7280",
        primaryColorPressed: "#374151",
        primaryColorSuppl: "rgba(75,85,99,0.08)",
        borderRadius: "8px",
        borderRadiusSmall: "6px",
        fontFamily:
          '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif',
      },
      Card: {
        borderRadius: "12px",
      },
      Input: {
        borderRadius: "10px",
      },
      Button: {
        borderRadius: "8px",
      },
    };
  });

  // ── Contrast CSS variable ──
  function applyContrast() {
    const root = document.documentElement;
    const app = document.getElementById("app");
    const ratio = contrast.value / 100;
    root.style.setProperty("--xuflow-contrast", String(ratio));
    root.style.setProperty("--xuflow-ui-font-size", `${uiFontSize.value}px`);
    root.style.setProperty("--xuflow-code-font-size", `${codeFontSize.value}px`);

    // 仅在对比度偏离默认值（100）时才启用 filter，
    // 避免 contrast(1) 产生无意义的 GPU 合成层导致文字模糊
    if (app) {
      if (contrast.value !== 100) {
        app.style.filter = `contrast(${ratio})`;
      } else {
        app.style.filter = "";
      }
    }
  }

  // ── Apply theme to document ──
  function applyTheme() {
    if (isDark.value) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
    applyContrast();
  }

  // ── Actions ──
  function toggle() {
    variant.value = isDark.value ? "dawn" : "sunset";
    mode.value = isDark.value ? "light" : "dark";
    persist();
    applyTheme();
  }

  function setMode(m: ThemeMode) {
    mode.value = m;
    if (m === "dark") {
      variant.value = "sunset";
    } else {
      variant.value = "dawn";
    }
    persist();
    applyTheme();
  }

  function setVariant(v: ThemeVariant) {
    variant.value = v;
    if (v === "system") {
      mode.value = systemPrefersDark() ? "dark" : "light";
    } else if (v === "sunset") {
      mode.value = "dark";
    } else {
      mode.value = "light";
    }
    persist();
    applyTheme();
  }

  function setUiFontSize(size: number) {
    uiFontSize.value = Math.max(12, Math.min(20, size));
    persist();
    applyContrast();
  }

  function setCodeFontSize(size: number) {
    codeFontSize.value = Math.max(11, Math.min(18, size));
    persist();
    applyContrast();
  }

  function setContrast(val: number) {
    contrast.value = Math.max(80, Math.min(150, val));
    persist();
    applyContrast();
  }

  // ── Apply on init ──
  applyTheme();

  return {
    // state
    variant,
    mode,
    isDark,
    uiFontSize,
    codeFontSize,
    contrast,
    // computed
    theme,
    themeOverrides,
    // actions
    toggle,
    setMode,
    setVariant,
    setUiFontSize,
    setCodeFontSize,
    setContrast,
    applyContrast,
  };
});
