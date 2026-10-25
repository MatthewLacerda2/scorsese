// scorsese's mark in the page's theme: white on black in the dark theme, black
// on white in the light one (app/assets/README.md has how each is made). Both
// are in the markup and the theme's `dark` class shows one, so every mark on a
// page changes the moment the theme does, with nothing to keep in step.
//
// `icon` is the square one (the favicon's art, the account button, the sign-in
// popup); `silhouette` has no background, for the landing page's first screen.

import { cn } from "cn";
import silhouetteDark from "@/landing/media/mark.png";
import silhouetteLight from "@/landing/media/mark-light.png";

const SOURCES = {
  icon: { light: "/icon-light.png", dark: "/icon.png" },
  silhouette: { light: silhouetteLight, dark: silhouetteDark },
} as const;

export function Mark({
  kind,
  className,
  size,
  ...rest
}: {
  kind: keyof typeof SOURCES;
  className?: string;
  /** The picture's own width and height, so the page does not jump as it loads. */
  size: number;
} & Record<`data-${string}`, string>) {
  const src = SOURCES[kind];
  return (
    <>
      <img
        src={src.light}
        alt=""
        width={size}
        height={size}
        className={cn(className, "dark:hidden")}
        {...rest}
      />
      <img
        src={src.dark}
        alt=""
        width={size}
        height={size}
        className={cn(className, "hidden dark:block")}
        {...rest}
      />
    </>
  );
}
