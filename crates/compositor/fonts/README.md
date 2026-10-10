# The fonts scorsese ships

Forty families, compiled into `scorsese-compositor` with `include_bytes!`. These are the names a `text` asset's `style` can write; a
project may name a font file of its own instead.

A ninth face is compiled in beside them and is **not** in that list, because
nothing names it: Noto Color Emoji, the **fallback**. *The face nobody names*
below is the whole of it.

**The list itself lives in `src/text/font/shipped.rs`**, beside the
`include_bytes!` that make each one real. This file is the provenance: where
each came from, what it weighs, and what its licence asks for.

| name | family | weights | shape | italic |
| --- | --- | --- | --- | --- |
| `inter` — alias `sans` | Inter | 100 – 900 | variable | drawn, separate file |
| `source-serif` — alias `serif` | Source Serif 4 | 200 – 900 | variable | drawn, separate file |
| `liberation-sans` | Liberation Sans | 400, 700 | **drawn** | Italic + BoldItalic |
| `liberation-serif` | Liberation Serif | 400, 700 | **drawn** | Italic + BoldItalic |
| `montserrat` | Montserrat | 100 – 900 | variable | drawn, separate file |
| `lora` | Lora | 400 – 700 | variable | drawn, separate file |
| `playfair-display` | Playfair Display | 400 – 900 | variable | drawn, separate file |
| `jetbrains-mono` | JetBrains Mono | 100 – 800 | variable | drawn, separate file |

The other thirty-two came with #998 — the free faces people see every day, by
the kind of video that reaches for them. Every one sets Portuguese (á à â ã é ê
í ó ô õ ú ü ç and their capitals), checked in the files rather than taken from
Google Fonts' listing, and `tests/text/catalogue.rs` keeps checking it.

| name | family | weights | shape | italic |
| --- | --- | --- | --- | --- |
| `poppins` | Poppins | 100 – 900 by hundreds | **drawn**, 9 files | drawn, 9 files |
| `roboto` | Roboto | 100 – 900 | variable | drawn, separate file |
| `open-sans` | Open Sans | 300 – 800 | variable | drawn, separate file |
| `lato` | Lato | 100 – 900 by hundreds | **drawn**, 9 files | drawn, 9 files |
| `raleway` | Raleway | 100 – 900 | variable | drawn, separate file |
| `dm-sans` | DM Sans | 100 – 1000 | variable | drawn, separate file |
| `work-sans` | Work Sans | 100 – 900 | variable | drawn, separate file |
| `rubik` | Rubik | 300 – 900 | variable | drawn, separate file |
| `merriweather` | Merriweather | 300 – 900 | variable | drawn, separate file |
| `cormorant-garamond` | Cormorant Garamond | 300 – 700 | variable | drawn, separate file |
| `eb-garamond` | EB Garamond | 400 – 800 | variable | drawn, separate file |
| `libre-baskerville` | Libre Baskerville | 400 – 700 | variable | drawn, separate file |
| `dm-serif-display` | DM Serif Display | 400 | **drawn** | drawn, separate file |
| `nunito` | Nunito | 200 – 1000 | variable | drawn, separate file |
| `quicksand` | Quicksand | 300 – 700 | variable | none |
| `fredoka` | Fredoka | 300 – 700 | variable | none |
| `baloo-2` | Baloo 2 | 400 – 800 | variable | none |
| `comfortaa` | Comfortaa | 300 – 700 | variable | none |
| `anton` | Anton | 400 | **drawn** | none |
| `bebas-neue` | Bebas Neue | 400 | **drawn** | none |
| `oswald` | Oswald | 200 – 700 | variable | none |
| `archivo-black` | Archivo Black | 400 | **drawn** | none |
| `bangers` | Bangers | 400 | **drawn** | none |
| `lilita-one` | Lilita One | 400 | **drawn** | none |
| `caveat` | Caveat | 400 – 700 | variable | none |
| `patrick-hand` | Patrick Hand | 400 | **drawn** | none |
| `kalam` | Kalam | 300, 400, 700 | **drawn** | none |
| `gochi-hand` | Gochi Hand | 400 | **drawn** | none |
| `indie-flower` | Indie Flower | 400 | **drawn** | none |
| `permanent-marker` | Permanent Marker | 400 | **drawn** | none |
| `dancing-script` | Dancing Script | 400 – 700 | variable | none |
| `pacifico` | Pacifico | 400 | **drawn** | none |
| `great-vibes` | Great Vibes | 400 | **drawn** | none |

