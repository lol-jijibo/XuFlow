import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

// Git 操作封装层，为审查面板提供统一的 git 调用接口

export function useGitReview() {
  const loading = ref(false);
  const error = ref<string | null>(null);

  /** 获取原始 git diff 输出 */
  async function fetchRawDiff(args?: string): Promise<string> {
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<string>("git_diff_raw", { args: args || null });
      return result;
    } catch (e) {
      error.value = String(e);
      return "";
    } finally {
      loading.value = false;
    }
  }

  /** 获取 git status 输出 */
  async function fetchStatus(): Promise<string> {
    try {
      return await invoke<string>("git_status_raw");
    } catch (e) {
      error.value = String(e);
      return "";
    }
  }

  /** 暂存文件 */
  async function stageFile(path: string): Promise<boolean> {
    try {
      await invoke<string>("git_add", { files: path });
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  /** 取消暂存 */
  async function unstageFile(path: string): Promise<boolean> {
    try {
      await invoke<string>("git_reset_file", { path });
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  /** 回退文件 */
  async function revertFile(path: string): Promise<boolean> {
    try {
      await invoke<string>("git_checkout_file", { path });
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  /** 回退所有 */
  async function revertAll(): Promise<boolean> {
    try {
      await invoke<string>("git_checkout_all");
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  return {
    loading,
    error,
    fetchRawDiff,
    fetchStatus,
    stageFile,
    unstageFile,
    revertFile,
    revertAll,
  };
}
