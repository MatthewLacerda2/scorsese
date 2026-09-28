// Uploading: the hash the server is told, and the one 409 never retried.

import { expect, test } from "bun:test";
import { alreadyHave, duplicateOf, shouldRetry } from "./duplicate";
import { hashFile } from "./hash";
import { serverMessage } from "./uploader";

const response = (status: number, body: unknown) => ({
  getStatus: () => status,
  getBody: () => (typeof body === "string" ? body : JSON.stringify(body)),
});

test("a duplicate is a 409 whose body names the item", () => {
  const body = JSON.stringify({ error: "you already have this as “intro”", item: 7 });
  expect(duplicateOf(409, body)).toEqual({
    item: 7,
    message: "you already have this as “intro”",
  });
});

test("a misplaced chunk's 409 is not a duplicate, nor is anything else", () => {
  expect(duplicateOf(409, JSON.stringify({ error: "wrong offset", offset: 1024 }))).toBeNull();
  expect(duplicateOf(409, "not json")).toBeNull();
  expect(duplicateOf(409, "")).toBeNull();
  expect(duplicateOf(415, JSON.stringify({ error: "not a video", item: 7 }))).toBeNull();
  expect(duplicateOf(undefined, undefined)).toBeNull();
});

test("tus never retries a duplicate, and defers to its default otherwise", () => {
  let asked = 0;
  const fallback = () => {
    asked += 1;
    return true;
  };
  expect(shouldRetry(response(409, { error: "dup", item: 3 }), fallback)).toBe(false);
  expect(asked).toBe(0);
  expect(shouldRetry(response(409, { error: "offset", offset: 0 }), fallback)).toBe(true);
  expect(shouldRetry(response(423, { error: "busy" }), fallback)).toBe(true);
  expect(shouldRetry(null, fallback)).toBe(true);
  expect(asked).toBe(3);
});

test("a duplicate found by hash reads like the server's refusal", () => {
  expect(alreadyHave({ id: 5, name: "intro" })).toEqual({
    item: 5,
    message: "you already have this as “intro”",
  });
});

test("a refusal is shown in the server's words when it gave some", () => {
  expect(serverMessage('{"error":"that is not a file scorsese can import"}')).toBe(
    "that is not a file scorsese can import",
  );
  expect(serverMessage("<html>bad gateway</html>")).toBeUndefined();
  expect(serverMessage(undefined)).toBeUndefined();
});

test("a file is hashed as SHA-256 hex, streamed, with progress up to 1", async () => {
  const seen: number[] = [];
  const hash = await hashFile(new Blob(["abc"]), (fraction) => seen.push(fraction));
  expect(hash).toBe("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  expect(seen.at(-1)).toBe(1);
  expect(await hashFile(new Blob([]))).toBe(
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  );
});
