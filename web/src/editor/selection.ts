// Which clips are selected. A click picks one clip; a click with Shift (or
// Ctrl, or ⌘) adds it to the selection or takes it out — how a person gathers
// the clips an intro is made of to save them as a template (#546), and how
// Delete removes several at once. The inspector shows a clip only when one is
// selected: a value typed there is one clip's.

/** The selection after clicking `clip`, `adding` when a modifier was held. */
export function choose(current: string[], clip: string, adding: boolean): string[] {
  if (!adding) return [clip];
  return current.includes(clip) ? current.filter((id) => id !== clip) : [...current, clip];
}

/** Whether a pointer or key event asks to add to the selection. */
export function adds(event: { shiftKey: boolean; ctrlKey: boolean; metaKey: boolean }): boolean {
  return event.shiftKey || event.ctrlKey || event.metaKey;
}

/** The selection with the clips a project no longer has let go of. */
export function kept(current: string[], present: Set<string>): string[] {
  const still = current.filter((id) => present.has(id));
  return still.length === current.length ? current : still;
}
