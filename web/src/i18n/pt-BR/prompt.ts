import type { Messages } from "@/i18n/catalogue";

// The shared prompt box, on the landing page and the signed-in home.

export const prompt: Messages["prompt"] = {
  // Typed into the empty box one after another (`useTypewriter`), at most 50 characters each, the kinds of
  // video a person with an idea and some footage would ask for.
  examples: [
    "Vídeo de lançamento do meu tênis no Instagram",
    "Montagem de aniversário de 60 anos da minha mãe",
    "Vídeo explicativo narrado sobre o nosso app",
    "Tour por um apartamento de dois quartos",
    "Melhores momentos do casamento, com os votos",
    "Anúncio de 15s da promoção da minha padaria",
    "Resumo da viagem com legenda de cada dia",
    "Vinheta para o meu canal de culinária",
  ],
  label: "Descreva seu vídeo",
  send: "Enviar",
};
