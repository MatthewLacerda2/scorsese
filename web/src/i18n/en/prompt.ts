// The shared prompt box, on the landing page and the signed-in home.

export const prompt = {
  // Typed into the empty box one after another (`useTypewriter`), at most 50 characters each, the kinds of
  // video a person with an idea and some footage would ask for.
  examples: [
    "Instagram launch video for my new sneakers",
    "Birthday montage for my mom's 60th",
    "Narrated explainer on how our app saves time",
    "Real-estate tour of a sunny two-bedroom flat",
    "Wedding highlights with the vows as audio",
    "15-second promo for my bakery's weekend deal",
    "Trip recap with captions for each day",
    "YouTube intro for my cooking channel",
  ],
  label: "Describe your video",
  send: "Send",
  // The files attached to a prompt (#1534): the "+", the tray above the box, a drop.
  attach: {
    add: "Attach files",
    tray: "Attached files",
    remove: (name: string) => `Remove ${name}`,
    refused: (name: string) => `${name}: scorsese can't use this kind of file`,
    drop: "Drop files to attach them",
    failed: "This file could not be uploaded",
  },
};
