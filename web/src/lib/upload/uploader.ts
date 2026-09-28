// The Uppy instance every upload goes through: tus against `/api/uploads`.
//
// Why tus at all: the Cloudflare tunnel in front of the server refuses a
// request body over ~100 MB, so a file travels in chunks under that, and a
// dropped connection resumes from the last chunk that landed rather than from
// zero (docs/web.md, *Library*). The server speaks exactly the tus subset
// Uppy's plugin needs; `crates/server/src/http/uploads.rs` has which.

import Uppy from "@uppy/core";
import Tus from "@uppy/tus";
import { shouldRetry } from "./duplicate";

/** Chunks stay well under Cloudflare's 100 MB request cap. */
export const CHUNK_BYTES = 50 * 1024 * 1024;

/** What each file carries: the server reads `name` and `sha256` from it. */
export interface UploadMeta {
  name: string;
  sha256: string;
  [key: string]: unknown;
}

export type Uploader = Uppy<UploadMeta, Record<string, never>>;

/** A new uploader. Files added to it start uploading at once. */
export function createUploader(): Uploader {
  return new Uppy<UploadMeta, Record<string, never>>({ autoProceed: true }).use(Tus, {
    endpoint: "/api/uploads",
    chunkSize: CHUNK_BYTES,
    // Only what the server reads; Uppy would otherwise send every field it
    // keeps (type, relative path…) in `Upload-Metadata`.
    allowedMetaFields: ["name", "sha256"],
    onShouldRetry: (error, _attempt, _options, next) =>
      shouldRetry(error.originalResponse, () => next(error)),
  });
}

/** The part of a response Uppy hands to `upload-success` and `upload-error`. */
export interface UppyResponse {
  body?: unknown;
}

function xhrOf(response: UppyResponse | undefined): XMLHttpRequest | undefined {
  return (response?.body as { xhr?: XMLHttpRequest } | undefined)?.xhr;
}

/** The response body's text, when there is one. */
export function responseText(response: UppyResponse | undefined): string | undefined {
  return xhrOf(response)?.responseText;
}

/** The library item a finished upload became: the last chunk's `Scorsese-Library-Item`. */
export function libraryItem(response: UppyResponse | undefined): number | undefined {
  const id = Number(xhrOf(response)?.getResponseHeader("Scorsese-Library-Item"));
  return Number.isInteger(id) && id > 0 ? id : undefined;
}

/**
 * The server's own sentence from a refusal's `{"error": "…"}` body — it says
 * what was wrong with the file, where tus's message says which request failed.
 */
export function serverMessage(body: string | undefined): string | undefined {
  try {
    const parsed = JSON.parse(body ?? "");
    return typeof parsed?.error === "string" ? parsed.error : undefined;
  } catch {
    return undefined;
  }
}
