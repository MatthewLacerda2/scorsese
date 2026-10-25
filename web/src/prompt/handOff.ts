// A prompt written signed out, kept across the Google sign-in (#1535). The
// landing's box keeps the prompt and its files in the browser, never in the
// URL (the `next` it goes through is sent to Google), and sends the browser to
// the sign-in with `next` = `HAND_OFF`. That page takes what was kept —
// deleting it before acting on it, so a reload makes no second project — and
// starts the project as the signed-in home does.
//
// The prompt and the files' names go in `localStorage` (`lib/storage.ts`);
// the files, as Blobs, in IndexedDB, up to `MAX_KEPT_BYTES` together. Files
// over that, or a browser that refuses to keep them (a private window), come
// back as `missing` names: the person arrives with the prompt written and is
// asked to attach those again, so nothing is lost silently.

import { browserStorage, type Codec, KEYS, read, write } from "@/lib/storage";

/** Where the sign-in lands to carry on: the hand-off page. */
export const HAND_OFF = "/start";

/** The most the files kept may weigh together: 1 GB. */
export const MAX_KEPT_BYTES = 1024 ** 3;

/** What was kept: the prompt, the files that came through, the names of those that did not. */
export interface Kept {
  prompt: string;
  files: File[];
  missing: string[];
}

/** Where the files are kept; tests hand in their own. */
export interface FileBox {
  put: (files: File[]) => Promise<void>;
  /** The files kept, deleted as they are read. */
  take: () => Promise<File[] | undefined>;
  clear: () => Promise<void>;
}

interface Written {
  prompt: string;
  names: string[];
}

const written: Codec<Written | null> = {
  fallback: null,
  parse: (raw) => {
    const value = JSON.parse(raw) as Partial<Written>;
    return typeof value.prompt === "string" && Array.isArray(value.names)
      ? { prompt: value.prompt, names: value.names.map(String) }
      : undefined;
  },
  format: (value) => (value === null ? null : JSON.stringify(value)),
};

const STORE = "files";

/** Run `act` in one transaction on the hand-off database; its request's result. */
async function inStore<T>(act: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const db = await new Promise<IDBDatabase>((resolve, reject) => {
    const opening = indexedDB.open(KEYS.handOff, 1);
    opening.onupgradeneeded = () => opening.result.createObjectStore(STORE);
    opening.onsuccess = () => resolve(opening.result);
    opening.onerror = () => reject(opening.error);
  });
  try {
    return await new Promise<T>((resolve, reject) => {
      const transaction = db.transaction(STORE, "readwrite");
      const request = act(transaction.objectStore(STORE));
      transaction.oncomplete = () => resolve(request.result);
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error);
    });
  } finally {
    db.close();
  }
}

/** The browser's IndexedDB. */
const indexedBox: FileBox = {
  put: async (files) => void (await inStore((store) => store.put(files, STORE))),
  take: () =>
    inStore((store) => {
      const kept = store.get(STORE) as IDBRequest<File[] | undefined>;
      store.delete(STORE);
      return kept;
    }),
  clear: async () => void (await inStore((store) => store.delete(STORE))),
};

/** Keep `prompt` and `files` for the page the sign-in lands on. Never throws. */
export async function keep(
  prompt: string,
  files: readonly File[],
  box: FileBox = indexedBox,
  storage: Storage | undefined = browserStorage(),
): Promise<void> {
  write(storage, KEYS.handOff, written, { prompt, names: files.map((file) => file.name) });
  const bytes = files.reduce((sum, file) => sum + file.size, 0);
  try {
    if (files.length === 0 || bytes > MAX_KEPT_BYTES) await box.clear();
    else await box.put([...files]);
  } catch {
    // Not kept: the hand-off finds their names without them, and asks for them again.
  }
}

/** What was kept, deleted before it is returned; `null` when nothing was. Never throws. */
export async function take(
  box: FileBox = indexedBox,
  storage: Storage | undefined = browserStorage(),
): Promise<Kept | null> {
  const kept = read(storage, KEYS.handOff, written);
  write(storage, KEYS.handOff, written, null);
  const files = (await box.take().catch(() => undefined)) ?? [];
  if (kept === null) return null;
  const came = new Set(files.map((file) => file.name));
  return {
    prompt: kept.prompt,
    files,
    missing: kept.names.filter((name) => !came.has(name)),
  };
}
