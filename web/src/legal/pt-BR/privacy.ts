// The privacy policy in Portuguese, the binding text (`document.ts`; LGPD). It
// is a list of facts about the deployment: what the server keeps, which
// vendor each request goes to, the session cookie's 30 days
// (`accounts::sessions::LIFETIME_DAYS`) and the backups' 30
// (`SCORSESE_BACKUP_KEEP_DAYS`). A change to any of those, or a new vendor, is
// a change to this text and to `updated`.

import type { LegalDocument } from "@/legal/document";
import { SELLER } from "@/legal/seller";

export const PRIVACY_PT: LegalDocument = {
  title: "Política de privacidade",
  updatedLabel: "Última atualização",
  updated: "2026-10-25",
  summary: {
    heading: "Em resumo",
    points: [
      "Você entra com o Google e guardamos só o seu nome, o e-mail e o identificador da conta Google, nunca uma senha.",
      "Os seus arquivos e vídeos são seus: ficam no nosso servidor no Brasil, separados dos de outras pessoas.",
      "Para gerar o que você pede, cada fornecedor recebe só o necessário (a lista está abaixo).",
      "Não vendemos dados, não mostramos anúncios e não usamos o seu conteúdo para treinar inteligência artificial.",
      "Você pode pedir uma cópia ou a exclusão de tudo por e-mail, e respondemos em até 15 dias.",
    ],
  },
  sections: [
    {
      heading: "Quem cuida dos seus dados",
      blocks: [
        `O controlador dos seus dados pessoais, nos termos da Lei Geral de Proteção de Dados (LGPD, Lei 13.709/2018), é ${SELLER.name}, CNPJ ${SELLER.cnpj}, ${SELLER.address}. O canal para qualquer assunto de privacidade, inclusive os seus pedidos como titular, é ${SELLER.email}.`,
      ],
    },
    {
      heading: "O que guardamos",
      blocks: [
        [
          "Conta: o seu e-mail, o seu nome e o identificador da sua conta Google, recebidos quando você entra com o Google. A sua foto do Google não é guardada.",
          "Conteúdo: os arquivos que você envia (que podem mostrar pessoas, inclusive o rosto e a voz delas), os seus projetos, o que é gerado para você e os textos que você escreve para pedir cada geração.",
          "Conversas com o assistente: as mensagens e o registro de cada ação que o assistente faz no seu projeto. A gente usa esse registro para encontrar erros e melhorar o assistente.",
          "Créditos e pagamentos: o seu saldo, o histórico de gastos e, de cada compra, o valor, a cotação usada e o identificador do pagamento na Stripe. Os dados do seu cartão vão direto para a Stripe e nunca passam pelo nosso servidor.",
          "Registros de acesso: o endereço IP, a data e a hora de cada vez que você entra com o Google e do primeiro acesso de cada dia, guardados por no mínimo 6 meses, como determina o Marco Civil da Internet (Lei 12.965/2014).",
          ""Registros do servidor: para cada requisição ao site, o endereço IP, a localização aproximada que a Cloudflare deduz dele (cidade, estado e país), o navegador (user agent), o endereço pedido e a data e a hora, usados para manter o serviço seguro e encontrar erros. Eles nunca registram o seu nome ou e-mail. Ficam guardados em rodízio limitado por tamanho, em que os mais antigos são apagados à medida que chegam novos.",
        ],
      ],
    },
    {
      heading: "Para que usamos",
      blocks: [
        [
          "Para prestar o serviço que você contratou: guardar e editar os seus projetos, gerar o que você pede e cobrar por isso (execução de contrato, art. 7º, V, da LGPD).",
          "Para cumprir obrigações legais, como as fiscais e a guarda dos registros de acesso (art. 7º, II).",
          "Para manter o serviço seguro e melhorar o assistente a partir do registro das conversas (legítimo interesse, art. 7º, IX).",
        ],
        "Não vendemos os seus dados, não mostramos anúncios e não usamos o seu conteúdo para treinar modelos de inteligência artificial.",
      ],
    },
    {
      heading: "Com quem compartilhamos",
      blocks: [
        "Cada fornecedor recebe só o que precisa para fazer a sua parte:",
        [
          {
            text: "Anthropic (o assistente Claude): as suas mensagens e o conteúdo do projeto que o assistente lê para responder.",
            link: {
              label: "Política de privacidade",
              href: "https://www.anthropic.com/legal/privacy",
            },
          },
          {
            text: "Google: os pedidos de vídeo (Veo) e de imagem (Gemini), com as imagens de referência que você indicar; e o login com o Google.",
            link: { label: "Política de privacidade", href: "https://policies.google.com/privacy" },
          },
          {
            text: "ElevenLabs: os textos a serem narrados e as descrições de voz.",
            link: {
              label: "Política de privacidade",
              href: "https://elevenlabs.io/privacy-policy",
            },
          },
          {
            text: "Stripe: o pagamento. A Stripe recebe o seu e-mail e os dados do cartão que você digita na página dela.",
            link: { label: "Política de privacidade", href: "https://stripe.com/privacy" },
          },
          {
            text: "Cloudflare: todo acesso ao site passa por ela, que protege e entrega as páginas.",
            link: {
              label: "Política de privacidade",
              href: "https://www.cloudflare.com/privacypolicy/",
            },
          },
          {
            text: "Pixabay: só os termos de busca, quando o assistente procura vídeos ou fotos de banco.",
            link: {
              label: "Política de privacidade",
              href: "https://pixabay.com/service/privacy/",
            },
          },
          {
            text: "LottieFiles: só os termos de busca, quando o assistente procura animações de banco.",
            link: {
              label: "Política de privacidade",
              href: "https://lottiefiles.com/page/privacy-policy",
            },
          },
        ],
        "Esses fornecedores ficam, em sua maioria, fora do Brasil, principalmente nos Estados Unidos. A transferência é feita para executar o serviço que você contratou (art. 33, IX, da LGPD) e segue as regras de proteção de dados de cada um deles.",
        "Fora isso, só compartilhamos dados se uma lei ou uma ordem judicial exigir.",
      ],
    },
    {
      heading: "Onde ficam e por quanto tempo",
      blocks: [
        "Os seus dados ficam no nosso servidor, no Brasil, e numa cópia de segurança, refeita periodicamente e mantida por 30 dias.",
        "Eles ficam guardados enquanto a sua conta existir. Quando a conta é encerrada, tudo é apagado do servidor, e as cópias de segurança que ainda tiverem os seus dados expiram em até 30 dias. Os registros de pagamento continuam na Stripe e na nossa contabilidade pelo prazo que a lei fiscal exige, e os registros de acesso continuam guardados, como determina o Marco Civil.",
      ],
    },
    {
      heading: "Os seus direitos",
      blocks: [
        `Pela LGPD (art. 18), você pode pedir a qualquer momento, por ${SELLER.email}: a confirmação de que tratamos os seus dados e o acesso a eles; a correção do que estiver errado; uma cópia dos seus dados; a lista de com quem os compartilhamos; e a exclusão dos seus dados e da sua conta. Respondemos em até 15 dias.`,
        "Você também pode reclamar à Autoridade Nacional de Proteção de Dados (ANPD).",
      ],
    },
    {
      heading: "Cookies",
      blocks: [
        "Usamos apenas os cookies necessários para o site funcionar: um que mantém você conectado por até 30 dias e outro, temporário, usado durante o login com o Google. Não usamos cookies de publicidade nem de análise. O idioma e o tema que você escolhe ficam guardados no seu próprio navegador.",
      ],
    },
    {
      heading: "Segurança",
      blocks: [
        "O acesso ao site é sempre criptografado, os arquivos de cada pessoa ficam separados dos das outras, e o que o assistente executa a partir do seu conteúdo roda isolado e sem acesso à internet. Nenhum sistema é totalmente seguro: se acontecer um incidente que possa trazer risco a você, avisaremos você e a ANPD, como manda a lei.",
      ],
    },
    {
      heading: "Menores de idade",
      blocks: [
        "O scorsese é para maiores de 18 anos e não coleta, de propósito, dados de menores.",
      ],
    },
    {
      heading: "Se você não mora no Brasil",
      blocks: [
        "O scorsese é operado a partir do Brasil e segue a LGPD. Os direitos descritos acima valem para qualquer pessoa, onde quer que more, pelo mesmo e-mail. Se você mora na União Europeia ou no Reino Unido, também pode reclamar à autoridade de proteção de dados do seu país. Não vendemos nem compartilhamos dados pessoais para publicidade, no sentido de leis como a da Califórnia (CCPA).",
      ],
    },
    {
      heading: "Mudanças nesta política",
      blocks: [
        "Se esta política mudar de forma relevante, avisaremos por e-mail ou no site antes de a mudança valer. A data no topo diz quando ela foi alterada pela última vez.",
      ],
    },
  ],
};
