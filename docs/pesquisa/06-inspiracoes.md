# Inspirações de personagem

> Busca de 2026-10-02: 5 frentes de pesquisa, curadoria e verificação de links/licenças. Links conferidos na data.

## Recomendação da curadoria

A escolha deve ser pelo olho, abrindo os GIFs dos três primeiros conceitos. Foi pelo olho que o usuário recusou o ASCII.

Minha aposta principal é a Pochi, da ToffeeCraft. Ela custa US$1,80, ou US$2,40 no Cat Mega Bundle, que também traz o Mochi. Já vem com dança, feliz, susto, choro, dormir, cócegas e caixinha, então o pet fica bonito quase sem desenhar nada. Faltam só 'trabalhando' e 'arrastada', uns 10 frames no estilo dela.

Se ele quiser a cara do Brasil, a melhor rota é a dupla da exclusiveOlive:
- A capivara zen é o pet principal.
- A arara tagarela chama você e comemora voando.
- As duas são da mesma artista, com a mesma paleta, por US$3,50 no Animal Bundle. O bundle ainda traz sapo, gato e raposa como skins extras.
- Desenhamos só a mexerica, uma pose de frente e uma dancinha.

O Caramelo (ChanceKite, US$30) tem o vocabulário mais rico: toca aqui, rolar, latir, uivar, sim e não. Mas ninguém viu a arte ainda, então eu só compraria depois de abrir os GIFs.

Em qualquer caminho:
- O pet funciona com skins (sheet + JSON por estado).
- Os packs pagos ficam numa pasta local fora do git, montada no Docker.
- Se o repo virar público, ele publica só o código e uma skin livre: o Ninja Frog (CC0) ou o Gopher com créditos.
- Os balões '!', '?' e 'Zzz' vêm do Kenney Emotes (CC0), com falas curtas em PT-BR: 'Pronto!', 'Psiu! Preciso de você', 'Deu ruim...'.

Nesta curadoria, eu conferi as páginas (licenças, preços e listas de animação), mas não abri os GIFs. A pesquisa viu de perto o Mochi, as capivaras da exclusiveOlive e da Melloinc e o Gopher. O resto foi avaliado pela descrição. Nas páginas da exclusiveOlive, o GIF 'Rate!!!' é só um pedido de avaliação, não um personagem.

## Conceitos

### 1. Pochi, a gatinha que dança quando o Claude termina

Uma gatinha chibi cabeçuda, de traço macio e paleta pastel, que já vem com todo o vocabulário de um Tamagotchi. Dá para o pet ficar lindo quase sem desenhar nada. Quando o Claude termina, ela fica toda feliz (Happy) ou cai na dança (Dance) com confete. Quando o Claude precisa de você, ela leva um susto (Surprised) e pula com um balão '!'. Enquanto ele trabalha, ela fica concentrada, enfiada na caixinha (Box) ou dando patadinhas no teclado, e quando tudo para ela dorme enroladinha (Sleeping).

- **Personagem:** Gata chibi da ToffeeCraft. Pochi tem 64x64 e 6 pelagens: marrom, branca de olho azul, preta de olho amarelo, cinza de olho verde, laranja de olho verde e cinza-e-branca. Mochi tem 32x32, é creme/laranja e tem laços em 8 cores. A gata laranja de olho verde combina com o laranja do Claude.
- **Estilo:** Kawaii/cozy de Tamagotchi. Cabeça com mais ou menos metade da altura do corpo, olhos grandes e brilhantes, contorno escuro suave (não preto), 3-4 tons pastel por cor, poses claras e curtas. Exibir a Pochi a 2x (128px) ou o Mochi a 4x (128px), sempre em escala inteira com nearest-neighbor.
- **Arte:** Comprar o Cat Mega Bundle da ToffeeCraft (US$2,40+). Ele traz Pochi 64x64, Mochi 32x32, Little Kitties, Cat Room e Cat UI. Se preferir, dá para comprar só a Pochi (US$1,80+). Usar as animações direto, sem redesenhar. Arte nova mínima, no estilo dela: 'trabalhando' (patinhas num notebookzinho, 4-6 frames) e 'arrastada' (pendurada pela nuca, 2-3 frames). Balões e Zzz ficam numa camada separada (Kenney Emotes, CC0, recolorido para a paleta do pack). Plano B da mesma artista: Pet Mobile Pixel Asset Pack (US$1,80+), com gatos que têm Dancing/Surprising e cães que têm Bark.
- **Licença:** Pago: "For commercial or personal use. - It can be used and edit freely. - Not redistribute or resell this assets. - It can be used for game development and other productions." A versão grátis diz "For personal use. ..." e tem só 2 animações. Repo privado: OK com a cópia comprada. Repo público: NÃO commitar as sheets/PNGs, nem editadas, porque isso é redistribuição. A skin deve vir de uma pasta local listada no .gitignore e montada no container. O repo público leva só o código e uma skin CC0; para manter um gato, o LuizMelo Pet Cats é CC0.
- **Cobertura:** Já vem pronto na Pochi:
- idle: Idle/Chilling
- feito-pequeno: Happy
- feito-grande: Dance
- precisa de você: Surprised (+Jump)
- erro: Cry (Hurt; Die/Die2 só como exagero cômico)
- dormir: Sleeping
- clique/carinho: Tickle
- piadinhas: Box1-3 e So full
- solta: final do Jump
O Mochi acrescenta Sleepy→Sleeping, Excited, Eat, Play, Licking, Bath, Box, Waiting e Despise. Esses nomes foram lidos na folha de preview. Falta desenhar: 'trabalhando' dedicado (4-6 frames) e 'arrastada' (2-3 frames), cerca de 10 frames no total.
- **Riscos:** Só o Mochi foi visto em GIF pela pesquisa. A Pochi foi avaliada pela página e pela nota (5,0 estrelas, 44 avaliações), então abra os GIFs antes de comprar. Gato é a escolha mais óbvia, e o usuário já recusou um gatinho (em ASCII); mostre a arte real lado a lado. Não existe animação de 'digitando': use Box como 'modo foco' ou desenhe uma. A licença proíbe redistribuição, então nenhuma sprite pode ir para um GitHub público. A autora ainda está adicionando animações, e os nomes podem mudar.

**Referências:**

