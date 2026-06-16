import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { darkTheme, type GlobalThemeOverrides } from "naive-ui";

export type ThemeMode = "light" | "dark";

export const useThemeStore = defineStore("theme", () => {
  const mode = ref<ThemeMode>(
    (localStorage.getItem("xuflow-theme") as ThemeMode) || "light"
  );

  const isDark = computed(() => mode.value === "dark");

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

  function applyTheme() {
    if (mode.value === "dark") {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  }

  function toggle() {
    mode.value = mode.value === "light" ? "dark" : "light";
    localStorage.setItem("xuflow-theme", mode.value);
    applyTheme();
  }

  function setMode(m: ThemeMode) {
    mode.value = m;
    localStorage.setItem("xuflow-theme", m);
    applyTheme();
  }

  // Apply on init
  applyTheme();

  return { mode, isDark, theme, themeOverrides, toggle, setMode };
});
