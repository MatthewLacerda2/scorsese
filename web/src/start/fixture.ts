// A small menu for tests: two placements, three styles, no previews but one.

import type { StyleMenu } from "@/api";

export const MENU: StyleMenu = {
  platforms: [
    { id: "youtube", name: "YouTube", ad: false, width: 1920, height: 1080 },
    { id: "tiktok_ad", name: "Anúncio no TikTok", ad: true, width: 1080, height: 1920 },
  ],
  styles: [
    {
      id: "kinetic_type",
      name: "Tipografia em movimento",
      description: "Palavras grandes no ritmo da música.",
      platforms: ["youtube", "tiktok_ad"],
      preview: null,
    },
    {
      id: "flash_offer",
      name: "Oferta relâmpago",
      description: "O produto, o preço e a urgência.",
      platforms: ["tiktok_ad"],
      preview: null,
    },
    {
      id: "top_list",
      name: "Lista / Top N",
      description: "Uma contagem regressiva.",
      platforms: ["youtube"],
      preview: "/previews/top_list.gif",
    },
  ],
};
