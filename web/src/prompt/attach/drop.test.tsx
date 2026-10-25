// A page-wide file drop leaves alone what an inner drop zone takes: a drop the
// zone handled, and anything over a portal (the editor's Library modal), whose
// React events still bubble to the page.

import { expect, test } from "bun:test";
import { fireEvent, render, screen } from "@testing-library/react";
import type { DragEvent } from "react";
import { createPortal } from "react-dom";
import { useFileDrop } from "@/prompt/attach/useFileDrop";

function Page({ onFiles }: { onFiles: (files: File[]) => void }) {
  const drop = useFileDrop(onFiles);
  const inner = {
    onDragOver: (event: DragEvent) => event.preventDefault(),
    onDrop: (event: DragEvent) => event.preventDefault(),
  };
  return (
    <div data-testid="page" {...drop.handlers}>
      {drop.dragging && <p>overlay</p>}
      <section data-testid="zone" {...inner} />
      {createPortal(<div data-testid="dialog" />, document.body)}
    </div>
  );
}

const files = [new File(["x"], "trip.mp4")];
const dataTransfer = () => ({ types: ["Files"], files, dropEffect: "none" });

function setup() {
  const taken: File[][] = [];
  render(<Page onFiles={(dropped) => void taken.push(dropped)} />);
  return taken;
}

test("a drop on the page itself is taken", () => {
  const taken = setup();
  fireEvent.dragEnter(screen.getByTestId("page"), { dataTransfer: dataTransfer() });
  expect(screen.getByText("overlay")).toBeTruthy();
  fireEvent.drop(screen.getByTestId("page"), { dataTransfer: dataTransfer() });
  expect(taken).toEqual([files]);
});

test("a drop an inner zone handled is not taken again", () => {
  const taken = setup();
  fireEvent.dragOver(screen.getByTestId("zone"), { dataTransfer: dataTransfer() });
  fireEvent.drop(screen.getByTestId("zone"), { dataTransfer: dataTransfer() });
  expect(taken).toEqual([]);
});

test("a drag over a portal shows no overlay and its drop is not taken", () => {
  const taken = setup();
  const dialog = screen.getByTestId("dialog");
  fireEvent.dragEnter(dialog, { dataTransfer: dataTransfer() });
  expect(screen.queryByText("overlay")).toBeNull();
  expect(fireEvent.dragOver(dialog, { dataTransfer: dataTransfer() })).toBe(true);
  fireEvent.drop(dialog, { dataTransfer: dataTransfer() });
  expect(taken).toEqual([]);
});
