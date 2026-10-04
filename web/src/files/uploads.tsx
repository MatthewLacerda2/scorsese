// Uploading files into the library, and the list of what is on its way.
//
// Each file goes through three steps: hash it in the browser, ask the server
// whether that hash is already in the library (a duplicate never crosses the
// network), then hand it to Uppy, which sends it over tus in chunks. The
// server refuses a duplicate at upload too, as a backstop; both end the same
// way — "you already have this as X", linking to X.
//
// The uploader lives in a provider around the whole signed-in app rather than
// on the library page, so an upload keeps going while the user browses.

import { useQueryClient } from "@tanstack/react-query";
import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { api } from "@/api";
import { useT } from "@/i18n/I18nProvider";
import { alreadyHave, duplicateOf } from "@/lib/upload/duplicate";
import { hashFile } from "@/lib/upload/hash";
import {
  createUploader,
  libraryItem,
  responseText,
  serverMessage,
  type Uploader,
} from "@/lib/upload/uploader";

/** One file on its way into the library. */
export interface Upload {
  key: string;
  name: string;
  phase: "hashing" | "uploading" | "done" | "duplicate" | "failed";
  /** 0–1 through the current phase. */
  progress: number;
  /** Why it stopped, for `duplicate` and `failed`. */
  message?: string;
  /** The library item it is (or duplicates), once known. */
  item?: number;
}

let nextKey = 0;

/** What the provider hands down. */
export interface Uploads {
  /** Every upload this session started, newest first. */
  uploads: Upload[];
  /** Hash, check and upload these files. */
  add: (files: Iterable<File>) => Promise<void>;
  /** Forget the rows that have finished, one way or another. */
  dismiss: () => void;
}

const UploadsContext = createContext<Uploads | null>(null);

/** The uploads in progress; only inside {@link UploadsProvider}. */
export function useUploads(): Uploads {
  const uploads = useContext(UploadsContext);
  if (!uploads) throw new Error("useUploads is used outside <UploadsProvider>");
  return uploads;
}

/** Keeps one uploader for as long as the signed-in app is open. */
export function UploadsProvider({ children }: { children: ReactNode }) {
  const value = useUploader();
  return <UploadsContext.Provider value={value}>{children}</UploadsContext.Provider>;
}

function useUploader(): Uploads {
  const queryClient = useQueryClient();
  const t = useT();
  const [uploads, setUploads] = useState<Upload[]>([]);
  const uploader = useRef<Uploader | null>(null);
  // Uppy's file id → the row it is drawn as.
  const rows = useRef(new Map<string, string>());

  const update = useCallback((key: string, change: Partial<Upload>) => {
    setUploads((all) => all.map((row) => (row.key === key ? { ...row, ...change } : row)));
  }, []);

  useEffect(() => {
    const uppy = createUploader();
    uploader.current = uppy;
    const rowOf = (id: string | undefined) => (id ? rows.current.get(id) : undefined);
    const finish = (id: string, change: Partial<Upload>) => {
      const key = rowOf(id);
      if (key) update(key, change);
      rows.current.delete(id);
      queryClient.invalidateQueries({ queryKey: ["library"] });
    };
    uppy.on("upload-progress", (file, progress) => {
      const key = rowOf(file?.id);
      const total = progress.bytesTotal ?? 0;
      if (key && total > 0) update(key, { progress: progress.bytesUploaded / total });
    });
    // A finished file stays in Uppy: removing it would have the tus plugin
    // terminate an upload the server has already turned into a library item.
    // Picking the same bytes again is caught by the hash check before Uppy.
    uppy.on("upload-success", (file, response) => {
      if (file) finish(file.id, { phase: "done", progress: 1, item: libraryItem(response) });
    });
    uppy.on("upload-error", (file, error, response) => {
      if (!file) return;
      const body = responseText(response);
      const duplicate = duplicateOf(response?.status, body);
      finish(
        file.id,
        duplicate
          ? { phase: "duplicate", message: duplicate.message, item: duplicate.item }
          : { phase: "failed", message: serverMessage(body) ?? error.message },
      );
      // A failed file leaves, so the user can pick it again once it is fixed.
      uppy.removeFile(file.id);
    });
    return () => {
      uppy.destroy();
      uploader.current = null;
    };
  }, [queryClient, update]);

  const add = useCallback(
    async (files: Iterable<File>) => {
      for (const file of files) {
        const key = `upload-${nextKey++}`;
        setUploads((all) => [{ key, name: file.name, phase: "hashing", progress: 0 }, ...all]);
        try {
          const sha256 = await hashFile(file, (progress) => update(key, { progress }));
          const [existing] = await api.library.list({ sha256 });
          if (existing) {
            const duplicate = alreadyHave(existing, t.files.uploads.alreadyHave);
            update(key, { phase: "duplicate", message: duplicate.message, item: duplicate.item });
            continue;
          }
          update(key, { phase: "uploading", progress: 0 });
          const id = uploader.current?.addFile({
            name: file.name,
            type: file.type,
            data: file,
            meta: { name: file.name, sha256 },
          });
          if (id) rows.current.set(id, key);
        } catch (error) {
          update(key, { phase: "failed", message: (error as Error).message });
        }
      }
    },
    [update, t],
  );

  const dismiss = useCallback(() => {
    setUploads((all) => all.filter((row) => row.phase === "hashing" || row.phase === "uploading"));
  }, []);

  return { uploads, add, dismiss };
}
