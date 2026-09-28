// The live stream, `GET /api/events`, held once per page however many parts of
// it listen. The server sends everything live for a user down one connection
// (job states, the assistant's words, a project's new revision), so a second
// `EventSource` would only be a second copy of the same messages — and a
// browser caps connections per origin. The stream opens with the first
// listener and closes with the last.
//
// A reconnect — `EventSource` does that by itself after a dropped connection
// or a server restart — may have missed anything, so every listener is handed
// a `resync`, the same instruction the server sends a reader that fell behind:
// re-read what you show.

import { useEffect, useRef } from "react";
import type { ServerEvent } from "@/api/events";

type Listener = (event: ServerEvent) => void;

const listeners = new Set<Listener>();
let source: EventSource | null = null;

/** Hand `event` to every listener; one that throws does not starve the rest. */
function deliver(event: ServerEvent) {
  for (const listener of listeners) {
    try {
      listener(event);
    } catch (error) {
      console.error("an event listener failed", error);
    }
  }
}

function open() {
  const stream = new EventSource("/api/events");
  let opened = false;
  stream.onopen = () => {
    if (opened) deliver({ type: "resync" });
    opened = true;
  };
  stream.onmessage = (message) => {
    let event: ServerEvent;
    try {
      event = JSON.parse(message.data);
    } catch {
      return;
    }
    deliver(event);
  };
  return stream;
}

/**
 * Call `onEvent` with every message on the live stream while the component is
 * mounted. The latest `onEvent` is used, so it need not be memoised.
 */
export function useServerEvents(onEvent: (event: ServerEvent) => void): void {
  const latest = useRef(onEvent);
  latest.current = onEvent;
  useEffect(() => {
    const listener: Listener = (event) => latest.current(event);
    listeners.add(listener);
    if (!source && typeof EventSource !== "undefined") source = open();
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0 && source) {
        source.close();
        source = null;
      }
    };
  }, []);
}
