// What a page is told, and which of the clips and words it read (#810, #811).
//
// Called with the contract (viewport, rate, length), the clips and the words,
// before any of the page's own scripts run. `window.scorsese` is the contract
// with `clips` and `words` beside it, all frozen. Both are watched: every name
// a page looks up on one is noted — names nothing has too, since a clip or a
// word given one later changes what the page draws — and so is listing them,
// which makes every one part of what it drew. `__scorseseTold()` hands both
// back to the capture.
//
// Inherited names (`hasOwnProperty`, `toString`) are not noted unless a clip
// or word really has one: they are how a page uses the object, not what it read.
((contract, clips, words) => {
  const watch = (spans) => {
    const names = new Set();
    const seen = { names, listed: false };
    for (const span of Object.values(spans)) Object.freeze(span);
    const target = Object.freeze(spans);
    const note = (key) => {
      if (typeof key !== "string") return;
      if (Object.hasOwn(target, key) || !(key in Object.prototype)) names.add(key);
    };
    seen.proxy = new Proxy(target, {
      get(target, key, receiver) {
        note(key);
        return Reflect.get(target, key, receiver);
      },
      has(target, key) {
        note(key);
        return Reflect.has(target, key);
      },
      getOwnPropertyDescriptor(target, key) {
        note(key);
        return Reflect.getOwnPropertyDescriptor(target, key);
      },
      ownKeys(target) {
        seen.listed = true;
        return Reflect.ownKeys(target);
      },
    });
    return seen;
  };
  const toldClips = watch(clips);
  const toldWords = watch(words);
  Object.defineProperty(window, "scorsese", {
    value: Object.freeze({ ...contract, clips: toldClips.proxy, words: toldWords.proxy }),
  });
  Object.defineProperty(window, "__scorseseTold", {
    value: () => ({
      names: [...toldClips.names],
      listed: toldClips.listed,
      words: [...toldWords.names],
      listed_words: toldWords.listed,
    }),
    enumerable: false,
  });
})