**Poppins and Lato are keyed by their CSS weight**, the number Google Fonts
serves each file as: Thin is `100` and ExtraLight `200`, although the files' own
`usWeightClass` says 250 and 275. A document writes what a stylesheet would.

**Where a family has an italic, it is a real one, and none of them is an
oblique.** Where it has none — most display, handwriting and script faces were
drawn upright only — `italic: true` is refused, never faked. An italic is
a different drawing — a redrawn `a`, `f` and `g` rather than the upright leaned
over — so it is a second set of files keyed by weight exactly like the first.
Inter is the case that proves the point: its `Inter-V.ttf` carries a `slnt` axis
that would produce a perfectly good oblique, and `italic: true` ignores it in
favour of `Inter-Italic.ttf`, where the letters are actually different.

## The face nobody names

**Noto Color Emoji, and it is reached by coverage rather than by name.** A
document that writes `Ship it 🔥` names `sans` like any other caption; the fire
is drawn because Inter has no glyph for `U+1F525` and the next face in the chain
does. There is no `style` field that selects it, none that turns it off, and it
never appears in the list above — a face a document could name is a face a
document would have to know about, and the point of a fallback is that nobody
does.

| file | family | covers | shape |
| --- | --- | --- | --- |
| `Noto-COLRv1.ttf` | Noto Color Emoji | 1,499 codepoints, and the sequences built from them | **COLRv1**, layered vector paints |

**The vector build, not the bitmap one, and that is a decision rather than a
preference.** Upstream ships the same emoji twice: `NotoColorEmoji.ttf` carries
CBDT bitmap strikes at 136 px, and `Noto-COLRv1.ttf` carries COLRv1 paint
graphs — outlines, gradients and compositing modes, resolution-free like every
other glyph in this directory. A title card is where an emoji is *large*, so a
136-pixel strike blown up to fill a quarter of a 4K frame is exactly the failure
this build avoids. It is the smaller file as well: 4,875 KB against 10,423 KB.

The flags are in it. `Noto-COLRv1-noflags.ttf` is 2,922 KB and would have saved
1,953 KB by dropping every regional-indicator pair, which is to say by silently
not drawing 🇧🇷 — the same silent drop the fallback exists to end, moved one step
along. Paying two megabytes not to reintroduce it is the whole trade.

**No skin tone or joined sequence is a special case here.** 👍🏽 is `U+1F44D`
followed by a modifier and 👨‍👩‍👧 is three people joined by zero-width joiners;
both are ligatures the font's own `GSUB` resolves, so they arrive as one glyph
for the same reason `fi` does. What makes that work is not the file, it is that
the whole sequence is shaped against one face — see
`src/text/runs.rs`.

## Variable and drawn, and why the difference is in the code

A **variable** family is one file whose `wght` axis covers a range, so any
weight inside it is a real position on that axis — including ones the designer
never drew, which is what `500` between Regular and Medium means.

A **drawn** family is several files the designer actually drew, and the only
weights it has are the ones in the table. **Liberation is the reason this
distinction exists.** There is no variable build of it anywhere — upstream ships
Regular, Bold, Italic and BoldItalic as four separate files — so a model where a
face is one file could not express it at all. That is why the Arial and Times
look-alikes were unshippable the moment `weight` arrived, and it is what #278
fixed.

