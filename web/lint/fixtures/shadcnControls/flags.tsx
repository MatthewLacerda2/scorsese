export const a = <button type="button" />; // flagged
export const b = <button type="button">{1}</button>; // flagged
export const c = <select />; // flagged
export const d = <select>{1}</select>; // flagged
export const e = <textarea />; // flagged
export const f = <input />; // flagged
export const g = <input type="text" />; // flagged
export const h = <input type="number" value={1} />; // flagged
export const i = <input type="radio" />; // flagged
