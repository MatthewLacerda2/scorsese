// Every line marked `flagged` must be reported, and no other.
declare const name: string | undefined;
declare const busy: boolean;
declare const count: number;
declare function Field(props: { label: string; placeholder?: string }): JSX.Element;

export const text = <p>Save the cut</p>; // flagged
export const child = <p>{"Untitled"}</p>; // flagged
export const either = <p>{busy ? "Saving" : "Saved"}</p>; // flagged
export const fallback = <p>{name ?? "Untitled"}</p>; // flagged
export const or = <p>{name || "Untitled"}</p>; // flagged
export const and = <p>{busy && "Saving"}</p>; // flagged
export const nested = <p>{busy ? "Saving" : (name ?? "Untitled")}</p>; // flagged
export const template = <p>{`${count} clips`}</p>; // flagged
export const inside = [1].map((i) => <li key={i}>Clip</li>); // flagged
export const after = <p>{name}, welcome back</p>; // flagged
export const ariaLabel = <button type="button" aria-label="Close" />; // flagged
export const ariaText = <div aria-roledescription="splitter" />; // flagged
export const alt = <img alt="A still from the cut" src="x.png" />; // flagged
export const title = <span title={busy ? "Busy" : "Idle"} />; // flagged
export const placeholder = <input placeholder="Search your files" />; // flagged
export const prop = <Field label="Project name" />; // flagged
export const word = <p>Ok</p>; // flagged
