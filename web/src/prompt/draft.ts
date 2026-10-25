// A first message that was not sent, left for the new project's chat box
// (#1535): the prompt, its files, and why — the server's refusal (no credit
// at all), or files that did not come through the sign-in. The start path
// leaves it and opens the editor; the editor writes it in the box, puts the
// files back in its tray and shows why, then forgets it. It lives in this page
// only: a reload of the editor shows an empty box, never a resend.

export interface Draft {
  prompt: string;
  files: File[];
  /** Why it was not sent: what sending threw. */
  refusal?: unknown;
  /** The names of files to attach again. */
  missing?: string[];
}

const drafts = new Map<number, Draft>();

/** Leave `draft` for project `id`'s chat box. */
export function leaveDraft(id: number, draft: Draft): void {
  drafts.set(id, draft);
}

/** Project `id`'s draft, if one was left; reading it twice gives the same one. */
export function draftFor(id: number): Draft | undefined {
  return drafts.get(id);
}

/** Forget project `id`'s draft: the editor has it. */
export function forgetDraft(id: number): void {
  drafts.delete(id);
}
