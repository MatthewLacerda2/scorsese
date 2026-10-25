// A project's name from the prompt that starts it (#1535): the three longest
// words, each taken once, that are neither a function word nor one every
// prompt has ("vídeo", "quero", "make"), in the order the prompt has them (the
// earlier of two equally long ones wins), each with a capital, and at most
// `MAX_LENGTH` characters. Numbers and punctuation are never words. A prompt
// with none gives the fallback the caller passes ("Novo vídeo 24/10"). The
// stopwords are the three languages' together, since a prompt need not be in
// the language of the site.

const STOPWORDS = new Set(
  `the a an and or of to for in on with my our your is are was be this that it its at by from as
   about into me i we you they he she video videos clip film movie
   want make create need would like please can could do some new
   o a os as um uma uns umas de do da dos das em no na nos nas para por com sem e ou que meu minha
   meus minhas seu sua seus suas nosso nossa ao aos à às pelo pela como mais sobre entre esse essa
   isso este esta isto é são foi ser vídeo vídeos filme
   quero queria fazer faça faz criar crie preciso gostaria pode novo nova
   el la los las un una unos unas del y mi mis tu tus su sus nuestro nuestra al lo es son fue
   eso esto más quiero quisiera hacer haz crear crea necesito puedes nuevo nueva`.split(/\s+/),
);

/** The most words a name takes. */
const WORDS = 3;
/** The longest a name may be, in characters. */
const MAX_LENGTH = 40;

/** The words of `prompt`: runs of letters; digits, apostrophes and punctuation split them. */
function wordsOf(prompt: string): string[] {
  return (prompt.match(/[\p{L}\p{M}]+/gu) ?? []).filter((word) => word.length > 1);
}

function capitalised(word: string): string {
  return word.charAt(0).toLocaleUpperCase() + word.slice(1);
}

/** `prompt`'s name, or `fallback` when it has no word worth naming a project by. */
export function projectName(prompt: string, fallback: string): string {
  const seen = new Set<string>();
  const candidates = wordsOf(prompt)
    .map((word, order) => ({ word, order, key: word.toLocaleLowerCase() }))
    .filter(({ key }) => !STOPWORDS.has(key) && !seen.has(key) && seen.add(key));
  const longest = [...candidates]
    .sort((a, b) => b.word.length - a.word.length || a.order - b.order)
    .slice(0, WORDS)
    .sort((a, b) => a.order - b.order)
    .map(({ word }) => capitalised(word));
  if (longest.length === 0) return fallback;
  while (longest.length > 1 && longest.join(" ").length > MAX_LENGTH) longest.pop();
  return longest.join(" ").slice(0, MAX_LENGTH);
}
