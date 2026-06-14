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
          primaryColor: "#6366f1",
          primaryColorHover: "#818cf8",
          primaryColorPressed: "#4f46e5",
          primaryColorSuppl: "rgba(99,102,241,0.15)",
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
        primaryColor: "#4f46e5",
        primaryColorHover: "#6366f1",
        primaryColorPressed: "#4338ca",
        primaryColorSuppl: "rgba(79,70,229,0.08)",
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

  function toggle() {
    mode.value = mode.value === "light" ? "dark" : "light";
    localStorage.setItem("xuflow-theme", mode.value);
  }

  function setMode(m: ThemeMode) {
    mode.value = m;
    localStorage.setItem("xuflow-theme", m);
  }

  return { mode, isDark, theme, themeOverrides, toggle, setMode };
});
