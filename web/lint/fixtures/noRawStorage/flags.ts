// Every way to reach browser storage without src/lib/storage.ts.
export const bare = localStorage.getItem("key"); // flagged
export const session = sessionStorage.getItem("key"); // flagged
export const dotted = window.localStorage.getItem("key"); // flagged
export const global = globalThis.sessionStorage; // flagged
export const bracketed = window["localStorage"]; // flagged
export const { localStorage: renamed } = window; // flagged
export const { sessionStorage } = window; // flagged
