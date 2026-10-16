// The terms of use (#547), in Portuguese (`document.ts` has why). What they
// promise about money is what the code does: the price shown before a
// generation (`credits::Product`), failed generations released, credits that
// never expire, R$ 20 to R$ 5.000 per purchase (`payments`). A change to any
// of those is a change to this text, and to `updated`.

import type { LegalDocument } from "@/legal/document";
import { SELLER } from "@/legal/seller";

export const TERMS: LegalDocument = {
  title: "Termos de uso",
  updated: "2026-10-16",
  sections: [
    {
      heading: "Quem somos",
      blocks: [
        `O scorsese, em ${SELLER.site}, é oferecido por ${SELLER.name}, CNPJ ${SELLER.cnpj}, com endereço em ${SELLER.address}. Para falar com a gente sobre qualquer assunto, inclusive cancelamento, reembolso e seus dados, escreva para ${SELLER.email}.`,
        "Ao criar sua conta ou usar o site, você concorda com estes termos e com a nossa Política de privacidade. Se não concordar, não use o serviço.",
      ],
    },
    {
      heading: "O serviço",
      blocks: [
        "O scorsese é um editor de vídeo com um assistente: você envia seus arquivos, descreve o vídeo que quer, e o assistente monta e edita o vídeo para você. Parte do trabalho (narração, imagens e vídeos gerados, e o próprio assistente) é feita por serviços de inteligência artificial de terceiros, que a gente contrata e repassa.",
        "O programa do scorsese também existe como software livre, que qualquer pessoa pode instalar e usar no próprio computador com as próprias chaves. Estes termos tratam só do site, que é a versão pronta para usar.",
      ],
    },
    {
      heading: "Sua conta",
      blocks: [
        "Você entra com a sua conta Google, e o primeiro acesso cria a sua conta no scorsese. É preciso ter 18 anos ou mais.",
        "A conta é pessoal. Você é responsável pelo que for feito com ela e deve nos avisar se suspeitar que alguém a está usando sem permissão.",
      ],
    },
    {
      heading: "Créditos e preços",
      blocks: [
        "Não há mensalidade. Você compra créditos e eles são gastos conforme o uso.",
        [
          "Créditos são comprados em reais, de R$ 20 a R$ 5.000 por compra, pela Stripe, por Pix ou cartão de crédito Visa ou Mastercard (outras bandeiras, como Elo e American Express, e cartões de débito não são aceitos). O valor é convertido para dólares americanos (US$) pela cotação mostrada na hora da compra, porque é em dólar que os serviços que usamos nos cobram. O seu saldo é mostrado em US$.",
          "Antes de cada geração (narração, imagem, vídeo e outras), o scorsese mostra quanto ela vai custar e só gera depois que você confirma.",
          "Cada mensagem ao assistente consome créditos conforme o trabalho que ele faz para responder, com um limite máximo por mensagem. O custo de cada uma aparece no seu histórico de gastos.",
          "Créditos não expiram.",
          "Os preços acompanham o que os fornecedores nos cobram e podem mudar. Uma mudança nunca afeta uma geração que você já confirmou.",
        ],
      ],
    },
    {
      heading: "Gerações que falham e resultados que não agradam",
      blocks: [
        "Se uma geração falhar, por erro nosso ou do fornecedor, ela não é cobrada e os créditos voltam para o seu saldo.",
        "Uma geração que foi feita mas não ficou do seu gosto é cobrada normalmente, porque o fornecedor nos cobra por ela do mesmo jeito. Você pode conferir cada cobrança no seu histórico de gastos.",
      ],
    },
    {
      heading: "Reembolso e direito de arrependimento",
      blocks: [
        `Você pode desistir de uma compra de créditos em até 7 dias depois de fazê-la, como prevê o artigo 49 do Código de Defesa do Consumidor. Basta escrever para ${SELLER.email}. A gente devolve a parte daquela compra que ainda não foi gasta, pelo mesmo meio de pagamento.`,
        "Passados os 7 dias, os créditos comprados não são reembolsados, mas continuam no seu saldo, sem prazo para usar.",
      ],
    },
    {
      heading: "O seu conteúdo",
      blocks: [
        "Os arquivos que você envia e os vídeos que você faz são seus. Você nos autoriza a guardá-los, processá-los e enviá-los aos fornecedores apenas na medida necessária para prestar o serviço, como descreve a Política de privacidade.",
        "Você declara ter os direitos necessários sobre o que envia, inclusive a imagem e a voz das pessoas que aparecem nos seus arquivos. Não é permitido usar o scorsese para:",
        [
          "conteúdo ilegal, ou que envolva sexualização de menores, em qualquer forma;",
          "imitar uma pessoa real sem a permissão dela, inclusive a voz, ou enganar alguém sobre quem fala ou aparece num vídeo;",
          "violar direitos autorais, de marca ou de imagem de terceiros;",
          "assédio, discurso de ódio ou incentivo à violência;",
          "tentar burlar os limites, as cobranças ou a segurança do serviço.",
        ],
        "Os fornecedores de inteligência artificial têm suas próprias regras e podem recusar um pedido. A gente pode suspender ou encerrar uma conta que viole estes termos.",
      ],
    },
    {
      heading: "Resultados de inteligência artificial",
      blocks: [
        "O que a inteligência artificial gera pode sair diferente do que você pediu, ou conter erros. Revise o vídeo antes de publicá-lo ou usá-lo comercialmente. Você é responsável pelo uso que faz dele.",
      ],
    },
    {
      heading: "Disponibilidade",
      blocks: [
        "O scorsese roda num servidor próprio e pode ficar fora do ar por manutenção ou por falha. A gente faz cópias de segurança, mas recomenda que você guarde também uma cópia dos arquivos e vídeos que forem importantes.",
        "Na medida em que a lei permite, a nossa responsabilidade por danos ligados ao serviço fica limitada ao valor que você pagou nos 12 meses anteriores.",
      ],
    },
    {
      heading: "Encerramento da conta",
      blocks: [
        `Você pode pedir o encerramento da sua conta a qualquer momento, por ${SELLER.email}. A conta, os projetos e os arquivos são apagados em até 15 dias. O saldo que restar se perde, exceto o que estiver dentro do prazo de arrependimento descrito acima, que é devolvido.`,
      ],
    },
    {
      heading: "Mudanças nestes termos",
      blocks: [
        "Se mudarmos estes termos de forma relevante, avisaremos por e-mail ou no site com pelo menos 15 dias de antecedência. Continuar usando o serviço depois disso significa concordar com a nova versão.",
      ],
    },
    {
      heading: "Lei e foro",
      blocks: [
        "Estes termos seguem a lei brasileira. Fica eleito o foro de Brasília, Distrito Federal, sem prejuízo do seu direito de, como consumidor, entrar com uma ação no foro do seu domicílio.",
      ],
    },
  ],
};
