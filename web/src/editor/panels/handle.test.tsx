// The panel handles, driven the way a person drives them (#898): a drag
// resizes inside the panel's range, a double-click puts it back, the arrow
// keys nudge it, and the size outlives the editor being closed and reopened.
// Mounted in happy-dom (src/test/dom.ts) with the same hook the editor uses.

import { describe, expect, test } from "bun:test";
import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Handle } from "./Handle";
import { DEFAULTS, LEAST, MOST, SIZES_KEY } from "./sizes";
import { usePanels } from "./usePanels";

/** The editor's grid in miniature: the chat's edge, the assets panel's, and the timeline's (#943). */
function Panels() {
  const panels = usePanels();
  return (
    <div ref={panels.grid}>
      <Handle
        grows="right"
        size={panels.sizes.chat}
        label="chat"
        onSize={(size) => panels.resize("chat", size)}
        onReset={() => panels.reset("chat")}
      />
      <Handle
        grows="left"
        size={panels.sizes.assets}
        label="assets"
        onSize={(size) => panels.resize("assets", size)}
        onReset={() => panels.reset("assets")}
      />
      <Handle
        grows="up"
        size={panels.sizes.timeline}
        label="timeline"
        onSize={(size) => panels.resize("timeline", size)}
        onReset={() => panels.reset("timeline")}
      />
    </div>
  );
}

const size = (label: string) =>
  Number(screen.getByRole("separator", { name: label }).getAttribute("aria-valuenow"));

/** A press on `label`'s handle at 100px, moved by `dx`/`dy` pixels, let go. */
function drag(label: string, dx: number, dy = 0) {
  const handle = screen.getByRole("separator", { name: label });
  fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientX: 100, clientY: 100 });
  fireEvent.pointerMove(handle, { pointerId: 1, clientX: 100 + dx, clientY: 100 + dy });
  fireEvent.pointerUp(handle, { pointerId: 1, clientX: 100 + dx, clientY: 100 + dy });
}

describe("a panel handle", () => {
  test("dragged toward the preview, resizes the panel by the distance, in rem", () => {
    render(<Panels />);
    expect(size("chat")).toBe(DEFAULTS.chat);
    drag("chat", 32);
    expect(size("chat")).toBe(DEFAULTS.chat + 2);
    expect(size("assets")).toBe(DEFAULTS.assets);
    drag("assets", -32);
    expect(size("assets")).toBe(DEFAULTS.assets + 2);
  });

  test("the timeline's edge grows as it is dragged up", () => {
    render(<Panels />);
    drag("timeline", 0, -16);
    expect(size("timeline")).toBe(DEFAULTS.timeline + 1);
  });

  test("dragged past either end, stops at the panel's least and most", () => {
    render(<Panels />);
    drag("assets", -10_000);
    expect(size("assets")).toBe(DEFAULTS.assets * MOST);
    drag("assets", 10_000);
    expect(size("assets")).toBe(DEFAULTS.assets * LEAST);
  });

  test("double-clicked, goes back to its default", async () => {
    render(<Panels />);
    drag("assets", 64);
    expect(size("assets")).toBe(DEFAULTS.assets - 4);
    await userEvent.dblClick(screen.getByRole("separator", { name: "assets" }));
    expect(size("assets")).toBe(DEFAULTS.assets);
  });

  test("the arrow keys nudge it the way they point", async () => {
    const user = userEvent.setup();
    render(<Panels />);
    await user.tab();
    expect(window.document.activeElement).toBe(screen.getByRole("separator", { name: "chat" }));
    await user.keyboard("{ArrowRight}{ArrowRight}{ArrowLeft}");
    expect(size("chat")).toBe(DEFAULTS.chat + 1);
    await user.tab();
    await user.keyboard("{ArrowLeft}{ArrowLeft}{ArrowRight}");
    expect(size("assets")).toBe(DEFAULTS.assets + 1);
    await user.tab();
    await user.keyboard("{ArrowUp}");
    expect(size("timeline")).toBe(DEFAULTS.timeline + 1);
  });

  test("a size is kept in this browser and read back when the editor reopens", () => {
    const first = render(<Panels />);
    drag("assets", -48);
    expect(JSON.parse(window.localStorage.getItem(SIZES_KEY) ?? "{}").assets).toBe(
      DEFAULTS.assets + 3,
    );
    first.unmount();
    render(<Panels />);
    expect(size("assets")).toBe(DEFAULTS.assets + 3);
  });
});
