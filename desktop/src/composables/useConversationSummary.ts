import { invoke } from "@tauri-apps/api/core";
import { useProjectStore } from "../stores/project";

/** Guard against concurrent summarization calls. */
let summarizing = false;

/** Minimum character count to trigger LLM summarization for a single user message. */
const SHORT_PROMPT_MAX = 100;

/** Max characters from a user prompt to use directly as title. */
const DIRECT_TITLE_MAX = 60;

/**
 * Check whether the active conversation needs a title update and perform it.
 * Called after every complete agent response (agent:done event).
 *
 * Logic:
 *  - 1 short user message (<100 chars): use the prompt directly as title.
 *  - 1 long user message (>=100 chars): call LLM to generate a summary.
 *  - 2+ user messages: call LLM to generate a summary covering all exchanges.
 *
 * Skips if there's no API key configured, or if the title was manually set.
 */
export async function trySummarizeConversation(): Promise<void> {
  if (summarizing) return;

  const projectStore = useProjectStore();
  const project = projectStore.activeProject;
  const conv = projectStore.activeConversation;
  if (!project || !conv) return;

  // Never overwrite a manually-set title
  if (conv.titleSource === "manual") return;

  // Gather user messages
  const userMsgs = conv.messages.filter((m) => m.role === "user");
  if (userMsgs.length === 0) return;

  const totalUserMsgs = userMsgs.length;

  if (totalUserMsgs === 1) {
    const firstPrompt = userMsgs[0].content.trim();
    if (firstPrompt.length < SHORT_PROMPT_MAX) {
      // Short prompt — use directly as title
      const title =
        firstPrompt.length > DIRECT_TITLE_MAX
          ? firstPrompt.slice(0, DIRECT_TITLE_MAX - 1) + "…"
          : firstPrompt;
      projectStore.updateConversationTitle(project.id, conv.id, title, "auto");
      return;
    }
    // Long single prompt — fall through to LLM summarization
  }

  // Need LLM summarization: single long prompt or multi-turn
  summarizing = true;
  try {
    // Build a compact message list for the summarization prompt
    // We send role+content for all messages (truncated on the Rust side)
    const compactMessages = conv.messages
      .filter((m) => m.role === "user" || m.role === "assistant")
      .map((m) => ({ role: m.role, content: m.content }));

    const title = await invoke<string>("generate_title", {
      messagesJson: JSON.stringify(compactMessages),
    });

    // Clean up the result — remove stray quotes, newlines, etc.
    const cleaned = title
      .replace(/^["'「『\s]+/, "")
      .replace(/["'」』\s]+$/, "")
      .replace(/[\n\r]+/g, " ")
      .trim()
      .slice(0, 60);

    if (cleaned) {
      projectStore.updateConversationTitle(project.id, conv.id, cleaned, "auto");
    }
  } catch (e) {
    console.warn("[summary] Failed to generate title:", e);
    // Silent failure — the default title stays
  } finally {
    summarizing = false;
  }
}