**A drawn family refuses a weight it was not drawn at**, naming the ones it has.
`liberation-sans` at `600` is an error, not a quiet 700. Snapping would be the
same silent substitution the variable rules already refuse when a weight falls
off the end of an axis.

## Provenance

Inter from **3.19**, as `Inter Desktop/Inter-V.ttf`:
<https://github.com/rsms/inter>

Source Serif 4 from **4.005**, as `VAR/SourceSerif4Variable-Roman.ttf` on the
`release` branch: <https://github.com/adobe-fonts/source-serif>

Liberation from **liberation-fonts 2.1.5**:
<https://github.com/liberationfonts/liberation-fonts>

Montserrat, Lora, Playfair Display and JetBrains Mono from **google/fonts**,
each the `[wght]` variable file under `ofl/<family>/`:
<https://github.com/google/fonts>

The thirty-two families of #998 from **google/fonts at `bd8f81dd`** (2026-10-09),
each from its own folder — `ofl/<family>/`, and `apache/permanentmarker/` for
Permanent Marker — taking the `[axes]` variable file where Google ships one and
the static files where it does not (Poppins, Lato, Kalam, DM Serif Display and
the single-weight display and handwriting faces). Each licence was read from
that folder's own `OFL.txt` / `LICENSE.txt` and is committed beside the faces as
`<Family>-OFL.txt`, or `PermanentMarker-LICENSE.txt`.

Noto Color Emoji from **noto-emoji v2.051**, as `fonts/Noto-COLRv1.ttf`:
<https://github.com/googlefonts/noto-emoji>

