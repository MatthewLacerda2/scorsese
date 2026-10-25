// A project's name from the prompt that starts it: the three longest words that
// are not stopwords, in the order the prompt has them (the longest word of two
// equally long ones is the earlier). A prompt with none gives the fallback the
// caller passes ("Novo vídeo 24/10"). The stopwords are the three languages'
// together, since a prompt need not be in the language of the site.

const STOPWORDS = new Set(
  `the a an and or of to for in on with my our your is are was be this that it its at by from as
   about into me i we you they he she video videos
   o a os as um uma uns umas de do da dos das em no na nos nas para por com sem e ou que meu minha
   meus minhas seu sua seus suas nosso nossa ao aos à às pelo pela como mais sobre entre esse essa
   isso este esta isto é são foi ser vídeo vídeos
   el la los las un una unos unas del y mi mis tu tus su sus nuestro nuestra al lo es son fue
   eso esto más`.split(/\s+/),
);

/** The most words a name takes. */
const WORDS = 3;
/** The longest a name may be, in characters. */
const MAX_LENGTH = 60;

/** The words of `prompt`: runs of letters and digits, apostrophes inside a word kept out. */
function wordsOf(prompt: string): string[] {
  return prompt.match(/[\p{L}\p{N}]+/gu) ?? [];
}

/** `prompt`'s name, or `fallback` when it has no word worth naming a project by. */
export function projectName(prompt: string, fallback: string): string {
  const candidates = wordsOf(prompt)
    .map((word, order) => ({ word, order }))
    .filter(({ word }) => !STOPWORDS.has(word.toLowerCase()));
  const longest = [...candidates]
    .sort((a, b) => b.word.length - a.word.length || a.order - b.order)
    .slice(0, WORDS)
    .sort((a, b) => a.order - b.order);
  const name = longest.map(({ word }) => word).join(" ");
  if (name === "") return fallback;
  const capital = name.charAt(0).toLocaleUpperCase() + name.slice(1);
  return capital.slice(0, MAX_LENGTH).trim();
}
