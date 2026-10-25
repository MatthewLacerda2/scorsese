// The files attached to one prompt (#1534): what the tray shows, what a
// send carries. Signed in, each file goes into the library at once, through
// the uploads every page shares (`files/uploads.tsx`), so its duplicate rule
// holds and a file already there never re-crosses the network; signed out
// (the landing), they are only held, for the sign-in round trip to carry.
// Taking one off forgets it here and leaves the library as it is.

import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { FileKind } from "@/api";
import { formatsQuery, useFormats } from "@/app/queries";
import { useOptionalUploads } from "@/files/uploads";
import { accept, sort } from "@/lib/upload/formats";
import { busy, isSettled, items, type Status, statusOf } from "./status";

/** One attached file and where it is. */
export interface Attachment {
  key: string;
  file: File;
  /** By its extension; `null` while the library's list has not loaded. */
  kind: FileKind | null;
  status: Status;
}

export interface Attachments {
  list: Attachment[];
  /** Attach these, refusing by name what the library does not take. */
  add: (files: Iterable<File>) => void;
  remove: (key: string) => void;
  /** Take these off the tray: the message that carried them went. A file added since stays. */
  clear: (keys: readonly string[]) => void;
  /** The names the last pick or drop refused. */
  refused: string[];
  /** A file is still on its way, so sending waits. */
  busy: boolean;
  /** The library items that go with the message. */
  items: number[];
  /** The file picker's hint, once the list is in. */
  accept: string | undefined;
  /** Someone is reaching for the box: ask what the library takes, if not yet asked. */
  prepare: () => void;
}

let nextKey = 0;

interface Held {
  key: string;
  file: File;
  kind: FileKind | null;
  /** Its upload row, signed in. */
  upload?: string;
  /** What it came to, kept once final: the row may be cleared. */
  settled?: Status;
}

export function useAttachments(): Attachments {
  const uploads = useOptionalUploads();
  const queryClient = useQueryClient();
  const [wanted, setWanted] = useState(false);
  const formats = useFormats(wanted).data;
  const [held, setHeld] = useState<Held[]>([]);
  const [refused, setRefused] = useState<string[]>([]);

  const add = useCallback(
    async (files: Iterable<File>) => {
      const picked = Array.from(files);
      setWanted(true);
      // Refused by the list, never by a guess: it is asked for if not in yet,
      // and only when it cannot be had does the server's own refusal stand in.
      const list = await queryClient.ensureQueryData(formatsQuery).catch(() => undefined);
      const sorted = sort(picked, list);
      setRefused(sorted.refused);
      const rows = uploads?.add(sorted.taken.map(({ file }) => file));
      const added = sorted.taken.map(
        ({ file, kind }, index): Held => ({
          key: `attachment-${nextKey++}`,
          file,
          kind,
          upload: rows?.[index],
        }),
      );
      setHeld((now) => [...now, ...added]);
    },
    [queryClient, uploads],
  );
  const remove = useCallback((key: string) => {
    setHeld((now) => now.filter((one) => one.key !== key));
    setRefused([]);
  }, []);
  const clear = useCallback((keys: readonly string[]) => {
    const sent = new Set(keys);
    setHeld((now) => now.filter((one) => !sent.has(one.key)));
    setRefused([]);
  }, []);

  const rows = uploads?.uploads;
  const list = useMemo(
    () => held.map((one) => ({ ...one, status: statusOf(one.upload, rows ?? [], one.settled) })),
    [held, rows],
  );
  // Keep what each upload came to, so clearing the uploads corner changes nothing here.
  useEffect(() => {
    const final = new Map(
      list
        .filter((one) => !one.settled && isSettled(one.status))
        .map((one) => [one.key, one.status]),
    );
    if (final.size === 0) return;
    setHeld((now) => now.map((one) => ({ ...one, settled: one.settled ?? final.get(one.key) })));
  }, [list]);
  const statuses = list.map((one) => one.status);
  const prepare = useCallback(() => setWanted(true), []);
  return {
    list,
    add: (files) => void add(files),
    remove,
    clear,
    refused,
    busy: busy(statuses),
    items: items(statuses),
    accept: accept(formats),
    prepare,
  };
}
