// What a page is told, and which of the clips it read (#810).
//
// Called with the contract (viewport, rate, length) and the clips, before any
// of the page's own scripts run. `window.scorsese` is the contract with
// `clips` beside it, all frozen. `clips` is watched: every name a page looks
// up on it is noted — names no clip has too, since a clip given one later
// changes what the page draws — and so is listing them, which makes every clip
// part of what it drew. `__scorseseTold()` hands both back to the capture.
//
// Inherited names (`hasOwnProperty`, `toString`) are not noted unless a clip
// really has one: they are how a page uses the object, not clips it read.
((contract, clips) => {
  const names = new Set();
  let listed = false;
  for (const span of Object.values(clips)) Object.freeze(span);
  const target = Object.freeze(clips);
  const note = (key) => {
    if (typeof key !== "string") return;
    if (Object.hasOwn(target, key) || !(key in Object.prototype)) names.add(key);
  };
  const watched = new Proxy(target, {
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
      listed = true;
      return Reflect.ownKeys(target);
    },
  });
  Object.defineProperty(window, "scorsese", {
    value: Object.freeze({ ...contract, clips: watched }),
  });
  Object.defineProperty(window, "__scorseseTold", {
    value: () => ({ names: [...names], listed }),
    enumerable: false,
  });
})