- [Cat Pack - Mochi (ToffeeCraft), prévia das animações](https://toffeecraft.itch.io/cat-pack) — [imagem](https://img.itch.zone/aW1nLzE4OTMyMTcwLmdpZg==/original/yQjbzV.gif): A pesquisa viu este GIF: gatinha cabeçuda creme/laranja, contorno suave e paleta pastel que lê bem até em 32px. As poses de dança e de empolgação são exatamente a fofura profissional que faltou no ASCII.
- [Cat Pack - Pochi 64x64 (ToffeeCraft)](https://toffeecraft.itch.io/cat-retro) — [imagem](https://img.itch.zone/aW1nLzIwMjMxNDg1LmdpZg==/original/mVL7QS.gif): Versão maior e mais detalhada, com Dance, Happy, Surprised, Cry, Tickle, Chilling, Sleeping e Box1-3. Tem 5,0 estrelas em 44 avaliações, e a página diz 'No generative AI was used'.
- [Mochi, atualização 'licking + despise face' (devlog)](https://toffeecraft.itch.io/cat-pack) — [imagem](https://img.itch.zone/aW1nLzIxOTU4NzIwLmdpZg==/original/bo6BCp.gif): GIF do devlog com a lambidinha (boa para variar o idle) e a cara de desdém, que vira um estado de 'erro' engraçado.

### 2. Capi, a capivara zen da mexerica

A capivara mais brasileira possível: gordinha, de olhos fechados de quem está em paz, com uma mexerica na cabeça. É a ideia do primeiro rascunho, só que desenhada por quem sabe. Quando o Claude termina, ela dá pulinhos e os filhotes sobem nas costas, ou faz a 'Party' do pack gratuito. Quando ele precisa de você, a mexerica cai e ela sai correndo assustada com um '!' (ou a arara ajudante vem gritar). Enquanto o Claude trabalha, ela mastiga tranquila, e quando tudo para ela deita e cochila (Sleep→Awake).

- **Personagem:** Capivara chibi 32x32 da exclusiveOlive (2 variações de cor + filhotes), com uma mexerica com folhinha como acessório. Opcional: a arara do conceito 3 como ajudante, que é da mesma artista.
- **Estilo:** Cozy de fazendinha, na pegada de Stardew. Corpo em formato de feijão, contorno 1px marrom-escuro, laranja-caramelo quente com brilho amarelo, olhos fechados de quem está 'de boa'. O humor vem do contraste: a capivara não se abala, quem faz barulho é o mundo em volta (mexerica, filhotes, arara). Exibir a 4x (128px).
- **Arte:** Escolher UMA artista como base. Misturar capivaras de artistas diferentes no mesmo personagem fica feio. Recomendado: Capybaras! da exclusiveOlive (US$1 em promoção). Outra opção é o Animal Bundle de US$3,50, que traz capivara, arara, sapo, gato, raposa, porquinho-da-índia e gaivota no mesmo estilo. Desenhar no estilo dela:
- a mexerica com folha (sprite de ~8x8 em camada própria, com uma rampa laranja nova);
- uma pose de frente 'olhando pra você' (3-4 frames);
- uma dancinha de comemoração (6-8 frames).
Alternativas:
- Prototipar de graça com a capivara do Kalengo.
- Usar a capivara de frente com mexerica da Melloinc (US$4). Mas são loops curtos de cursor e faltam comemorar, dormir e 'solta'.
- Usar a capivara da Tine (2tin.itch.io/capybara, US$5): 24 animações de mastigar, sentar, deitar e dormir, e a autora promete uma 'PET EXPANSION' com vista de frente, danças e emotes.
- **Licença:** exclusiveOlive: "These assets can be used in both commercial and non-commercial projects. These assets can be edited. These assets cannot be redistributed or resold, even if the assets have been edited. Credit is appreciated, but it is not necessary :)". Kalengo Phiri: "commercial use allowed in shipped games and prototypes, modification allowed, no redistribution / resale / re-upload, no training AI on the files, no NFT / crypto use." Melloinc: "You may use this asset for personal and commercial purposes. Feel free to modify it to your needs. ... Please DO NOT resell or redistribute this item." the14collective: "You may not repackage, redistribute or resell the assets, no matter how much they are modified." Repo privado: OK. Repo público: nenhuma dessas sprites pode entrar, nem editada; elas ficam numa pasta local fora do git.
- **Cobertura:** Já vem pronto na exclusiveOlive:
- idle: Idle (+ filhote Idle)
- trabalhando: Bite (mastigando) ou Walk Forwards/Backwards (andando de um lado pro outro)
- feito-pequeno: Jump(Up)→Jump(Down)→Land
- erro: Sad(Start)→Sad(Loop)→Sad(End) (Hurt para falha grave)
- dormir: Sleep(Start)→Sleep→Awake
- arrastada: frame de Jump(Down) segurado, com as patas penduradas
- solta: Land
Falta: feito-grande (dança + filhotes + mexerica, cerca de 8 frames), o 'precisa de você' de frente (cerca de 4 frames) e a própria mexerica. O 'chamar' também pode ficar com a arara. O Kalengo já tem feito-grande (Party), feliz, triste e dormir prontos, com vista de frente.
- **Riscos:** Capivara é calma por natureza. Para chamar atenção, ela depende de balão/som, de frames novos ou da arara. Os packs da exclusiveOlive e da the14collective são de perfil; só Kalengo e Melloinc têm vista de frente. O usuário viu, e achou feia, uma capivara com laranja em ASCII; os GIFs reais precisam provar a diferença. Fora a Party do Kalengo, nenhuma capivara pronta tem dança de comemoração. O pack do Kalengo está 'In development', e a licença dele fala em 'games'.

**Referências:**

- [Capybaras! - Pixel Art Asset Pack (exclusiveOlive)](https://exclusiveolive.itch.io/capybaras-pixel-art-asset-pack) — [imagem](https://img.itch.zone/aW1nLzE5OTA0NTg2LmdpZg==/original/esI%2Bs4.gif): A pesquisa viu este GIF: capivaras gordinhas, contorno escuro limpo, sombreamento quente e filhotes montados nas costas. São 16 animações com início/loop/fim (Sleep, Sad), e é isso que faz o bicho parecer vivo.
- [Custom Capybara Cursor - 32px (Melloinc)](https://melloinc.itch.io/custom-capybara-cursor-32px) — [imagem](https://raw.githubusercontent.com/mellotadaa/cursorTRex/refs/heads/main/cursorCapybara_allScreenshot.gif): A ideia original (capivara com mexerica) feita com capricho: de frente, chibi, olhos fechados de paz, com poses de '?', lápis, joinha e banho de ofurô. Prova que o conceito fica lindo.
- [Cute Capybara (Kalengo Phiri)](https://kalengophiri.itch.io/cute-capybara) — [imagem](https://img.itch.zone/aW1hZ2UvMjk4NTc3MC8xNzg1ODg5OS5naWY=/original/nA3Up1.gif): Grátis e já tem Party, Happy, Sad, Sleep e andar em 4 direções, inclusive de frente. Cobre comemorar, erro e dormir sem desenhar nada; ótimo para prototipar.
- [Pixel Capibaras - 8bit Character Sprites (the14collective)](https://14collective.itch.io/8-bit-capibaras) — [imagem](https://img.itch.zone/aW1nLzIxNDA5NDE4LmdpZg==/original/knV4yS.gif): Já vem com a variante 'capivara com laranja na cabeça' (22 sprites, 11 animações).

### 3. Arara Tagarela, a fofoqueira que te chama

Uma arara gordinha, de olhos enormes e cores berrantes: o alarme mais natural que existe. Quando o Claude precisa de você, ela grita (Chirp) e voa até o canto do terminal com um balão 'Ô! Preciso de você!'. Quando a tarefa termina, ela decola, plana pela tela e pousa de volta (Take Off→Glide→Landing). Enquanto o Claude trabalha, ela fica bicando sementes (Eating), e cochila com Sleep(Start)→Sleep→Awake.

- **Personagem:** Arara/papagaio chibi 48x48 da exclusiveOlive (3 variações de cor). Pode ser o pet sozinha ou a ajudante da capivara do conceito 2.
- **Estilo:** Mesma linha cozy das capivaras, porque é a mesma artista: corpo redondo, olhos brancos enormes, vermelho/azul/amarelo/verde saturados com contorno escuro. Salta em qualquer wallpaper, claro ou escuro. Exibir a 3x (144px). Pássaros do Pantanal podem fazer ponta na comemoração.
- **Arte:** Comprar o Cute Parrots! (US$0,50 em promoção, ou dentro do Animal Bundle de US$3,50 junto com a capivara) e usar direto. Arte nova só para o balão de fala em PT-BR e, se quiser, um loop de comemoração montado com Dive(Start/Loop/End). Alternativas mais barulhentas: o Parrotpack da SeethingSwarm (US$11,99) ou a arara do Elthen (US$1). São de outros artistas, então não dá para usar como ajudante da capivara da exclusiveOlive.
- **Licença:** exclusiveOlive: "These assets can be used in both commercial and non-commercial projects. These assets can be edited. These assets cannot be redistributed or resold, even if the assets have been edited. Credit is appreciated, but it is not necessary." SeethingSwarm: "Do not resell or give away the assets individually. These assets are not to be openly distributed." (e proíbe blockchain/NFT). Elthen: "Feel free to use the sprites in commercial/non-commercial projects!"; os termos completos ficam num post do Patreon que não abriu (403). Lara: "Feel free to use it where you need to!". Repo privado: OK. Repo público: NÃO para exclusiveOlive e SeethingSwarm; para Elthen e Lara, perguntar ao autor.
- **Cobertura:** Tudo já vem pronto no pack da exclusiveOlive:
- idle: Idle/Sit Idle
- trabalhando: Eating ou Walk
- feito-pequeno: Chirp alegre + pulinho
- feito-grande: Take Off→Fly/Glide pela tela→Landing (ou Dive Start/Loop/End)
- precisa de você: Chirp em loop + voo até o terminal
- erro: Hurt
- dormir: Sleep(Start)→Sleep→Awake
- arrastada: Fly, batendo as asas
- solta: Landing
Não falta nada essencial.
- **Riscos:** A ave é de perfil, e voar pela tela exige programar a trajetória (é fácil, mas é código). Grito repetido irrita, então cooldown e 'não perturbe' são obrigatórios. A página não diz quais são as 3 cores; a pesquisa descreveu arara-canindé, arara-vermelha e papagaio verde, mas confirme no GIF. A arte foi avaliada pela página e pela descrição da pesquisa; eu não vi o GIF.

**Referências:**

- [Cute Parrots! - Pixel Art Asset Pack (exclusiveOlive)](https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack) — [imagem](https://img.itch.zone/aW1nLzI3NjU5NjAyLmdpZg==/original/4ELIRq.gif): GIF de demonstração do pack (21 animações: Chirp, Take Off, Fly, Glide, Dive, Sleep...), na mesma linha visual das capivaras da artista.
- [2D Pixel Art Parrot Sprites (Elthen)](https://elthen.itch.io/2d-pixel-art-parrot-sprites) — [imagem](https://img.itch.zone/aW1nLzM0MTgwMTMuZ2lm/original/gDauF%2B.gif): Arara-vermelha 32x32 com uma animação de grito (Squawk) dedicada. Alternativa de US$1.
- [Parrotpack (SeethingSwarm)](https://seethingswarm.itch.io/parrotpack) — [imagem](https://img.itch.zone/aW1nLzEzMTEwODc4LnBuZw==/original/ViS5SM.png): Arara-vermelha com loops Idle_caw e Sit_caw, o grito mais chamativo que a pesquisa encontrou, além de voo planado (Soar).
- [Jungle Birds - Pantanal (Lara / chariart)](https://chariart.itch.io/jungle-birds) — [imagem](https://img.itch.zone/aW1nLzk5MTcyNTkuZ2lm/original/QGZttR.gif): Tuiuiú (e também arara-azul, tucano, socó e ema) grátis, para fazer ponta na hora de comemorar. Só têm idle de 2 frames.

### 4. Caramelo, o vira-lata do 'toca aqui'

O vira-lata caramelo, símbolo nacional não oficial, como cachorro de mesa e com o vocabulário mais rico que encontramos. Quando o Claude termina, ele dá 'toca aqui' (HiFive), rola no chão e corre atrás do rabo. Quando o Claude precisa de você, ele late (Bark), senta te encarando (Wait) e uiva (Howl) se for ignorado. Enquanto o Claude trabalha, ele cava (Dig) e fareja (Sniff), e para os pedidos de permissão tem até as animações 'Yes' e 'No'.

- **Personagem:** Golden retriever top-down do 'Silly Dog Retriever' da ChanceKite. Vem em 6 pelagens (White, Gold, Cream, Chestnut, Choc, Black); a ideia é recolorir a Gold para um caramelo de vira-lata.
- **Estilo:** Pixel art de RPG top-down muito bem animada (2-8 frames por ação), com vistas de frente, costas e lado. O cão é pequeno (11x26px de frente, 23x19px de lado) e seria exibido a 4-5x. Caramelo com barriga creme, e a coleira serve de região de cor por estado.
- **Arte:** Comprar o Silly Dog Retriever (US$30) e trocar a paleta Gold por caramelo. Dá para automatizar a troca de paleta em todas as folhas. Arte nova só para os balões. Antes de gastar, prototipar de graça com os Pixel Dogs do Benvictus.
- **Licença:** ChanceKite: "Upon purchase, this artwork can be used in personal and commercial games. Credit is appreciated. Do not sell or redistribute any part of this artwork." Benvictus não tem texto formal: "Yes, feel free to use this commercially. I would ask that you put me in the credits if you are unable to donate any type of contribution." Repo privado: OK. Repo público: NÃO para ChanceKite; Benvictus é incerto, perguntar.
- **Cobertura:** Quase tudo já vem pronto:
- idle: Stand/Sit/Look-Around/Scratch/Lick
- trabalhando: Dig, Sniff, Pick-Item
- feito-pequeno: Yes ou um TailChase curto
- feito-grande: HiFive→RollOver→TailChase
- precisa de você: Bark→Wait→Howl (a escada de atenção já vem pronta)
- erro: Sigh/No (Scared/Stun para falha grave)
- dormir: Yawn→Laze/Lay→Sleep (Tired antes)
- arrastado: Petted ou BellyUp
- solto: FurShake
Extras: Yes/No para permissões, AirPaw, Eat. Praticamente nada a desenhar.
- **Riscos:** Achei este pack durante a curadoria, e ninguém viu os GIFs ainda; abra antes de comprar. É o pack mais caro (US$30). O sprite é pequeno e de RPG top-down, então pode parecer miúdo na tela; teste a 4-5x. A licença fala em 'games': um pet pessoal é uso razoável, mas não está escrito, então pergunte ao autor se for publicar. Um retriever recolorido não é exatamente um vira-lata (o pelo é mais longo).

**Referências:**

- [Silly Dog Retriever (ChanceKite)](https://chancekite.itch.io/sillydog) — [imagem](https://img.itch.zone/aW1hZ2UvMzI4MTEzNi8xOTU4MTA0MS5naWY=/original/pc%2F5QD.gif): 37 animações em 6 pelagens: HiFive, RollOver, TailChase, Bark, Howl, Wait, Yes, No, Petted, BellyUp, Yawn, Sleep, Dig, Sniff, FurShake e outras. É o vocabulário de pet mais completo que encontramos.
- [Pixel Dog Pomeranian 'Orange' (ChanceKite)](https://chancekite.itch.io/pixeldog03) — [imagem](https://img.itch.zone/aW1hZ2UvMTU0NDI2NS8xMzkxMzMyNy5naWY=/original/96%2BJS6.gif): O mesmo artista em vista lateral 40x32, com hi-five, beg, roll-over, fur-shake e howl. Bom para medir o nível de animação dele antes de gastar US$30.
- [Pixel Dogs (Benvictus)](https://benvictus.itch.io/pixel-dogs) — [imagem](https://img.itch.zone/aW1nLzE2Nzk4MjI4LmdpZg==/original/IDq5Fc.gif): Grátis, com 12 pelagens (dá para achar um caramelo), 64x48. Tem deitado, sentado e em pé, Beg, Sleep, latido e bolinha, com 4,8 estrelas. Serve de protótipo grátis.

### 5. Sapinho caça-bugs

Um sapinho fofo que mora pendurado na borda da janela, no clima do Ropuka's Idle Island. Quando o Claude precisa de você, ele leva um susto (Surprised) e pula com um '!'. Quando a tarefa termina, ele desce planando numa folha e aterrissa. Enquanto o Claude trabalha, ele 'caça bugs' com botes rápidos e cochila com Sleep(Start)→Sleep→Awake. Ainda tem a versão Ninja Frog, CC0, que pode ir para o repositório público.

- **Personagem:** Para uso privado, o sapo chibi 48x48 da exclusiveOlive (4 cores + filhotes, com planador de folha). Como skin pública, o Ninja Frog 32x32 da Pixel Frog (CC0).
- **Estilo:** Cozy de desktop: um bichinho calmo fazendo sua tarefinha num canto da tela, em cima de uma 'ilhota' (sombra/base). O sapo da exclusiveOlive é redondo, de olhos grandes e contorno escuro. O Ninja Frog é mais saturado, tem cara de jogo de plataforma e roda a 20 FPS.
- **Arte:** Privado: comprar o Frogs! (US$0,50, ou dentro do Animal Bundle). Público: usar o Ninja Frog (CC0) e desenhar no estilo da Pixel Frog um 'dormindo' (olhos fechados, 2-4 frames) e um 'chamando' (aceno, 4 frames). Balões do Kenney Emotes (CC0).
- **Licença:** exclusiveOlive: "These assets can be used in both commercial and non-commercial projects. These assets can be edited. These assets cannot be redistributed or resold, even if the assets have been edited. Credit is appreciated, but it is not necessary." Privado OK, público não. Pixel Adventure 1: "Creative Commons Zero (CC0) license. You can distribute, remix, adapt, and build upon the material in any medium or format, even for commercial purposes. Attribution is not required." Privado e público OK. O espelho no OpenGameArt diz CC-BY 4.0, então por segurança dê crédito à Pixel Frog. Ropuka's Idle Island é um jogo comercial: só inspiração.
- **Cobertura:** Sapo da exclusiveOlive (22 animações):
- idle: Idle/Idle 2/Idle 3/Sit(Idle)
- trabalhando: Attack/Grapple em loop ('caçando bugs')
- feito-pequeno: Jump→Landing
- feito-grande: planador de folha + Landing
- precisa de você: Surprised + pulo + '!'
- erro: Hurt
- dormir: Sleep(Start)→Sleep→Awake
- arrastado: Grab Ledge (pendurado)
- solto: Fall→Landing
Ninja Frog: idle, run, jump, double jump, wall jump, fall, hit e o 'poof' de aparecer/sumir. Não tem dormir nem chamar.
- **Riscos:** Ninguém viu o GIF do sapo da exclusiveOlive. Cuidado: o GIF 'Rate!!!' que aparece em várias páginas da artista não é o sapo. Não está confirmado se Grapple/Attack é a língua, nem quantos frames tem o planador de folha. O Ninja Frog tem cara de jogo de plataforma, menos de pet. As duas skins têm estilos diferentes e não podem ser misturadas no mesmo personagem.

**Referências:**

- [Frogs! - Pixel Art Asset Pack (exclusiveOlive)](https://exclusiveolive.itch.io/frogs-pixel-art-asset-pack) — [imagem](https://media3.giphy.com/media/v1.Y2lkPTc5MGI3NjExOXNzNnZ4cXhuazV3b2loNDBvZ25xcXFibDNleTA4YnZsMWp2Y3l5cSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/R5IBTL2VKcdbLWVhjN/giphy.gif): GIF que está na descrição da página. São 22 animações (Idle 1-3, Surprised, Sleep/Awake, Jump/Fall/Landing, Grab/Climb Ledge, Grapple), mais filhotes e planador de folha. É da mesma artista da capivara e da arara.
- [Pixel Adventure 1, Ninja Frog (Pixel Frog)](https://pixelfrog-assets.itch.io/pixel-adventure-1) — [imagem](https://img.itch.zone/aW1nLzI1Mzc4MzcuZ2lm/original/OArrbk.gif): Um dos packs grátis mais usados do itch: personagens 32x32 saturados e quicantes, a 20 FPS, com um 'poof' de aparecer e sumir. É CC0, então pode ir para um repo público.
- [Ropuka's Idle Island (Steam)](https://store.steampowered.com/app/3416070/Ropukas_Idle_Island/) — [imagem](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/3416070/header.jpg?t=1789671909): Só inspiração de clima: um sapinho que corta grama numa ilhota em cima do desktop, com 95% de 2.384 análises positivas. Mostra o valor de ter uma base embaixo do bicho.

### 6. Gopher, o mascote dev que revira os olhos

O gopher do Go em pixel art profissional do Egon Elbre: olhos enormes, pálpebras expressivas e timing de desenho animado. Quando o Claude termina, ele cai na dança ou faz 'party'. Quando dá erro, ele revira os olhos de um jeito hilário. Enquanto o Claude trabalha, ele toma seu cafezinho ou vira ninja. É a única arte desse nível que pode ir para um repo público só com créditos.

- **Personagem:** Go Gopher (design de Renee French) em pixel art 32x32 do Egon Elbre, exportado a 3x.
- **Estilo:** Mascote dev em pixel: contorno azul-escuro de 1px, 2-3 tons, olhos brilhantes com reflexo. Os arquivos .ase e o palette.ase estão incluídos, então a fonte da verdade já vem pronta.
- **Arte:** Usar direto (é grátis). A partir dos .ase, desenhar dormir (3-4 frames), um aceno de 'precisa de você' (4), arrastado (2-3) e solto (3). Também é preciso padronizar canvas e âncora, porque as animações têm tamanhos diferentes.
- **Licença:** Arte do repositório: "The images and art-work in this repository are under CC0 license." O design do Gopher é CC BY 4.0 de Renee French (README: "The Go gopher was designed by the awesome Renee French."; go.dev: "The design is licensed under the Creative Commons 4.0 Attributions license."). Privado e público: OK, com o crédito "Go Gopher by Renee French (CC BY 4.0); pixel art by Egon Elbre (CC0)". Não usar o logo nem o nome do Go.
- **Cobertura:** Já vem pronto: dance (longa), party, aww, eyeroll, ninja, morning-coffee/morning-beer e run/walk 2-bit. Mapa:
- idle: morning-coffee
- trabalhando: ninja ou run
- feito-pequeno: party
- feito-grande: dance
- precisa de você: aww (+ aceno a desenhar)
- erro: eyeroll
Falta: dormir, arrastado, solto e aceno, cerca de 12-15 frames.
- **Riscos:** A identidade é emprestada do Go; se o usuário não programa em Go, o mascote pode não pegar. As animações são ilustrações soltas (a dança, por exemplo, está num canvas de 256x256), não uma folha de pet uniforme. Ele não lembra nem o Claude nem o Brasil.

**Referências:**

- [Gopher dançando (Egon Elbre)](https://github.com/egonelbre/gophers) — [imagem](https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/animation/gopher-dance-long-3x.gif): A pesquisa viu: cabeça redonda, olhos enormes e brilhantes, contorno azul-escuro de 1px. A dança longa é uma comemoração grande pronta.
- [Gopher party (Egon Elbre)](https://github.com/egonelbre/gophers) — [imagem](https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/animation/party.gif): Comemoração curta, pronta para o 'feito-pequeno'.
- [Gopher eyeroll (Egon Elbre)](https://github.com/egonelbre/gophers) — [imagem](https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/animation/eyeroll-3x.gif): O revirar de olhos mais engraçado da pesquisa, um estado de erro perfeito.
- [Gopher morning-coffee (Egon Elbre)](https://github.com/egonelbre/gophers) — [imagem](https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/animation/morning-coffee-3x.gif): Cafezinho em loop para o idle ou o 'trabalhando'.

### 7. Tatu-bola, o pet que vira bolinha

O bicho brasileiro com a piada física mais engraçada para um pet de mesa, e que já foi mascote de Copa: quando você arrasta, ele vira bolinha na hora (Curl). Quando o Claude termina, ele se enrola, rola pela borda da tela e desenrola com um 'tcharã'. Quando o Claude precisa de você, ele fica espiando para fora da bola. Enquanto o Claude trabalha, ele anda de um lado para o outro, e para dormir é só ficar enroladinho, respirando.

- **Personagem:** O tatu 32x32 do Elthen, recolorido para o amarelo-acastanhado do tatu-bola.
- **Estilo:** Criaturinha redonda e bem sombreada do Elthen, em 32x32. A forma de bola permite girar o sprite inteiro para rolar. Squash & stretch forte no quique, seguindo os tutoriais do saint11, o pixel artist brasileiro de Celeste.
- **Arte:** Usar o pack grátis do Elthen e desenhar o resto no estilo dele:
- comemorar: enrolar→rolar→desenrolar, 6-8 frames + rotação no código;
- chamar: espiar para fora da bola, 4 frames;
- dormir: bola respirando, 2 frames;
- solto: quique + desenrolar, 3-4 frames.
Recolorir de cinza para amarelo-acastanhado. Para uma piada parecida, há o jabuti da FUJIIDEW (US$1).
- **Licença:** Elthen: "Feel free to use the sprites in commercial/non-commercial projects! If you do, please consider tipping, or at least dropping a comment down below." Os termos completos ficam num post do Patreon que não abriu (403). A pesquisa registrou nas páginas do Elthen um banner dizendo 'NOT FOR BLOCKCHAIN/CRYPTO RELATED PROJECT USE. READ THE LICENSE.'. FUJIIDEW proíbe: "Uploading the asset to other sites, Reselling this asset, Redistributing outside of projects, Editing the asset and then sharing it". Privado: OK. Público: Elthen é incerto, perguntar; FUJIIDEW não pode. O Fuleco, mascote da Copa de 2014, é marca da FIFA: serve só de inspiração, sem nome nem visual.
- **Cobertura:** Já vem pronto: Idle, Movement, Curl, Damage e Death. Mapa: idle = Idle; trabalhando = Movement; arrastado = Curl (perfeito); erro = Damage. Falta quase toda a parte emocional: comemorar, chamar, dormir e solto, cerca de 15-20 frames novos.
- **Riscos:** O pack é simples (5 animações), então a graça depende de animação nova. Ninguém avaliou de perto quão fofo o GIF é. O tatu cinza precisa ser recolorido. A licença do Elthen está incompleta, porque o post do Patreon não abriu.

**Referências:**

- [2D Pixel Art Armadillo Sprites (Elthen)](https://elthen.itch.io/2d-pixel-art-armadillo-sprites) — [imagem](https://img.itch.zone/aW1nLzYxNDMxMzQuZ2lm/original/1OdTCZ.gif): Tem Curl (vira bola), Idle, Movement, Damage e Death em 32x32, e é grátis.
- [8-bit Tortoise Sprite (FUJIIDEW)](https://fujiidew.itch.io/8-bit-tortoise-sprite) — [imagem](https://img.itch.zone/aW1nLzgyMjY3NzcuZ2lm/original/%2BXZCCZ.gif): Outra piada física brasileira: um jabuti que fica preso de casco para cima (erro), se esconde no casco (dormir) e leva susto (chamar).
- [Pixel art tutorials, Squash (saint11)](https://saint11.art/blog/pixel-art-tutorials/) — [imagem](https://saint11.art/img/pixel-tutorials/Squash.gif): Guia do Pedro Medeiros (Celeste) para quique, rolagem e squash & stretch. Serve só como referência de técnica; a página não declara licença.

### 8. Caranguejinho laranja, o primo legal do Clawd

Um caranguejinho laranja original, que conversa com a identidade do Claude Code sem copiar o Clawd. Ele comemora com uma dancinha de lado e confete, acena as duas garras com um '!' quando o Claude precisa de você e 'digita' com as garras num notebookzinho enquanto o Claude trabalha. Quando dá erro, ele cai de costas balançando as perninhas, e dorme recolhido na carapaça.

- **Personagem:** Caranguejo original inspirado no Ferris (CC0), desenhado em pixel art de 32-48px.
- **Estilo:** Pixel kawaii com props por estado: notebook, malabares para subagentes, vassoura para a compactação, fogo 'this is fine' no erro. Olhos grandes com brilho e sorriso largo. Usar o laranja-avermelhado do Ferris; evitar o salmão (~#DA7758) e o corpo-bloco de dois olhos do Clawd.
- **Arte:** Encomendar a um pixel artist (ou desenhar) um sprite original e cerca de 9 animações, seguindo as regras de estilo. Não existe pack pronto de qualidade: os sprites 8-bit de fãs do Ferris (fórum do Rust, 2019) são toscos e não têm licença.
- **Licença:** Ferris: "To the extent possible under law, Karen Rustad Tölva has waived all copyright and related or neighboring rights to Ferris the Rustacean." Uma arte própria derivada pode ir para um repo público. Clawd on Desk: "Artwork and bundled theme assets (including assets/ and themes/*/assets/) are NOT covered by AGPL-3.0. All rights reserved by their respective copyright holders." e "Clawd character is the property of Anthropic". ClawdMoji: "no rights to the character or mark are granted". Os dois servem só de inspiração. Segundo a pesquisa, a Anthropic pediu ao projeto 'Clawdbot' que mudasse de nome em jan/2026, então evite nome e visual parecidos.
- **Cobertura:** Nada vem pronto. Tudo é arte nova: cerca de 60-90 frames para idle, trabalhando, feito-pequeno, feito-grande, chamar, erro, dormir, arrastado e solto.
- **Riscos:** É o caminho de maior custo e prazo, e a qualidade depende 100% do artista. Há risco de ficar com cara de 'Clawd genérico' ou de arte de programador, que é justamente o que o usuário rejeitou. Só vale se a identidade 'Claude' for a prioridade.

**Referências:**

- [Cuddly Ferris (rustacean.net)](https://www.rustacean.net/) — [imagem](https://www.rustacean.net/assets/cuddlyferris.png): Base CC0: olhos enormes, sorriso largo e laranja-avermelhado. Ainda não existe um Ferris em pixel art de qualidade, então o nosso seria único.
- [Clawd on Desk, clawd-juggling (só inspiração)](https://github.com/rullerzhou-afk/clawd-on-desk) — [imagem](https://raw.githubusercontent.com/rullerzhou-afk/clawd-on-desk/main/assets/gif/clawd-juggling.gif): O pet de desktop mais parecido com este projeto (6,4 mil estrelas). Mostra como props explicam o estado (malabares = subagentes). A arte é 'All rights reserved' e o Clawd pertence à Anthropic.
- [ClawdMoji, fogo 'this is fine' (só inspiração)](https://github.com/afspies/ClawdMoji) — [imagem](https://raw.githubusercontent.com/afspies/ClawdMoji/main/emoji/fire/clawd_fire.gif): Ideia de piada para o erro: o bichinho tranquilo no meio do fogo.

## Packs prontos

### [Cat Pack - Pochi (64x64), ToffeeCraft](https://toffeecraft.itch.io/cat-retro)

- Prévia: https://img.itch.zone/aW1nLzIwMjMxNDg1LmdpZg==/original/mVL7QS.gif
- Licença: Paid: "For commercial or personal use. - It can be used and edit freely. - Not redistribute or resell this assets. - It can be used for game development and other productions." Free: "For personal use. - It can be used and edit freely. - Not redistribute or resell this assets. - It can be used for game development and other productions."
- Preço: Amostra grátis (só uso pessoal); completo por US$1,80+. Também vem no Cat Mega Bundle (US$2,40+, junto com o Mochi). Tem 5,0 estrelas em 44 avaliações e a página diz 'No generative AI was used'.
- Animações: Idle, Run, Jump, Attack, Hurt, Die, Die2, Sleeping, Happy, Cry, Tickle, Chilling, Dance, Surprised, So full, Box1, Box2, Box3, 6 pelagens (marrom, branca/olho azul, preta/olho amarelo, cinza/olho verde, laranja/olho verde, cinza-e-branca)
- Mapa para estados: - idle: Idle/Chilling (com Box1-3 de vez em quando)
- working: Box ('modo foco') ou Attack curtinho em loop ('patadas no teclado'); o ideal é desenhar um 'digitando' de 4-6 frames
- done-small: Happy
- done-big: Dance + confete
- needs-attention: Surprised → Jump + balão '!'
- error: Cry (Hurt para falha; Die/Die2 só como exagero cômico em build quebrado)
- sleep: Sleeping
- dragged: Tickle (ou um frame de Hurt segurado)
- dropped: final do Jump (aterrissagem) ou So full

### [Cat Pack - Mochi (32x32), ToffeeCraft](https://toffeecraft.itch.io/cat-pack)

- Prévia: https://img.itch.zone/aW1nLzE4OTMyMTcwLmdpZg==/original/yQjbzV.gif
- Licença: Paid: "For commercial or personal use. - It can be used and edit freely. - Not redistribute or resell this assets. - It can be used for game development and other productions." Free: "For personal use. - It can be used and edit freely. - Not redistribute or resell this assets. - It can be used for game development and other productions."
- Preço: Grátis (2 animações, uso pessoal); completo por US$1,90+ (Cat Room por US$1+). Cat Mega Bundle por US$2,40+.
- Animações: 23 animações (contagem do Cat Mega Bundle), Nomes na folha de preview, segundo a pesquisa: Sleepy, Idle, Idle 2, Sleeping, Dance, Excited, Eat, Play, Bath, Licking, Despise, Box, Waiting, Banheira em 6 cores, Laços em 8 cores, Versão Halloween, Ainda sem andar/correr/pular (a autora planeja adicionar)
- Mapa para estados: - idle: Idle/Idle 2/Licking/Bath, sorteados
- working: Play, Eat ou Waiting em loop
- done-small: Excited
- done-big: Dance
- needs-attention: Excited/Waiting + balão '!' (dá para trocar o laço para amarelo)
- error: Despise
- sleep: Sleepy → Sleeping
- dragged: não tem; segurar um frame de Excited (recomendado desenhar 2-3 frames)
- dropped: Box, como piada de cair dentro da caixa

### [Capybaras! - Pixel Art Asset Pack, exclusiveOlive](https://exclusiveolive.itch.io/capybaras-pixel-art-asset-pack)

- Prévia: https://img.itch.zone/aW1nLzE5OTA0NTg2LmdpZg==/original/esI%2Bs4.gif
- Licença: "These assets can be used in both commercial and non-commercial projects. These assets can be edited. These assets cannot be redistributed or resold, even if the assets have been edited. Credit is appreciated, but it is not necessary :)"
- Preço: US$1,00 (50% de desconto sobre US$2,00, conferido em 2026-10-02). Também vem no Animal Bundle por US$3,50 (https://itch.io/s/201961/animal-bundle), com 7 packs: porquinho-da-índia, capivara, gato, sapo, raposa, arara e gaivota.
- Animações: Idle, Sleep(Start)/Lie down, Sleep, Awake, Walk Backwards, Walk Forwards, Run(Scared), Bite, Hurt, Death, Jump(Up), Jump(Down), Land, Sad(Start), Sad(Loop), Sad(End), Filhotes: Idle, Sleep, 2 variações, 32x32, Aseprite + sprite sheets
- Mapa para estados: - idle: Idle (+ filhote Idle)
- working: Bite (mastigando) ou Walk Forwards/Backwards de um lado pro outro
- done-small: Jump(Up) → Jump(Down) → Land
- done-big: pulo duplo + filhote subindo nas costas + mexerica quicando (a mexerica é arte nova)
- needs-attention: Run(Scared) no lugar + '!' (ou chamar a arara do mesmo pack-família)
- error: Sad(Start) → Sad(Loop) → Sad(End) (Hurt para falha grave)
- sleep: Sleep(Start) → Sleep → Awake
- dragged: frame de Jump(Down) segurado, com as patas penduradas
- dropped: Land

### [Cute Parrots! - Pixel Art Asset Pack, exclusiveOlive](https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack)

- Prévia: https://img.itch.zone/aW1nLzI3NjU5NjAyLmdpZg==/original/4ELIRq.gif
- Licença: "These assets can be used in both commercial and non-commercial projects. These assets can be edited. These assets cannot be redistributed or resold, even if the assets have been edited. Credit is appreciated, but it is not necessary."
- Preço: US$0,50 (50% de desconto sobre US$1,00), ou dentro do Animal Bundle por US$3,50.
- Animações: Idle, Walk, Sit, Sit Idle, Stand, Sleep (Start), Sleep, Awake, Eating, Chirp, Take Off, Fly, Glide, Dive (Start), Dive (Loop), Dive (End), Bite, Hurt, Landing, Death, 3 variações de cor, 48x48, Aseprite
- Mapa para estados: - idle: Idle/Sit Idle
- working: Eating (bicando) ou Walk
- done-small: Chirp + pulinho
- done-big: Take Off → Fly/Glide pela tela → Landing (ou Dive Start/Loop/End)
- needs-attention: Chirp em loop + voo até o canto do terminal
- error: Hurt
- sleep: Sleep (Start) → Sleep → Awake
- dragged: Fly, batendo as asas
- dropped: Landing

### [Frogs! - Pixel Art Asset Pack, exclusiveOlive](https://exclusiveolive.itch.io/frogs-pixel-art-asset-pack)

- Prévia: https://img.itch.zone/aW1hZ2UvMzk1NzQ3Ni8yMzU5MTYyMi5wbmc=/original/jltkOD.png
- Licença: "These assets can be used in both commercial and non-commercial projects. These assets can be edited. These assets cannot be redistributed or resold, even if the assets have been edited. Credit is appreciated, but it is not necessary."
- Preço: US$0,50 (50% de desconto sobre US$1,00), ou dentro do Animal Bundle por US$3,50.
- Animações: Idle, Idle 2, Idle 3, Run, Sleep(Start)/Lie down, Sleep, Awake, Attack, Attack(Up), Jump, Fall, Landing, Hurt, Death, Sit, Sit(Idle), Stand, Surprised, Grapple, Grapple(Up), Grab Ledge, Climb Ledge, Planador de folha (bônus), Filhotes: Idle, Sleep, 4 variações, 48x48
- Mapa para estados: - idle: Idle/Idle 2/Idle 3/Sit(Idle)
- working: Attack/Grapple em loop ('caçando bugs'; confirmar no GIF se é a língua)
- done-small: Jump → Landing
- done-big: planador de folha descendo + Landing
- needs-attention: Surprised + pulo + '!'
- error: Hurt
- sleep: Sleep(Start) → Sleep → Awake
- dragged: Grab Ledge (pendurado)
- dropped: Fall → Landing

### [Cute Capybara, Kalengo Phiri](https://kalengophiri.itch.io/cute-capybara)

- Prévia: https://img.itch.zone/aW1hZ2UvMjk4NTc3MC8xNzg1ODg5OS5naWY=/original/nA3Up1.gif
- Licença: "commercial use allowed in shipped games and prototypes, modification allowed, no redistribution / resale / re-upload, no training AI on the files, no NFT / crypto use."
- Preço: Pague quanto quiser (grátis). Status: 'In development'.
- Animações: Walk (4 direções), Idle, Happy, Sad, Sleep, Party
- Mapa para estados: - idle: Idle
- working: Walk (patrulha em 4 direções)
- done-small: Happy
- done-big: Party
- needs-attention: Walk de frente vindo até você + Happy + '!'
- error: Sad
- sleep: Sleep
- dragged: não tem; congelar um frame de Sad ou Happy
- dropped: não tem; desenhar 3 frames

### [Custom Capybara Cursor - 32px, Melloinc](https://melloinc.itch.io/custom-capybara-cursor-32px)

- Prévia: https://raw.githubusercontent.com/mellotadaa/cursorTRex/refs/heads/main/cursorCapybara_allScreenshot.gif
- Licença: "You may use this asset for personal and commercial purposes. Feel free to modify it to your needs. Credit is not required but would be appreciated. Please consider donating as it will greatly help me continue making art!" + "Please DO NOT resell or redistribute this item. Thank you!"
- Preço: US$4,00+ (o instalador é só para Windows, mas os GIFs são arquivos comuns).
- Animações: 17 GIFs de cursor 32x32 (escalados para 256x256), Poses descritas pela pesquisa: normal (sentada com mexerica), ajuda com '?', ocupada no ofurô (2 variações), precisão, seleção de texto, escrevendo com lápis, indisponível, redimensionar (x4), mover, alternativa, link (joinha), pessoa (duas capivaras empilhadas), localização (pin)
- Mapa para estados: - idle: pose normal (sentada com a mexerica)
- working: lápis (handwriting) ou 'ocupada' no ofurô
- done-small: joinha (link select)
- done-big: não tem; desenhar
- needs-attention: pose de ajuda com '?'
- error: indisponível (unavailable)
- sleep: ofurô de olhos fechados
- dragged: mover (move)
- dropped: não tem; desenhar
São loops curtos de cursor de 32px: escala x4.

### [Silly Dog Retriever (TopDown), ChanceKite](https://chancekite.itch.io/sillydog)

- Prévia: https://img.itch.zone/aW1hZ2UvMzI4MTEzNi8xOTU4MTA0MS5naWY=/original/pc%2F5QD.gif
- Licença: "Upon purchase, this artwork can be used in personal and commercial games. Credit is appreciated. Do not sell or redistribute any part of this artwork."
- Preço: US$30,00+ ('No generative AI was used'; 5,0 estrelas com só 1 avaliação).
- Animações: Stand, Walk, Run, Chase, Jump, Trudge, Sneak, Crouch, Sit, Lay, Laze, Sleep, Yawn, Sniff, Lick, Eat, Bark, Bite, Howl, Look-Around, FurShake, RollOver, TailChase, Dig, AirPaw, HiFive, Scratch, Pick-Item, Wait, Petted, BellyUp, Stun, Yes, No, Tired, Scared, Sigh, 6 pelagens (White, Gold, Cream, Chestnut, Choc, Black); frente/costas 24x48, lado 48x48; 2-8 frames por animação
- Mapa para estados: - idle: Stand/Sit/Look-Around/Scratch/Lick, sorteados
- working: Dig (cavando), Sniff (lendo/buscando), Pick-Item (trazendo o resultado)
- done-small: Yes ou um TailChase curto
- done-big: HiFive → RollOver → TailChase
- needs-attention: Bark → Wait (sentado te olhando) → Howl se ignorado
- error: Sigh ou No (Scared/Stun para falha grave)
- sleep: Yawn → Laze/Lay → Sleep (Tired antes)
- dragged: Petted ou BellyUp
- dropped: FurShake
Bônus: Yes/No para pedidos de permissão.

### [Pixel Adventure 1, Pixel Frog (Ninja Frog, Mask Dude, Pink Man, Virtual Guy)](https://pixelfrog-assets.itch.io/pixel-adventure-1)

- Prévia: https://img.itch.zone/aW1nLzI1Mzc4MzcuZ2lm/original/OArrbk.gif
- Licença: "Creative Commons Zero (CC0) license. You can distribute, remix, adapt, and build upon the material in any medium or format, even for commercial purposes. Attribution is not required." (O espelho no OpenGameArt lista CC-BY 4.0; a página da autora diz CC0.)
- Preço: Pague quanto quiser (grátis).
- Animações: Idle, Run, Jump, Double Jump, Wall Jump, Fall, Hit, Appearing (96x96), Desappearing (96x96), 4 personagens 32x32, 20 FPS (50 ms)
- Mapa para estados: - idle: Idle
- working: Run no lugar
- done-small: Jump
- done-big: Double Jump (giro) → Fall → aterrissagem, com o 'poof' de Appearing
- needs-attention: Wall Jump grudado na borda da tela/terminal + balão '!'
- error: Hit
- sleep: não tem; Idle + 'Zzz', ou editar os olhos fechados (o CC0 permite)
- dragged: Fall
- dropped: final do Jump + poeira (arte nova)
Extra: Desappearing/Appearing para o pet se teletransportar.

### [gophers (animações do Go Gopher), Egon Elbre](https://github.com/egonelbre/gophers)

- Prévia: https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/animation/gopher-dance-long-3x.gif
- Licença: "The images and art-work in this repository are under CC0 license." + "The Go gopher was designed by the awesome Renee French." (go.dev: "The design is licensed under the Creative Commons 4.0 Attributions license.")
- Preço: Grátis.
- Animações: dance (gopher-dance-long), party, aww, eyeroll, ninja, morning-coffee, morning-beer, buy-morning-coffee, 2-bit sprite run/walk (sheet + JSON), fontes .ase + palette.ase
- Mapa para estados: - idle: morning-coffee
- working: ninja ou run 2-bit
- done-small: party
- done-big: dance
- needs-attention: aww (+ um aceno a desenhar)
- error: eyeroll
- sleep, dragged e dropped: não têm; desenhar a partir dos .ase

### [2D Pixel Art Armadillo Sprites, Elthen](https://elthen.itch.io/2d-pixel-art-armadillo-sprites)

- Prévia: https://img.itch.zone/aW1nLzYxNDMxMzQuZ2lm/original/1OdTCZ.gif
- Licença: "Feel free to use the sprites in commercial/non-commercial projects! If you do, please consider tipping, or at least dropping a comment down below." Os termos completos estão num post do Patreon (patreon.com/posts/licensing-27430241) que não abriu (403).
- Preço: Pague quanto quiser. Também vem no Animals Bundle #1 (US$20, 25 packs).
- Animações: Idle, Movement, Curl, Damage, Death, 32x32
- Mapa para estados: - idle: Idle
- working: Movement (andando de lá pra cá)
- done-small: Curl → desenrolar com 'tcharã' (arte nova)
- done-big: Curl + rolar pela tela (girar o sprite da bola no código) + desenrolar
- needs-attention: desenrolar e espiar repetidamente (arte nova)
- error: Damage
- sleep: Curl parado, com a bola respirando (2 frames novos)
- dragged: Curl (vira bola na hora)
- dropped: a bola quica e desenrola (arte nova)

### [Pet Cats Pack, LuizMelo](https://luizmelo.itch.io/pet-cat-pack)

- Prévia: https://img.itch.zone/aW1hZ2UvMTk0NzA3Ni8xMTQ2ODM5Mi5naWY=/original/e%2BgtA1.gif
- Licença: "Creative Commons Zero v1.0 Universal"
- Preço: Pague quanto quiser (grátis).
- Animações: Idle (10f), Walk (8f), Run (8f), Meow (4f), Lying Down (8f), Itch (2f), Sleeping1 (1f), Sleeping2 (1f), Sitting (1f), Licking1 (5f), Licking2 (5f), Stretching (13f), 6 gatos de ~20x14 px
- Mapa para estados: - idle: Idle/Licking/Itch/Sitting, sorteados
- working: Walk/Run (zoomies pela base da tela)
- done-small: Stretching
- done-big: Run em círculo + emote de coração (não há comemoração própria)
- needs-attention: Meow em loop
- error: Itch + emote bravo
- sleep: Lying Down → Sleeping1/2 (acorda com Stretching)
- dragged: Sitting
- dropped: Lying Down/Sitting (recomendado desenhar)
É a skin de gato CC0 para um repo público. O sprite é minúsculo: escala x5-x6.

### [Emotes Pack, Kenney (balões para qualquer skin)](https://kenney.nl/assets/emotes-pack)

- Prévia: https://kenney.nl/media/pages/assets/emotes-pack/0cae06c6af-1677578794/preview.png
- Licença: "Creative Commons CC0"
- Preço: Grátis.
- Animações: 480 ícones/balões de emote estáticos (não animados)
- Mapa para estados: Não é um pet; são balões para sobrepor a qualquer skin, numa camada separada:
- needs-attention: '!' quicando
- pergunta/permissão: '?'
- done-small/done-big: coração/estrela
- sleep: 'Zzz'
- error: raiva/caveira
- working: '...'
A animação (quique/escala) é feita no código. Recolorir para a paleta da skin escolhida.

## Regras de estilo para quadros novos

- Tamanho nativo e âncora: todo frame novo é desenhado no canvas nativo do pack escolhido (Pochi 64x64; Mochi e capivara 32x32; arara e sapo 48x48). Os pés ficam na mesma linha de pixel e o ponto de âncora é o mesmo de todas as animações originais. Nunca desenhar grande e reduzir depois.
- Escala só inteira, com nearest-neighbor: 32px vira 128px (x4), 48px vira 144px (x3), 64px vira 128px (x2). No Hyprland, compensar o fator de escala do monitor para que o resultado final continue inteiro. Pixel borrado denuncia 'arte de programador'.
- Paleta travada: extrair a paleta do próprio sprite sheet (no Aseprite, Palette > New Palette from Sprite) e usar só essas cores. Props novos (mexerica, notebook, balões, confete, Zzz) reaproveitam as rampas existentes. No máximo uma rampa nova de 3 tons (por exemplo, o laranja da mexerica), com o mesmo deslocamento de matiz do pack: sombra puxando para vermelho/roxo, luz para amarelo.
- Contorno igual ao do pack: 1px, na cor mais escura da rampa do material (marrom ou roxo-escuro, nunca #000000). Linhas internas ficam 1 tom acima do preenchimento. Se o pack usa contorno seletivo (mais claro no lado iluminado), repetir.
- Luz de cima-esquerda, 3-4 tons por material, sem gradiente suave e sem dithering (a menos que o pack use). Nada de anti-aliasing contra o fundo transparente: o pet flutua sobre qualquer wallpaper, e o anti-aliasing vira um halo sujo.
- Rosto e proporção: reaproveitar os olhos do pack pixel a pixel (formato, tamanho e pixel de brilho sempre no mesmo canto). A emoção vem da pálpebra ou sobrancelha (1px), da boca de 1-3px e de 1-2px de bochecha rosada. Boca grande só para comemorar e chamar. Não 'corrigir' a anatomia: cabeça grande, corpo curto em formato de saco de farinha, membros curtos (a regra 'Cuteness' do saint11).
- Timing do pack: medir no Aseprite a duração de frame original e manter a mesma base (a Pixel Frog usa 50 ms; Kings and Pigs e Treasure Hunters usam 100 ms). Segurar as poses-chave e acelerar os intermediários; no idle, por exemplo, segura ~0,6 s e mexe ~0,2 s.
- Quantidade de frames por estado: idle 4-8 (respiração de 1px + piscada aleatória a cada 3-6 s); trabalhando 4-6 em loop; feito-pequeno 4-6 (até 1 s); feito-grande 8-12 (2-3 s, depois volta ao idle); chamar 4-6 em loop; erro 4-6; dormir com entrada de 3-4, loop lento de 2 frames e acordar de 3-4; arrastado 2-4; solto 3 (+2 de poeira).
- Física de desenho animado: squash & stretch com volume constante, 1-2 frames de antecipação antes de pulos e 1 frame de smear em giros. No erro, 1 frame de silhueta branca + tremida de 2-4px. Arrastado = pendurado pelo ponto de pega, com as patas balançando e o corpo esticado para baixo. Solto = achatamento na aterrissagem + nuvenzinha de poeira.
- Overlays em camada ou sprite separado, nunca 'queimados' no personagem: '!', '?', '...', 'Zzz', confete, gota de suor, mexerica. Usam o mesmo contorno de 1px e a mesma paleta do pack. O balão tem borda escura de 1px, fundo off-white e rabicho apontando para o pet. Texto em fonte pixel (5x7 ou 6x8) na mesma escala inteira, com frases curtas em PT-BR: 'Pronto!', 'Psiu! Preciso de você', 'Deu ruim...', 'Posso?'.
- Escada de atenção, para chamar sem irritar: 1) olhar ou orelha em pé; 2) balão '!' quicando; 3) pulo, aceno ou caminhada até a borda do terminal; 4) um som curto com cooldown. Nada de loop infinito de movimento grande. Silenciar quando o terminal já está focado ou em 'não perturbe'. A comemoração dura 2-3 s e volta ao idle.
- Uma região de cor por estado, como o cabelo da Madeline em Celeste: só um detalhe muda de cor (o laço do Mochi, que já vem em 8 cores; a folha da mexerica; a coleira do Caramelo). Azul = trabalhando, verde = pronto, amarelo pulsando = precisa de você, vermelho = erro, dessaturado = dormindo. Sempre acompanhado de um ícone, sem depender só da cor.
- Chão e contraste: uma sombra elíptica de 1-2px embaixo do pet, em camada própria, para ele parecer apoiado na borda ou na base da tela. Testar no tema claro e no escuro do Omarchy e num wallpaper movimentado, tanto a 1x quanto na escala final. Se o pet sumir, adicionar um halo claro de 1px estilo adesivo.
- Um artista por personagem: nunca misturar sprites de artistas diferentes no mesmo bicho. O ajudante (a arara) só entra se for do mesmo artista (exclusiveOlive) ou se for redesenhado com estas regras. Nada gerado por IA: quebra a grade de pixels, e vários autores proíbem.
- Aseprite como fonte da verdade: uma tag por estado (idle, working, done_small, done_big, attention, error, sleep_start, sleep, wake, dragged, dropped), exportando sheet + JSON. Skins pagas ficam numa pasta local fora do git.
- Se o escolhido for o caranguejo: usar o laranja-avermelhado do Ferris, olhos grandes com brilho e garras expressivas. Evitar o salmão do Clawd (~#DA7758) e o corpo-bloco com dois olhos, por causa do risco de marca.

## Verificação

### Links

- OK https://toffeecraft.itch.io/cat-pack — Loaded. Page title is 'Cat Pack - Mochi' by ToffeeCraft, 32x32. Free tier: 'FreePack has only 2 animations.' Paid tiers: CatRoomPaid.zip $1+ and CatPackPaid.zip $1.90+. Rated 4.9 stars (62 ratings). States 'No generative AI was used'. Devlog line: 'UPDATE 2 JUL 2025: I added 2 new cat animations, licking and despite face.' Author comments confirm walk/run are not in Mochi yet and are planned.
- OK https://img.itch.zone/aW1nLzE4OTMyMTcwLmdpZg==/original/yQjbzV.gif — 200 image/gif, 512x384, appears on the cat-pack and cat-mega-bundle pages. First frame viewed: sheet 'ANIMATIONS - 1' with Sleepy, Idle, Sleeping, Dance, Excited, Idle 2. Cream/orange tabby with a 'preview' watermark. Matches the claim.
- OK https://toffeecraft.itch.io/cat-retro — Loaded. Page title is 'Cat Pack - Pochi' by ToffeeCraft, 64x64. Text lists 18 animations: Idle, Run, Jump, Attack, Hurt, Die, Die2, Sleeping, Happy, Cry, Tickle, Chilling, Dance, Surprised, So full, Box1-3. 6 coats as claimed. RetroCatsPaid.zip costs $1.80+, plus a free RetroCatsFree.png.zip. Rated 5.0 stars (44 ratings). States 'No generative AI was used'.
- OK https://img.itch.zone/aW1nLzIwMjMxNDg1LmdpZg==/original/mVL7QS.gif — 200 image/gif, 511x334, on the Pochi page, and it is the Pochi. It only partly matches the 'why': the sheet is 'ANIMATIONS - 1' with RUNNING, IDLE, SLEEPING, JUMPING, EXCITED, HAPPY. Dance, Surprised, Cry, Tickle, Chilling and So full are on U2Mrsz.gif, and Box is on KFG8zA.gif.
- OK https://img.itch.zone/aW1nLzIwMjMxNDg2LmdpZg==/original/U2Mrsz.gif — Suggested replacement or addition for the Pochi. 200 image/gif, on the cat-retro page. Viewed: 'ANIMATIONS - 2' with CRY, TICKLE, CHILLING, DANCE, SURPRISED, SO FULL.
- OK https://img.itch.zone/aW1nLzIwMjMxNDk0LmdpZg==/original/KFG8zA.gif — Pochi 'ANIMATIONS - BOX' (3 box poses), on the cat-retro page. Also on that page: https://img.itch.zone/aW1nLzIwMjMxNDk3LmdpZg==/original/LgUz2p.gif, 'ANIMATIONS - FURRY FURY' (3 attack-like poses).
- OK https://img.itch.zone/aW1nLzIxOTU4NzIwLmdpZg==/original/bo6BCp.gif — 200 image/gif, 512x285. Viewed: 'ANIMATIONS - 4' with LICKING and DESPITE (the author's spelling). It is embedded in the main cat-pack page body, so it is not a devlog-only image.
- OK https://img.itch.zone/aW1nLzE4OTMyMTUzLmdpZg==/original/tRDXKN.gif — Extra Mochi sheet on the cat-pack page. Viewed: 'ANIMATIONS - 2' with SURPRISED, CRYING, EATING, WAITING, DEAD, LAY DOWN. Further sheets: https://img.itch.zone/aW1nLzIxMzk1NzY0LmdpZg==/original/UPtmRC.gif ('ANIMATIONS - 3': SHY, REFUSING, ANGRY) and https://img.itch.zone/aW1nLzIyNDgzNTM2LmdpZg==/original/ml5Sr4.gif ('ANIMATIONS - 5': SICK 1, SICK 2).
- OK https://exclusiveolive.itch.io/capybaras-pixel-art-asset-pack — Loaded. 'Capybaras! -Pixel Art Asset Pack' by exclusiveOlive. Priced $1.00 (50% off $2.00), also in the Animal Bundle at $3.50. 2 adult variations with 16 animations, plus babies (Idle, Sleep). 32x32, Aseprite files and SpriteSheets. Rated 5.0 stars (1 rating). States 'No generative AI was used'.
- OK https://img.itch.zone/aW1nLzE5OTA0NTg2LmdpZg==/original/esI%2Bs4.gif — 200 image/gif, first image on the page. Viewed: 'Capybaras' title banner showing side-view orange capybaras in a forest, with a baby riding on one's back. Matches.
- OK https://melloinc.itch.io/custom-capybara-cursor-32px — Loaded. 'Custom Capybara Cursor - 32px' by Melloinc, $4.00+. Windows-only installer. 17 GIFs at 32x32, 'scaled to 256 x 256px'. No rating. States 'No generative AI was used'.
- OK https://raw.githubusercontent.com/mellotadaa/cursorTRex/refs/heads/main/cursorCapybara_allScreenshot.gif — 200 image/gif (341 KB), linked from the itch page. Viewed: front-view chibi capybara with closed eyes and an orange with leaf on its head. Shows '?', hot-spring bath x2, pencil, no-entry sign, resize arrows, thumbs-up, two stacked capybaras and a map pin. Signed 'MELLO INC'. Matches.
- OK https://kalengophiri.itch.io/cute-capybara — Loaded. 'Cute Capybara' by Kalengo Phiri, name your own price, status 'In development'. Animations: Walk (4 directions), Idle, Happy, Sad, Sleep, Party. Resolution not stated. States 'No generative AI was used'.
- OK https://img.itch.zone/aW1hZ2UvMjk4NTc3MC8xNzg1ODg5OS5naWY=/original/nA3Up1.gif — 200 image/gif, on the page. Viewed: one side-view brown capybara walking. It does not show Party or the front view; those come from the page text.
- OK https://14collective.itch.io/8-bit-capibaras — Loaded. 'Pixel Capibaras - 8bit Character Sprites' by the14collective, $2.00+ (price not given in the moodboard). 22 sprites, 11 animations, 2 variants including 'Capybara with orange on head'. Rated 5.0 stars (2 ratings).
- OK https://img.itch.zone/aW1nLzIxNDA5NDE4LmdpZg==/original/knV4yS.gif — 200 image/gif, on the page. Viewed: side-view capybaras with an orange on their heads in a swamp scene. Matches.
- OK https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack — Loaded. $0.50 (50% off $1.00), in the Animal Bundle. 3 parrot variations, 48x48. Says '21 different animations' but the list repeats Hurt, so 20 are unique. The colors are not named in the text. Rated 5.0 stars (1 rating).
- OK https://img.itch.zone/aW1nLzI3NjU5NjAyLmdpZg==/original/4ELIRq.gif — 200 image/gif (1.67 MB), on the page. First frame viewed: labeled 'Idle', a chubby red macaw-type parrot (red body, yellow/blue wing, big white eye) on a branch. Matches.
- OK https://elthen.itch.io/2d-pixel-art-parrot-sprites — Loaded. '2D Pixel Art Parrot Sprites' by Elthen's Pixel Art Shop, $1.00+. Animations: Idle, Idle2, Walk, Squawk, Idle Flying, Fly, Attack, Damage, Death. 32x32. Species is not named in the text. Banner image iE%2BhjV.png reads 'NOT FOR BLOCKCHAIN/CRYPTO RELATED PROJECT USE. READ THE LICENSE.' Rated 5.0 stars (4 ratings).
- OK https://img.itch.zone/aW1nLzM0MTgwMTMuZ2lm/original/gDauF%2B.gif — 200 image/gif, on the page. Viewed: scarlet-macaw-style parrots (red, yellow, blue) in several poses. Matches 'arara-vermelha'.
- OK https://seethingswarm.itch.io/parrotpack — Loaded. 'Parrotpack' by seethingswarm, $11.99+. Animations include Sit_caw, Idle_caw and Soar. Rated 3.0 stars (2 ratings).
- OK https://img.itch.zone/aW1nLzEzMTEwODc4LnBuZw==/original/ViS5SM.png — 200 image/png, 630x500, on the page. Static cover, not animated. Viewed: 'PARROTPACK' title with scarlet macaw poses.
- OK https://chariart.itch.io/jungle-birds — Loaded. Page title is 'Jungle Birds - Pack' by Lara (chariart), free. 7 Pantanal birds: Hyacinth Macaw, Tuiuiu, Toucan, Yellow-fronted parakeet, Great Egret, Soco, American Rhea. 2-frame animations. Rated 5.0 stars (2 ratings).
- OK https://img.itch.zone/aW1nLzk5MTcyNTkuZ2lm/original/QGZttR.gif — 200 image/gif (4.9 KB), on the page. Viewed: tiny Tuiuiu (jabiru) with white body, black head and red neck band. Matches.
- OK https://chancekite.itch.io/sillydog — Loaded. 'Silly Dog Retriever (TopDown)' by ChanceKite, $30.00+. 37 animations. 6 coats: White, Gold, Cream, Chestnut, Chocolate, Black. Frames are 24x48 front/back and 48x48 side, with 2-8 frames per animation. Rated 5.0 stars (1 rating).
- OK https://img.itch.zone/aW1hZ2UvMzI4MTEzNi8xOTU4MTA0MS5naWY=/original/pc%2F5QD.gif — 200 image/gif (2.9 MB), on the page. Viewed: demo with an 'Animation List: Chase' dropdown and a golden dog on a grass field. The dog is very small within the 450x300 frame, which confirms the 'miúdo' risk.
- OK https://chancekite.itch.io/pixeldog03 — Loaded. Title is 'Pixel Dog Pomeranian "Orange" (SideScrolling)'. Also costs $30.00+. 40x32, 14 animations including hi-five, beg, roll-over, fur-shake and howl.
- OK https://img.itch.zone/aW1hZ2UvMTU0NDI2NS8xMzkxMzMyNy5naWY=/original/96%2BJS6.gif — 200 image/gif, the only image on the page. Viewed: title card 'Pomeranian ... ChanceKite Pixel Dog Animation' with an orange pomeranian.
- OK https://benvictus.itch.io/pixel-dogs — Loaded. 'Pixel Dogs' by Benvictus, name your own price. 12 coat colors, 64x48. Animations: Idle (lay/sit/stand), Walk, Run, Beg, Sleep, plus Holding Ball and Barking frames. Rated 4.8 stars (9 ratings).
- OK https://img.itch.zone/aW1nLzE2Nzk4MjI4LmdpZg==/original/IDq5Fc.gif — 200 image/gif, on the page. Viewed: 'Pixel Dogs' logo with one dark-grey dog sitting. It is title art and does not show the coat variety.
- OK https://exclusiveolive.itch.io/frogs-pixel-art-asset-pack — Loaded. $0.50 (50% off $1.00), in the Animal Bundle. 22 animations as listed. 4 variations, 48x48, babies (Idle, Sleep) and leaf gliders. Rated 5.0 stars (3 ratings).
- OK https://media3.giphy.com/media/v1.Y2lkPTc5MGI3NjExOXNzNnZ4cXhuazV3b2loNDBvZ25xcXFibDNleTA4YnZsMWp2Y3l5cSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/R5IBTL2VKcdbLWVhjN/giphy.gif — 200 image/gif, 315x249, embedded in the page description. Viewed: 'Frogs' title card with 4 side-view frogs (green, red, teal, brown). It is the frog, not the 'Rate' GIF. It does not show the tongue, Grapple or leaf glider.
- OK https://img.itch.zone/aW1hZ2UvMzk1NzQ3Ni8yMzU5MTYyMi5wbmc=/original/jltkOD.png — 200 image/png (2.4 KB), 315x249. It is the original-size version of the page's first screenshot. Static copy of the same 'Frogs' title card with 4 frog colors.
- OK https://img.itch.zone/aW1nLzE4Nzk2Mjg2LmdpZg==/original/dJpcrM.gif — Not in the moodboard; checked only to confirm the warning. This is the 'Rate' GIF on the exclusiveOlive capybara and frog pages: 5 sad-faced stars, no character.
- OK https://pixelfrog-assets.itch.io/pixel-adventure-1 — Loaded. Page title is 'Pixel Adventure' by Pixel Frog, name your own price. 'These assets are released under a Creative Commons Zero (CC0) license.' Itch license field: 'Creative Commons Zero v1.0 Universal'. 'All animations have a speed of 20 FPS or 50 MS.' Rated 4.9 stars (665 ratings). Character names are not in the text, but the 4 screenshot GIFs show Mask Dude, Virtual Guy, Pink Man and Ninja Frog.
- **FALHA** https://img.itch.zone/aW1nLzI1Mzc4MzcuZ2lm/original/OArrbk.gif — Wrong content. Loads (200 image/gif) and is on the PA1 page, but it shows enemies: potted plant, pig, pink bunny, chicken, duck. On the page it sits next to 'Download all the 20 enemy characters in Pixel Adventure 2', which is a separate $5+ pack. It does not show the Ninja Frog or any PA1 character.
- OK https://img.itch.zone/aW1hZ2UvNDkwNzk4LzI1Mzk2NDguZ2lm/original/oGsPQt.gif — Replacement for OArrbk.gif. PA1 screenshot GIF, 200 image/gif (1.98 MB). Alt text: 'Pixel art platformer level with a green character...'. Viewed: Ninja Frog (green, red headband) in a level. The frog is small in the shot.
- OK https://store.steampowered.com/app/3416070/Ropukas_Idle_Island/ — Loaded. Developer: Moczan and Little Chmura, Begoña Pereda. 'Watch the frog cut grass...' 95% of 2,384 reviews positive. $3.99, on sale at $2.79 until Oct 8. Released Jan 29, 2025.
- OK https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/3416070/header.jpg?t=1789671909 — 200 image/jpeg. Viewed: 'ROPUKA'S IDLE ISLAND' logo with a frog by a small house on a grassy island and a 'zZ'. Matches.
- OK https://github.com/egonelbre/gophers — Loaded, 3.8k stars. README: 'The images and art-work in this repository are under CC0 license.' and 'The Go gopher was designed by the awesome Renee French.' Has a LICENSE-CC0 file. Aseprite sources exist: animation/dance.ase (64x64, 8 frames), morning-coffee.ase (64x64, 10 frames), ninja.ase, ninja-large.ase, palette.ase; icon/eyeroll.ase (32x32, 14 frames), icon/typing-furiously.ase (32x32, 13 frames), icon/awww.ase. The 2bit-sprite sheet.png and sheet.json are present.
- OK https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/animation/gopher-dance-long-3x.gif — 200 image/gif, 192x192 (the 1x version is 64x64). Viewed: full-body blue gopher with big eyes dancing.
- OK https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/animation/party.gif — 200 image/gif, 96x96. Head-only gopher, a 32x32 icon scaled 3x.
- OK https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/animation/eyeroll-3x.gif — 200 image/gif, 96x96. Head-only gopher with half-lidded eye roll (32x32 source). Matches.
- OK https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/animation/morning-coffee-3x.gif — 200 image/gif, 192x192. Viewed: full-body gopher holding a mug. Matches.
- OK https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/icon/typing-furiously-3x.gif — Suggested addition for the 'trabalhando' state. 200 image/gif. Viewed: head-only gopher typing furiously on a keyboard. Source is icon/typing-furiously.ase. Also present: icon/emoji/gopher-sleeping.png (static 'zz' emoji), .thumb/animation/aww.gif and ninja-3x.gif, all 200.
- OK https://elthen.itch.io/2d-pixel-art-armadillo-sprites — Loaded. Name your own price. Animations: Idle, Movement, Curl, Damage, Death. 32x32. Part of Animals Bundle #1 ($20, 25 packs). Has the same 'NOT FOR BLOCKCHAIN/CRYPTO RELATED PROJECT USE. READ THE LICENSE.' banner image. Rated 5.0 stars (3 ratings).
- OK https://img.itch.zone/aW1nLzYxNDMxMzQuZ2lm/original/1OdTCZ.gif — 200 image/gif. Viewed: 5 grey armadillo poses on a checkerboard. Matches; a recolor to yellow-brown would be needed.
- OK https://fujiidew.itch.io/8-bit-tortoise-sprite — Loaded. $1.00+. 16 animations of 8 frames each, including Hiding, Startled, Sleeping, 'Flipped on shell' and Flipping, in left and right versions.
- OK https://img.itch.zone/aW1nLzgyMjY3NzcuZ2lm/original/%2BXZCCZ.gif — 200 image/gif. Viewed: green tortoise with a brown shell walking, side view.
- OK https://saint11.art/blog/pixel-art-tutorials/ — Loaded. 'Pixel Art Tutorials' by Pedro Medeiros. Squash.gif and Cuteness.gif are present. Footer: '© 2025 Pedro Medeiros', with no CC license. The page itself does not mention Celeste.
- OK https://saint11.art/img/pixel-tutorials/Squash.gif — 200 image/gif. Viewed: 'SQUASH AND STRETCH' tutorial card ('MASS NEVER CHANGES', anticipation, prepare/jump/brake/recover).
- OK https://www.rustacean.net/ — Loaded. Waiver text is verbatim as claimed. Links assets/cuddlyferris.png. No pixel-art Ferris on the page.
- OK https://www.rustacean.net/assets/cuddlyferris.png — 200 image/png, 460x344. Viewed: red-orange cuddly Ferris with big eyes and a wide smile.
- OK https://github.com/rullerzhou-afk/clawd-on-desk — Loaded, 6.4k stars. License statements are verbatim as claimed, and the README says juggling = 2+ subagents.
- OK https://raw.githubusercontent.com/rullerzhou-afk/clawd-on-desk/main/assets/gif/clawd-juggling.gif — 200 image/gif. Viewed: salmon block-bodied Clawd juggling 3 balls.
- OK https://github.com/afspies/ClawdMoji — Loaded, 37 stars. Code is MIT. Disclaimer contains 'no rights to the character or mark are granted.'
- OK https://raw.githubusercontent.com/afspies/ClawdMoji/main/emoji/fire/clawd_fire.gif — 200 image/gif. Viewed: Clawd standing in flames ('this is fine').
- OK https://luizmelo.itch.io/pet-cat-pack — Loaded. 'Pet Cats Pack' by LuizMelo, name your own price. 6 cats at 20x14 (cat 4 is 22x15, cat 5 is 22x14). Animation and frame counts match. Rated 4.7 stars (20 ratings).
- OK https://img.itch.zone/aW1hZ2UvMTk0NzA3Ni8xMTQ2ODM5Mi5naWY=/original/e%2BgtA1.gif — 200 image/gif (0.9 MB). Viewed: small, realistic (not chibi) cats in black, orange, white, siamese, grey and tabby, in many poses.
- OK https://kenney.nl/assets/emotes-pack — Loaded. License 'Creative Commons CC0'. 480 static assets, free.
- OK https://kenney.nl/media/pages/assets/emotes-pack/0cae06c6af-1677578794/preview.png — 200 image/png, 918x515. Viewed: emote balloons ('!', '?', 'Z', '...', hearts, angry, etc.) in 8 styles. Footer reads 'CC0 1.0 This content is free to use in personal, educational and commercial projects.'
- OK https://itch.io/s/201961/animal-bundle — Loaded. Animal Bundle by exclusiveOlive, $3.50 (regular $8.00). Contents: Guinea Pigs, Capybaras, Cats, Frogs, Cute Foxes, Cute Parrots, Cute Seagulls.
- OK https://toffeecraft.itch.io/cat-mega-bundle — Not linked in the moodboard; it should be added. Loaded. Paid file costs '$2.40 USD or more'. Includes Mochi (23 animations), Pochi (18), Little Kitties, Cat Room, Cat User Interface. Rated 5.0 stars (31 ratings). Same ToffeeCraft license.
- OK https://toffeecraft.itch.io/pet-virtual-mobile-pixel-asset — URL for the 'Pet Mobile Pixel Asset Pack' (plan B). Loaded. $1.80+. Cats have dancing and surprising, dogs have bark, and it also has 5 parrot colors. Rated 5.0 stars (38 ratings).
- OK https://2tin.itch.io/capybara — Loaded. Page title is 'capybara! ₍ᐢ-(ｪ)- ྀིᐢ₎' by Tine, $5.00+. 24 animations, side view, 16x16 and 32x32. Quote: 'I will create a separate pack (PET EXPANSION) or add to this one for the following: turn, front view, dances, emotes.'
- **FALHA** https://www.patreon.com/posts/licensing-27430241 — Returned HTTP 403 again. Elthen's full license text is still unverified.
- OK https://go.dev/doc/gopher/README — Source of the quote: 'The Go gopher was designed by Renee French. ... The design is licensed under the Creative Commons 4.0 Attributions license.' go.dev/brand says '...licensed under the Creative Commons 4.0 Attribution License.'
- OK https://opengameart.org/content/pixel-adventure-1 — OpenGameArt mirror, submitted by Pixel Frog. License field: CC-BY 4.0, which confirms the moodboard's caveat.
- OK https://pixelfrog-assets.itch.io/pixel-adventure-2 — The pack the OArrbk.gif enemies come from. $5.00+, CC0, 20 enemy characters.
- OK https://en.wikipedia.org/wiki/OpenClaw — Confirms the Clawdbot claim: renamed to Moltbot on Jan 27, 2026 'following trademark complaints by Anthropic', then OpenClaw on Jan 30, 2026.

### Licenças

- OK ToffeeCraft - Cat Pack Pochi (cat-retro): alegado «Paid: "For commercial or personal use. - It can be used and edit freely. - Not redistribute or resell this assets. - It can be used for game development and other productions." Free: "For personal use. ..." Costs $1.80+. A public repo must not commit the sprites.» / observado «Identical paid and free texts. RetroCatsPaid.zip 'if you pay $1.80 USD or more'. The free file is RetroCatsFree.png.zip (21 kB); its animation count is not stated on this page.»
- OK ToffeeCraft - Cat Pack Mochi (cat-pack): alegado «Same ToffeeCraft license. Free version has 2 animations and is personal use only. Full pack $1.90+, Cat Room $1+.» / observado «Identical license texts. 'FreePack has only 2 animations.' CatRoomPaid.zip $1+ and CatPackPaid.zip $1.90+.»
- OK ToffeeCraft - Cat Mega Bundle: alegado «$2.40+, includes Pochi, Mochi, Little Kitties, Cat Room, Cat UI» / observado «'if you pay $2.40 USD or more'. Includes Mochi (23 animations), Pochi (18), Little Kitties, Cat Room, Cat User Interface. Same free and paid license texts. Redistribution is forbidden, so the public-repo conclusion stands.»
- OK ToffeeCraft - Pet Mobile Pixel Asset Pack: alegado «US$1.80+, cats with Dancing/Surprising, dogs with Bark (no license quoted)» / observado «$1.80+. Same ToffeeCraft license: paid is "For commercial or personal use. It can be used and edit freely. Not redistribute or resell this assets. ..." Cats have dancing and surprising; dogs have bark.»
- OK exclusiveOlive - Capybaras!: alegado «"These assets can be used in both commercial and non-commercial projects. These assets can be edited. These assets cannot be redistributed or resold, even if the assets have been edited. Credit is appreciated, but it is not necessary :)"» / observado «Same text, written as a dash list. No public redistribution.»
- OK exclusiveOlive - Cute Parrots!: alegado «Same text without the ':)'» / observado «"-These assets can be used in both commercial and non-commercial projects. -These assets can be edited. -These assets cannot be redistributed or resold, even if the assets have been edited. -Credit is appreciated, but it is not necessary :)" The only difference is the trailing ':)'.»
- OK exclusiveOlive - Frogs!: alegado «Same text without the ':)'» / observado «Same text, ending with 'Credit is appreciated, but it is not necessary :)'»
- OK Kalengo Phiri - Cute Capybara: alegado «"commercial use allowed in shipped games and prototypes, modification allowed, no redistribution / resale / re-upload, no training AI on the files, no NFT / crypto use."» / observado «Same text. The page has the typo 'modificationallowed'. Name your own price, 'In development'.»
- OK Melloinc - Custom Capybara Cursor: alegado «"You may use this asset for personal and commercial purposes. Feel free to modify it to your needs. Credit is not required but would be appreciated. Please consider donating ..." + "Please DO NOT resell or redistribute this item. Thank you!"» / observado «Verbatim. $4.00+, Windows-only installer.»
- OK the14collective - Pixel Capibaras: alegado «"You may not repackage, redistribute or resell the assets, no matter how much they are modified."» / observado «"This asset pack can be used in free and commercial projects. You can modify the assets as you need. You may not repackage, redistribute or resell the assets, no matter how much they are modified. Credit is not necessary, but always appreciated." $2.00+.»
- OK SeethingSwarm - Parrotpack: alegado «"Do not resell or give away the assets individually. These assets are not to be openly distributed." Prohibits blockchain/NFT. $11.99.» / observado «"Feel free to use for commercial projects and modify the character if needed. Please use SeethingSwarm If you want to credit me. Do not resell or give away the assets individually. These assets are not to be openly distributed." + "Under no circumstances can you use any of my work, sprites, assets in any form of blockchain-related technology, which includes NFTs, P2E Game, cryptocurrency, or future inventions in the space." $11.99+.»
- OK Elthen - Parrot and Armadillo: alegado «"Feel free to use the sprites in commercial/non-commercial projects!" (armadillo adds tipping/comment). Full terms are on Patreon (403). Banner reads 'NOT FOR BLOCKCHAIN/CRYPTO RELATED PROJECT USE. READ THE LICENSE.' Public repo: ask.» / observado «Same texts on both pages. The banner image text is confirmed on both pages. Patreon licensing-27430241 still returns 403. Parrot $1.00+; armadillo name your own price.»
- OK Lara (chariart) - Jungle Birds: alegado «"Feel free to use it where you need to!" Public repo: ask.» / observado «That sentence is the only license text. The pack is free.»
- OK ChanceKite - Silly Dog Retriever and Pomeranian: alegado «"Upon purchase, this artwork can be used in personal and commercial games. Credit is appreciated. Do not sell or redistribute any part of this artwork."» / observado «Verbatim on sillydog, and the same text on pixeldog03. Both cost $30.00+.»
- OK Benvictus - Pixel Dogs: alegado «No formal license; creator comment: "Yes, feel free to use this commercially. I would ask that you put me in the credits if you are unable to donate any type of contribution."» / observado «Comment quote is verbatim. Other replies: "That's fine! Feel free to add it to your game!" and "No problem!". No redistribution clause, so a public repo remains uncertain.»
- OK Pixel Frog - Pixel Adventure (1): alegado «"Creative Commons Zero (CC0) license. You can distribute, remix, adapt, and build upon the material in any medium or format, even for commercial purposes. Attribution is not required." OpenGameArt mirror says CC-BY 4.0.» / observado «Page: "These assets are released under a Creative Commons Zero (CC0) license." followed by the distribute/remix sentence. Itch license field: 'Creative Commons Zero v1.0 Universal'. The OGA mirror, submitted by Pixel Frog, lists CC-BY 4.0, so crediting is the safe choice. Note the reference GIF actually shows Pixel Adventure 2 enemies; PA2 is $5.00+ and also CC0.»
- OK egonelbre/gophers + Go Gopher design: alegado «"The images and art-work in this repository are under CC0 license." Gopher design is CC BY 4.0 by Renee French; go.dev: "The design is licensed under the Creative Commons 4.0 Attributions license."» / observado «README is verbatim, with a LICENSE-CC0 file. The go.dev sentence is verbatim at https://go.dev/doc/gopher/README. go.dev/brand: 'The Go Gopher mascot was created by Renee French and is licensed under the Creative Commons 4.0 Attribution License.' The brand page restricts use of the Go logo and word mark.»
- OK FUJIIDEW - 8-bit Tortoise: alegado «Prohibits "Uploading the asset to other sites, Reselling this asset, Redistributing outside of projects, Editing the asset and then sharing it". Public repo: no.» / observado «Allowed: 'Use in personal projects', 'Use in commercial projects', 'Editing the asset for use in your project'. Not allowed: the 4 items as quoted. $1.00+.»
- OK LuizMelo - Pet Cats Pack: alegado «"Creative Commons Zero v1.0 Universal"» / observado «'Creative Commons Zero v1.0 Universal' plus "Credit is not required, but I would appreciate it." Name your own price.»
- OK Kenney - Emotes Pack: alegado «"Creative Commons CC0"» / observado «'Creative Commons CC0'. The preview image also says 'CC0 1.0 This content is free to use in personal, educational and commercial projects.'»
- OK Ferris (rustacean.net): alegado «"To the extent possible under law, Karen Rustad Tölva has waived all copyright and related or neighboring rights to Ferris the Rustacean."» / observado «Verbatim.»
- OK Clawd on Desk (inspiration only): alegado «"Artwork and bundled theme assets (including assets/ and themes/*/assets/) are NOT covered by AGPL-3.0. All rights reserved by their respective copyright holders." and "Clawd character is the property of Anthropic"» / observado «Verbatim. It also says 'This is an unofficial fan project, not affiliated with or endorsed by Anthropic.'»
- OK ClawdMoji (inspiration only): alegado «"no rights to the character or mark are granted"» / observado «"The Clawd character and the Anthropic spark are Anthropic, PBC's — this is an unofficial fan project, not affiliated with or endorsed by Anthropic, and no rights to the character or mark are granted." Code is MIT.»
- **DIVERGE** saint11 - Pixel Art Tutorials: alegado «"a página não declara licença" (reference only)» / observado «There is no license, but the page carries '© 2025 Pedro Medeiros', so treat it as all rights reserved. Reference-only use is correct.»
- **DIVERGE** Tine - capybara! (2tin.itch.io/capybara): alegado «Not stated in the moodboard (only US$5, 24 animations, PET EXPANSION promise)» / observado «"Personal and Commercial digital use permitted. May not be redistributed as standalone asset pack. Commercial print forbidden. AI training and blockchain (ex. crypto, NFT) forbidden. Modify as desired. Commercial derivatives must provide credit and may not be redistributed as standalone asset pack. In non-derivative form, credit appreciated but not required. Must have purchased or received key for corresponding assets and license from original distributor (TINE). License only applies to businesses with under $1M USD revenue. Others must contact me to negotiate license."»
- OK Ropuka's Idle Island (Steam): alegado «Commercial game, inspiration only» / observado «Commercial Steam game ($3.99) by Moczan and Little Chmura and Begoña Pereda. No asset license. Inspiration only is correct.»

### Correções

- Concept 5 and the Pixel Adventure pack use the wrong image. https://img.itch.zone/aW1nLzI1Mzc4MzcuZ2lm/original/OArrbk.gif shows enemies from Pixel Adventure 2: a potted plant, pig, pink bunny, chicken and duck. On the PA1 page it is a promo next to 'Download all the 20 enemy characters in Pixel Adventure 2'. PA2 is a separate pack at $5+. The image does not show the Ninja Frog. Replace it with https://img.itch.zone/aW1hZ2UvNDkwNzk4LzI1Mzk2NDguZ2lm/original/oGsPQt.gif, a PA1 screenshot GIF with the Ninja Frog in a level (verified, 1.98 MB; the frog is small in it). The itch page title is 'Pixel Adventure', not 'Pixel Adventure 1'.
- Concept 1 and the Pochi pack: mVL7QS.gif is the 'ANIMATIONS - 1' sheet. It shows Running, Idle, Sleeping, Jumping, Excited and Happy, not the Dance/Surprised/Cry/Tickle/Chilling/Box the 'why' describes. Add or swap in https://img.itch.zone/aW1nLzIwMjMxNDg2LmdpZg==/original/U2Mrsz.gif ('ANIMATIONS - 2': Cry, Tickle, Chilling, Dance, Surprised, So full) and https://img.itch.zone/aW1nLzIwMjMxNDk0LmdpZg==/original/KFG8zA.gif ('ANIMATIONS - BOX'). Both are on the cat-retro page and verified. The preview also shows 'Excited', which is not in the page's text list.
- Concept 1 risks: drop 'Só o Mochi foi visto em GIF... A Pochi foi avaliada pela página'. All four Pochi sheets were opened and viewed (Animations 1, Animations 2, Box, Furry Fury). They show the claimed animations.
- Mochi animation list: the preview sheets show these names.
- Sheet 1 (yQjbzV.gif): Sleepy, Idle, Sleeping, Dance, Excited, Idle 2
- Sheet 2 (https://img.itch.zone/aW1nLzE4OTMyMTUzLmdpZg==/original/tRDXKN.gif): Surprised, Crying, Eating, Waiting, Dead, Lay Down
- Sheet 3 (https://img.itch.zone/aW1nLzIxMzk1NzY0LmdpZg==/original/UPtmRC.gif): Shy, Refusing, Angry
- Sheet 4 (bo6BCp.gif): Licking, Despite
- Sheet 5 (https://img.itch.zone/aW1nLzIyNDgzNTM2LmdpZg==/original/ml5Sr4.gif): Sick 1, Sick 2
- Box sheet: 3 box poses
'Play' does not appear on any of these sheets; remove it or mark it unverified. Update the Mochi mapping: needs-attention → Surprised; error → Crying, Angry or Sick (Despise as a joke); done-small → Excited; working → Eating or Waiting.
- Concept 1 license_path: 'tem só 2 animações' comes from the Mochi page ('FreePack has only 2 animations.'). The Pochi page does not say how many animations its free RetroCatsFree.png.zip has. Attribute the sentence to Mochi.
- Concept 1, third reference: bo6BCp.gif is in the main body of the cat-pack page, not only in a devlog. The label reads 'DESPITE' (the author's spelling). Devlog text: 'UPDATE 2 JUL 2025: I added 2 new cat animations, licking and despite face.'
- Concept 1 art_strategy: add the URLs the moodboard does not give.
- Cat Mega Bundle: https://toffeecraft.itch.io/cat-mega-bundle (paid file 'if you pay $2.40 USD or more', 5.0 stars, 31 ratings)
- Pet Mobile Pixel Asset Pack: https://toffeecraft.itch.io/pet-virtual-mobile-pixel-asset ($1.80+; cats with dancing and surprising, dogs with bark; also 5 parrot colors)
- Concept 6: 'a dança... está num canvas de 256x256' is wrong.
- animation/dance.ase is 64x64 with 8 frames, so gopher-dance-long-3x.gif is 192x192.
- morning-coffee.ase is 64x64 with 10 frames.
- party.gif and eyeroll-3x.gif are 96x96 head-only icons from 32x32 sources (eyeroll.ase has 14 frames).
The real normalization problem is mixing 64x64 full-body animations with 32x32 head-only icons.
- Concept 6 animation_coverage: add these assets.
- 'trabalhando': icon/typing-furiously (32x32, 13 frames). Viewed preview: https://raw.githubusercontent.com/egonelbre/gophers/master/.thumb/icon/typing-furiously-3x.gif, a gopher typing furiously.
- Static sleep emoji: https://raw.githubusercontent.com/egonelbre/gophers/master/icon/emoji/gopher-sleeping.png ('zz').
- aww.gif and ninja-3x.gif also exist under .thumb/animation.
The 'Falta' list shrinks: sleep only needs animating from that emoji.
- Concept 6: drop 'É a única arte desse nível que pode ir para um repo público só com créditos'. Pixel Frog's Pixel Adventure (CC0 on itch), LuizMelo Pet Cats (CC0) and Kenney Emotes (CC0) can also go in a public repo, with no credit required.
- Concept 4: pixeldog03 also costs $30.00+, so it is not a cheaper sample; only its free preview GIF helps judge the artist. The exact page title is 'Pixel Dog Pomeranian "Orange" (SideScrolling)'.
- Concept 4 style_direction: '11x26px de frente, 23x19px de lado' is not on the page. The page gives frame sizes of 24x48 for front/back and 48x48 for side; mark the smaller numbers as an estimate or remove them. The coat name is 'Chocolate', not 'Choc'. Drop 'ninguém viu os GIFs ainda': the demo GIF was viewed. It shows a golden dog that is very small in a 450x300 grass scene, which supports the 'pode parecer miúdo' risk.
- Concept 5 risks: drop 'Ninguém viu o GIF do sapo da exclusiveOlive'. The giphy GIF was viewed: it is the 'Frogs' title card with 4 side-view frogs (green, red, teal, brown). The 'Rate' GIF warning is confirmed: dJpcrM.gif is 5 sad-faced stars. Still unverified: whether Grapple is the tongue, and how many frames the leaf glider has.
- Frogs ready-made pack: jltkOD.png is a static 315x249 copy of the same 'Frogs' title card. Prefer the animated giphy URL from concept 5 as the clickable preview.
- Concept 3 / Parrotpack: the SeethingSwarm reference (ViS5SM.png) is a static PNG cover, not an animation. The page's rating is 3.0 stars (2 ratings). It does show scarlet macaws, as claimed.
- Concept 3: the Cute Parrots page says '21 different animations', but the list repeats Hurt, so there are 20 unique names. The moodboard's list of 20 is right. The 3 colors are not named on the page; the demo GIF's first frame shows a red macaw-type parrot.
- Concept 2: the14collective pack costs $2.00+ (missing from the moodboard). Its full license also says 'Credit is not necessary, but always appreciated.' The Kalengo GIF (nA3Up1.gif) shows only a side-view walking capybara. Party, Happy and the front view come from the page text, not from that image.
- Concept 2 alternative (Tine, 2tin.itch.io/capybara): add the license, which the moodboard omits: "Personal and Commercial digital use permitted. May not be redistributed as standalone asset pack. ... Commercial derivatives must provide credit ... Must have purchased or received key for corresponding assets and license from original distributor (TINE). License only applies to businesses with under $1M USD revenue." Also forbids AI training, blockchain and commercial print. Treat it like the other paid packs: private only, never in a public repo. Page title is 'capybara!' by Tine; side view; 16x16 and 32x32.
- Concept 7: saint11 is not 'sem licença declarada'. The page carries '© 2025 Pedro Medeiros' (all rights reserved), so link to it only and do not copy it. The page itself does not mention Celeste.
- Concept 8: the Clawdbot claim is confirmed by https://en.wikipedia.org/wiki/OpenClaw. It was renamed Moltbot on Jan 27, 2026 'following trademark complaints by Anthropic', then OpenClaw on Jan 30, 2026. Cite this instead of 'segundo a pesquisa'.
- Pixel Adventure license_path: the exact itch sentence is 'These assets are released under a Creative Commons Zero (CC0) license.' The OpenGameArt mirror is https://opengameart.org/content/pixel-adventure-1 (CC-BY 4.0, submitted by Pixel Frog). Keep the 'credit Pixel Frog to be safe' advice.
- Process note, not a moodboard correction: WebSearch was unavailable (session budget of 200/200 used), so the replacement URLs were found from the source pages and the GitHub API. WebFetch automatically saved binary copies of the image URLs it fetched into the Claude session's tool-results cache (~/.claude/projects/.../tool-results/). Those copies were only opened with Read to view the first frame. No project files were created or changed.