Italics from the same releases: Inter's from `Inter Variable/Single axis/
Inter-italic.ttf`, Source Serif's from `VAR/SourceSerif4Variable-Italic.ttf`,
Liberation's Italic and BoldItalic from the same tarball, and the four Google
families' from `<family>-Italic[wght].ttf` beside their uprights.

All unmodified. **51,577 KB of faces in total**, across 102 files. The first
eight families and the emoji face are 14,189 KB of that (21 files; the emoji face
alone is 4,875 KB, which is what a complete colour set costs), and #998's
thirty-two families are **37,388 KB across 81 files**.

Two families are most of #998's weight, and it is recorded rather than trimmed:

- **Lato, 11,577 KB** — eighteen static files of ~640 KB each. There is no
  variable Lato; every weight and its italic is its own file.
- **Merriweather, 9,003 KB** — two variable files carrying `opsz` and `wdth`
  as well as `wght`, ~4.5 MB each.

The other thirty families are 16,808 KB together.

```
sha256  ae7f78865f4e4c77c50f1ee0fbe603665e48539c3da815c04d99ffec8afc3c6d  Inter-Italic.ttf
sha256  69b1af837d101ab90b003d61d4ccc5e5320a6dcaefeb69906fa31c01a06e5837  Inter-V.ttf
sha256  85ae2a5cd3f56baf1ce1c21a851322c58e3d8fbe8e8ad4a4d090a820dd7fe558  JetBrainsMono-Italic[wght].ttf
sha256  48715a42ec242c21e9f02692891e147d022299a52e48d5e413e1a942193ffeda  JetBrainsMono[wght].ttf
sha256  698da70fc191cc5f33ad4d6d3fe830fe4624b898ea2e3169955928b7c491f1ee  LiberationSans-BoldItalic.ttf
sha256  788abee4c806d660e8aee46689dd8540cd4bb98da03dcc9d171ce3efd99a9173  LiberationSans-Bold.ttf
sha256  e5bae5c4cde31f22142753855f4f8fb86da6ff39955ed3c0a11248b0d16948b0  LiberationSans-Italic.ttf
sha256  76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8  LiberationSans-Regular.ttf
sha256  f17db8af71e24d2066b587546021d4f0b296be389512b658dec3c09affeb11a7  LiberationSerif-BoldItalic.ttf
sha256  d754ba427cfe0bca54ae052384baa8f842da5bd6550ad4da024ac441e7a7d5ce  LiberationSerif-Bold.ttf
sha256  0e3dea9f8d613e006ccfa62201f33e265d19167bd0907725c3e145368b04fc2e  LiberationSerif-Italic.ttf
sha256  058ea80864aef09a23f45cbec2bb5400bc3dfbdea01c3f10538a21fcb497fb74  LiberationSerif-Regular.ttf
sha256  22d8d8854b53807aa664ca34f2031a9ed57a1d0dea296b8b96cdd3aad937a2b3  Lora-Italic[wght].ttf
sha256  822a6621ccbe8d97d20ac88c1c41f5615c9c2c202eaa75f272cd452aac6475a7  Lora[wght].ttf
sha256  51607f316bc020e59f03cbf51543eecffbea501c0b31d73e5b82927c5cca442c  Montserrat-Italic[wght].ttf
sha256  0f7b311b2f3279e4eef9b2f968bcdbab6e28f4daeb1f049f4f278a902bcd82f7  Montserrat[wght].ttf
sha256  0ae57fe58645638523ba35f388d93739d292539a9acb84df5700c81b1e1a28d2  Noto-COLRv1.ttf
sha256  a5e26dc5e2e77fb2803a0bf02fd4f81ee136ec8dea863ccdb0c59a263b21378b  PlayfairDisplay-Italic[wght].ttf
sha256  c40f2293766a503bc70cce9e512ef844a4ccb7cbcde792fe2ea31d191917d8d6  PlayfairDisplay[wght].ttf
sha256  6a059a64838978d54e8fab71ed86b0d82e948c0e12b2664d0c15166326dcff82  SourceSerif4Variable-Italic.ttf
sha256  14d360ee1b76655da9276628b229e11671bc1f5d1083636144db6677d452cf55  SourceSerif4Variable-Roman.ttf
```

#998's faces:

```
sha256  a4ba3a92350ebb031da0cb47630ac49eb265082ca1bc0450442f4a83ab947cab  Anton-Regular.ttf
sha256  dd9a89a019b4849f66ab75455fe7bdf931311042cbb0f0f97acc061539703180  ArchivoBlack-Regular.ttf
sha256  d47a6852548059b1db49a1319d06d499d546c3fa2237cf9eee9c43c8abb025c2  Baloo2[wght].ttf
sha256  4160a7311de9342674cce9160cde9fcbb30f48190397d86ff1b70b455af65824  Bangers-Regular.ttf
sha256  08e4623805102d819f58601e46e345648846075e363b2ceb23313c2d1c83ec73  BebasNeue-Regular.ttf
sha256  0bdb6b660482d31531b3945849fba5916b3ef8695da7024a9e6b9ee3c4157988  Caveat[wght].ttf
sha256  0fc3f45dc48b614db9c39181502544b37217ecbf8bee2fb35886992bc96c5bd3  Comfortaa[wght].ttf
sha256  0f48ea6abb2084537854f7174c470991a463b13036309e3b50a81511611c530d  CormorantGaramond-Italic[wght].ttf
sha256  b20b7d9626dd956b2c5e558692ad328b1f19e3275e2782db4fa07670d83f35e0  CormorantGaramond[wght].ttf
sha256  22259c0cc8237221b80f44c76ba8d36e6bce3cda72779f5b2773643d499720ae  DMSans-Italic[opsz,wght].ttf
sha256  8cd08d97e89c24d0aa92edd2f0f4c8ee6195eee9b7c9f154865a58b02f0c1c0d  DMSans[opsz,wght].ttf
sha256  df74c0ac387baeaeb0fe4f2324e1668e6a3ed8c09cd9796fe162c71753e19e45  DMSerifDisplay-Italic.ttf
sha256  8cc3643535edf039aa5d95440a8542735e9197e4f4b8d9303e980fefbf5ab616  DMSerifDisplay-Regular.ttf
sha256  21808625578fe8d8cd10cb684be546dca077b27cd03a53a2f1ec11dc743c924c  DancingScript[wght].ttf
sha256  bba2c4499c93c9612b90b9825d32b07da52fce2fe57562a1eb6b833553f93c4e  EBGaramond-Italic[wght].ttf
sha256  ef9512f92f6d579e5dc75af59a5a4b1b8b47d2eda89e00b954d44520e5369027  EBGaramond[wght].ttf
sha256  2ba02e68b152868aef9ba28e24b3648c7d457fe6f25c761f2c2c53fb61a73fc8  Fredoka[wdth,wght].ttf
sha256  c46b029ab4846b2935e301af0b2cf85a1d74d2858e6a33636a3e64cf3cc4696b  GochiHand-Regular.ttf
sha256  8d509802186f1b51572531ecf313e8098f9a5bfdfaca93f0c9b34467f9982d15  GreatVibes-Regular.ttf
sha256  ccc94b22b156e9c5dfe50fd051f01b097600b252c24473e624bb43a143140a94  IndieFlower-Regular.ttf
sha256  2f6576601db015d4f6c08678120277fc8510b98c06e932ce7a6a9cbff4cbdded  Kalam-Bold.ttf
sha256  2b8f2c0208d397a1823688783da3317d6af18ea007a9a1b56ddb17e27820507e  Kalam-Light.ttf
sha256  57cecb63d4608019371954274ae1d8c397764debd5b19d4a33c1efa4dc923c0b  Kalam-Regular.ttf
sha256  808c62839c62dbce7de689af7603666fc7f8b81e0df537d8a5212c87580d4337  Lato-Black.ttf
sha256  3214f75bf96783a571bd25338c867c669e7c5e7c08a227f18a7b63766d6e6cbe  Lato-BlackItalic.ttf
sha256  8a0aace75d33794eece4b28187bfc1df0bbd2888b5d8a56e01788c8d65d16be1  Lato-Bold.ttf
sha256  62c1b7f0d2e74b45960154c3520efc337b553db0961bfdc950d5618334596cc8  Lato-BoldItalic.ttf
sha256  61018de62bdaf90d4ac80b1d53c5c130756c8e9219aea3d2773fc9bd5869af97  Lato-ExtraBold.ttf
sha256  a42b384b9c37913a61023e3a47c07fead26e776127bbc9e6a0c96fe413089374  Lato-ExtraBoldItalic.ttf
sha256  989989b481db5df0827d90c51fd08d024fe7bc4e367046e2584a0d3b0487936d  Lato-ExtraLight.ttf
sha256  fc56526250b9b64ce0908c52350b03e91b84afecb08fd2834a12e57b91abdb31  Lato-ExtraLightItalic.ttf
sha256  e399c44efe1387100531d26c7e4800c5d12251b890d6654a3098c7c679cb1786  Lato-Italic.ttf
sha256  cf2a774503baf418d584f49967bd160e1e03f087c13b25602f28024ec7788f08  Lato-Light.ttf
sha256  700e6722fd14a10e7646347a6b14c486d4c364a373ba61c70e0b5ccee7811635  Lato-LightItalic.ttf
sha256  d3ac182a6833e005745dd75679fbad081c0b12535df4e93ad8ed57817a31a338  Lato-Medium.ttf
sha256  ecbce6cffa42d8a67afeb27a9d6e85d515e81481d4b2cece989e833a48048405  Lato-MediumItalic.ttf
sha256  d636e4683231f931eda222d588e944d082bfd3bdba02f928bee461c0f185b251  Lato-Regular.ttf
sha256  71b8b7decbe75a881ed267be539d402bd1e9420b799658aada4e0d1bd5af803c  Lato-SemiBold.ttf
sha256  5adeb6b334d8fb99ef053bc052b952773d544315e0e1d668a12c0ce8bd51375c  Lato-SemiBoldItalic.ttf
sha256  663c55bfe9a38270fff44da273bf4d792eafc3e7e1201f62c3064cad80c5d5e7  Lato-Thin.ttf
sha256  e5e86e219ce6987b175470d94d57753a387d1860b635da6327a60e8d19a33beb  Lato-ThinItalic.ttf
sha256  223959683dc73ec4437bd61fabaa4b3f22209e22855ffd3aee36ba61a5116e97  LibreBaskerville-Italic[wght].ttf
sha256  05a95421961341c5b2556285e8415df9db27dab4f4abe22b446b3c6a8b916c5d  LibreBaskerville[wght].ttf
sha256  f5b641c45c69d772ee4eda687bc9fda411d5cad6b0b45371491da4580cbc8d59  LilitaOne-Regular.ttf
sha256  f68a8f4989258679e4fbaf50aa42400132b5373c2d9d2514ba82ef6e85947a0b  Merriweather-Italic[opsz,wdth,wght].ttf
sha256  d0ed0e359e396af7ad05e73dffd11a3a4c326ea0d0283c56bd9361cb2cc86a96  Merriweather[opsz,wdth,wght].ttf
sha256  b520cc871868b0acfca1beda875df7f4a44ebce914f8a89f83977fc9c09529c8  Nunito-Italic[wght].ttf
sha256  bb55a5ca5c2042335b3991af27c4d0705d0ef41cac6164ac737fd8f2a1e85207  Nunito[wght].ttf
sha256  fe269381e992f32e135801740998544d6235061e37c93ec067ad2be3edd5b17b  OpenSans-Italic[wdth,wght].ttf
sha256  36643644f318a812aab2d2ed3bb98f8cf0872527f835fe9398d95fe6b9adb878  OpenSans[wdth,wght].ttf
sha256  5b38c246e255a12f5712d640d56bcced0472466fc68983d2d0410ec0457c2817  Oswald[wght].ttf
sha256  5b6c0d5334a7bf77dea52b975c5a0c408878c0f7115ed5b6fb151f634b7bf701  Pacifico-Regular.ttf
sha256  0f173b3e6cb6d1af25babf7f0057c5ac4ee11f9992b0469bb817e967ef4ad0fc  PatrickHand-Regular.ttf
sha256  28f82c8a7943cb8e9d599f8554da1d4fc75dbcf69b9885ad6c0611d20c6946c5  PermanentMarker-Regular.ttf
sha256  d82aaaf98a9283f9a8edd24e51173337d8eaf09e25cd3d98831f8ec8461748a1  Poppins-Black.ttf
sha256  f4852ca89c29f69d800e14f097ed4d1f0a0cc454e9f77d73bfd6db1f71c287a0  Poppins-BlackItalic.ttf
sha256  983676516167748b74de6f4771fb384c664fd913acb8b471122ecacf5da5ea6c  Poppins-Bold.ttf
sha256  3572ac8116a0ac7317d342262b29937bcbaf94d8f03f90df6fe666fa7e2fb43a  Poppins-BoldItalic.ttf
sha256  f2ab17c1a63a0ecc12c2461848fc8a469395e3cd2d641803e889c643d9f958e1  Poppins-ExtraBold.ttf
sha256  dc00a2eb988373e9f4e99bf8ff76c6315ee21d36b341cbbff024e4c18cc7ac03  Poppins-ExtraBoldItalic.ttf
sha256  55c03314cc754e26f741f97890e5e9cbe3b3278fe3abcecfecafd60111b2643d  Poppins-ExtraLight.ttf
sha256  7171a17e4cda8f6ded78a7a931f9b7f38987057f3ffd1fe07a71528ad0f01e49  Poppins-ExtraLightItalic.ttf
sha256  4fa76ae75b40f926420514044722cb97f32186cafd3b38263cc34dad7174d46d  Poppins-Italic.ttf
sha256  650ba57fa99d12ec40c31ccfb680be656be4497fbe14164617d67e32ffe9cd46  Poppins-Light.ttf
sha256  b8f9c5be59723fadf8e5447fa1245c2c53b60a3464a24d6ece9ee3c283d8917b  Poppins-LightItalic.ttf
sha256  90373e7d838d32468438fc3e152dca0bdb12edcab99ea639f158790b1ba1fd05  Poppins-Medium.ttf
sha256  765addf6c7c11ec3c54325cacd68cabd05df8e4b6455302d812a9b9bafd1c614  Poppins-MediumItalic.ttf
sha256  7e65201e9b79159e2300267cc885e16c8dcef2424cdfa09a29bfb0980a94a7ba  Poppins-Regular.ttf
sha256  d3bf1bdaf0550e83da9ac0b1d1d9fe6db086835a83aa28578e609a394b9a0286  Poppins-SemiBold.ttf
sha256  16bb118aa232c9a13fa238027d24d7854dd1a1d9cbaf99b17fec4388d56b432c  Poppins-SemiBoldItalic.ttf
sha256  6d8e5d9d29140cc93e321745fa1243c67889e6bc3639ec34db64f3da7a496352  Poppins-Thin.ttf
sha256  46df80ac970f5e84829b868d283878b4d97e289c7b1245541d8bbf66b5d670a8  Poppins-ThinItalic.ttf
sha256  39c9b64223561f56aaff6062a6f04063c4fc86809ad6768722c06614d977e1cc  Quicksand[wght].ttf
sha256  96629caf2202183fab46c70237055a7d67e6a5400b85413d45a77ed6f2a0770c  Raleway-Italic[wght].ttf
sha256  8bbcc3eb8275c388f4bcd998832f8a4b943eadbaf6a595205312774b5951aefb  Raleway[wght].ttf
sha256  9725a847af6b460ffca162ae66d20dad48b01876137947180b42d7dcd7887182  Roboto-Italic[wdth,wght].ttf
sha256  d7598e12c5dbef095ff8272cfc55da0250bd07fbdecbac8a530b9b277872a134  Roboto[wdth,wght].ttf
sha256  08c6c4018a5ada8b517407b46897e46cf6ebb106853fbd3e89addb51d3b59c62  Rubik-Italic[wght].ttf
sha256  1b3a7437ba2af80e465e773ed60c5036d1ba6ace492d89046dbcf18fb31e4e88  Rubik[wght].ttf
sha256  0a9f935ea490d3477fc97e40248f356c29bce11a1973939056c4316b122341ec  WorkSans-Italic[wght].ttf
sha256  f50f61f2ba738e239442d40bf1069adb195c224b6a5a73a581fc2f3ed62a9f63  WorkSans[wght].ttf
```

The two Liberation hashes for Regular are the same bytes this repository
shipped before #267 swapped them out, which is worth noting: they came back
rather than being fetched anew.

## Which axes each file carries, and what is left alone

Only `wght` is read. Every other axis stays where the file's own `fvar` puts it,
so which axes a file has is something to check **before** choosing it.

| file | other axes | left at |
| --- | --- | --- |
| `Inter-V.ttf` | `slnt` −10–0 | `0`, upright |
| `SourceSerif4Variable-Roman.ttf` | `opsz` 8–60 | `20`, the text design |
| `Noto-COLRv1.ttf` | none — no `fvar` at all | — |
| `DMSans[opsz,wght].ttf` and its italic | `opsz` 9–40 | `9`, the caption design |
| `Merriweather[opsz,wdth,wght].ttf` and its italic | `opsz` 18–144, `wdth` 87–112 | `18` and `100` |
| `OpenSans[wdth,wght].ttf` and its italic | `wdth` 75–100 | `100`, normal width |
| `Roboto[wdth,wght].ttf` and its italic | `wdth` 75–100 | `100`, normal width |
| `Fredoka[wdth,wght].ttf` | `wdth` 75–125 | `100`, normal width |
| everything else | none | — |

**DM Sans and Merriweather are the Inter 4 and Source Serif case again**: an
`opsz` axis left at its smallest, so a title set in either is drawn at the text
design. Google ships no build of either without the axis, and the difference at
display size is subtle — recorded here so nobody discovers it later.

**Inter 3.19 rather than 4.x, deliberately.** Inter 4's `InterVariable.ttf`
carries an `opsz` axis running 14–32 defaulting to **14** — a design tuned for
small text — so every title would be set at the caption design with nothing
saying so. 3.19 has no `opsz` at all.

**Source Serif 4's `opsz` sits at 20 and there is no build without it.** A title
at display size is therefore drawn slightly off-design. Recorded here rather
than discovered later. Optical sizing is a real feature with a real rule and is
not this.

One trap worth naming: **Montserrat's own `fvar` default is 100** — Thin — and
it is not alone: Raleway's is 100, Nunito's 200, and Quicksand, Fredoka, Rubik,
Cormorant Garamond and Merriweather sit at 300. It does no harm, because a shipped family with no weight named is set at 400 rather
than at the file's default, which is exactly the rule that exists for it.

## Why they are committed

A system-font lookup resolves to a different file on Linux, macOS and Windows.
Text drawn from one would not match text drawn from another, so a golden
reference blessed on one machine would fail on every other and the pixel gate
would become noise. Deterministic text means a font we ship.

That makes these files part of the pixel gate rather than a detail beneath it,
and [`docs/golden-renders.md`](../../../docs/golden-renders.md) says so on its
own page — the faces sit upstream of the gate the way the decoder does, and
neither may be let go of quietly. The rule that follows lives there with the
other re-blessing rules: **swapping, subsetting or system-resolving a face
re-blesses every fixture that draws text**, and is legitimate only as a
deliberate visual change explained in the pull request. The desktop app's own
reference images are in the same position, because its preview draws a
compositor frame.

Adding a *new* name moves nothing, which is what makes the list cheap to grow:
every existing fixture names `sans`, `serif`, or a font its own project carries.

## Licence

SIL Open Font License 1.1 for all but one — full texts beside the files, exactly
as each shipped. The exception is **Permanent Marker, under the Apache License
2.0** (`PermanentMarker-LICENSE.txt`), which asks the same two things that
matter here: keep the licence with the file, and say so if you change it. Roboto,
once Apache too, is OFL in google/fonts today, and its `Roboto-OFL.txt` is the
file it came with. The first nine: `Inter-OFL.txt`, `SourceSerif4-OFL.txt`,
`Liberation-LICENSE.txt`, `Montserrat-OFL.txt`, `Lora-OFL.txt`,
`PlayfairDisplay-OFL.txt`, `JetBrainsMono-OFL.txt`,
`NotoColorEmoji-OFL.txt`; #998's are `<Family>-OFL.txt` for each, named by the
family without spaces (`DMSerifDisplay-OFL.txt`, `EBGaramond-OFL.txt`).

The two conditions that matter: every file is redistributed **unmodified** and
under its own name, and most of these families carry Reserved Font Names —
`Inter`, `Source`, `Liberation`, `Montserrat`, `Lora`, `Playfair`,
`JetBrains`, and among #998's `Comfortaa`, `Dancing Script`, `Gochi Hand`,
`Lato`, `Libre Baskerville`, `Lilita`, `Merriweather`, `Quicksand`, `Raleway`
and DM Serif Display's `Source` — so a modified copy would have to be renamed. Noto Color Emoji
reserves none, which changes nothing here: it is redistributed whole and
unmodified like the rest. Neither is a
constraint on scorsese's own licence; the OFL covers the font files and nothing
else. **Subsetting counts as modifying**, which is why they are committed whole.

**Arial and Times New Roman themselves can never ship.** They are Monotype's,
licensed through Microsoft, and cannot be committed to a public repository. That
is the entire reason Liberation exists: metric-compatible open substitutes, the
same advance widths, the same look to anyone who is not a typographer. If you
want the Arial look, `liberation-sans` is it.

What is still missing is a **condensed** or a **wide**, which would be a `wdth`
axis and a third field. Nobody has asked, and the same rule would apply: a real
width is a drawing, not a horizontal scale.
