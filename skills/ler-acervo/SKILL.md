---
name: ler-acervo
description: "Lê um livro, filme ou catálogo inteiro, página por página, depois de procurar índices. Use para documentos sem índice."
---

# Ler acervo

Lê um acervo inteiro e deixa escrito, página por página, o que foi coberto. Como baixar, recortar e ler uma imagem está no guia de [leitura de imagem](../../docs/guias/leitura-e-operacao.md): abra-o antes da primeira imagem.

**Argumentos:** o acervo e os alvos. O acervo é um filme do FamilySearch (o grupo de imagens), um documento do digitarq ou um livro de outro arquivo; [um catálogo](#um-catálogo) e [uma página só](#uma-página-só) têm roteiro mais curto. Os alvos são as pessoas procuradas e o período.

## Passos

1. **Abra** pela [abertura comum](../../docs/skills/abertura.md).
2. **Fixe os alvos.** Cada pessoa procurada, com todas as grafias do nome: as do elo e os colaterais delas. As palavras de parentesco (`filho`, `irm`, `cunhad`, `genro`, `sogr`, `herdeir`, `a rogo`). O período, com o documento que fixa cada limite. Pronto quando a lista de alvos está no relatório da conversa.
3. **Procure índices**, nos quatro lugares, antes de baixar o livro:

   | Lugar | Como procurar |
   |---|---|
   | FamilySearch | Os registros indexados, com `garimpo busca registros`. O texto completo do filme: `garimpo busca grupo <grupo> --saida pesquisa/imagens/<grupo>/texto.jsonl` e, sobre esse arquivo, `garimpo busca nomes` com cada grafia |
   | Índices de terceiros | As páginas de genealogistas e de projetos voluntários que o guia de [fontes](../../docs/guias/fontes.md) lista para a região |
   | Edições impressas | A série impressa ou a transcrição publicada do mesmo fundo, com OCR |
   | Fichas do arquivo | O catálogo e as fichas do arquivo dono. As fichas de microfilmagem e o índice do próprio livro estão no filme: aparecem no mapa do passo 4 |

   Crie antes a pasta do filme, com `mkdir -p pesquisa/imagens/<grupo>`. Passe `--cobertura pesquisa/cobertura.jsonl` às buscas do `garimpo`; as outras ganham uma linha em Cobertura fora do `garimpo`, no estado do elo.

   A varredura do livro inteiro só se troca pelas páginas que um índice aponta quando esse índice cobre o livro inteiro e acha todos os alvos. Em qualquer outro caso ela segue inteira: o acerto de um índice parcial, como o texto completo e os registros indexados, diz por onde começar, e o índice que não acha um alvo não diz que ele falta.

   Pronto quando cada um dos quatro lugares tem um resultado na cobertura: achou, com as páginas; negativo; ou pendente de acesso.
4. **Mapeie o livro pelas miniaturas:**

   ```sh
   garimpo imagens listar <grupo> --pasta pesquisa/imagens/<grupo>
   garimpo imagens miniaturas <grupo> --pasta pesquisa/imagens/<grupo>
   garimpo imagens folha <miniaturas da folha, em ordem> --saida pesquisa/imagens/<grupo>/folha-<de>-<ate>.jpg
   ```

   Leia as folhas e escreva o mapa em `pesquisa/imagens/<grupo>/mapa.md`, uma linha por faixa de imagens: capas e fichas, onde cada livro começa e acaba, a ordem, as lacunas e as páginas repetidas. Date cada livro pelos cabeçalhos de algumas páginas, lidos na resolução original. As faixas a ler são todas as do acervo pedido, a começar pelas que os índices e o período apontam; só o índice que cumpre a regra do passo 3 as reduz.

   - As fichas de microfilmagem e o índice do próprio livro, quando o mapa os mostra, se leem antes das faixas: são índice, pela mesma regra do passo 3.
   - Uma folha leva 54 miniaturas nas 6 colunas do padrão. Folhas também são imagens: mais de ~20 vão para subagentes.
   - As miniaturas contam no teto da sessão. Se o filme não cabe no teto junto com as páginas a baixar, mapeie por amostras, com `--de` e `--ate`: uma folha por trecho, e outras só para fechar onde um livro acaba.
   - Fora do FamilySearch não há miniaturas: liste o documento (no digitarq, `garimpo digitarq imagens <ID>`), baixe-o inteiro pelo passo 5 e monte as folhas com as páginas.

   Pronto quando cada imagem do filme cai numa faixa do mapa e as faixas a ler estão escolhidas.
5. **Baixe as faixas a ler**, um `baixar` por vez na conta:

   ```sh
   garimpo imagens baixar <grupo> --de <N> --ate <N> --pasta pesquisa/imagens/<grupo> --paralelo <N>
   garimpo digitarq baixar <ID> --pasta pesquisa/imagens/<ID> --cobertura pesquisa/cobertura.jsonl
   ```

   - Saída 4 ou 5: leia o que já veio. O mesmo comando retoma quando a pausa acabar ou o usuário decidir sobre o teto.
   - Páginas com o mesmo `sha256` no manifesto são repetidas: leia uma.
   - O `garimpo` baixa pelo grupo e pelo número da imagem. O índice ou a fonte que só dá o ARK pede o visualizador: abra o ARK na aba logada do usuário, ou peça a ele o grupo e o número.
   - O livro de outro arquivo desce pela receita dele no guia de [fontes](../../docs/guias/fontes.md), para `pesquisa/imagens/<acervo>/`, uma imagem por página, numeradas na ordem do livro. Sem o manifesto do `garimpo`, liste no mapa (`mapa.md`) cada arquivo com o `sha256` dele, e cada página que não desceu, com a pendência.

   Pronto quando o manifesto, ou essa lista do mapa, tem uma linha por imagem das faixas, com o hash ou com a pendência.
6. **Reparta a leitura.** Até ~20 páginas, a conversa lê. Acima disso, um subagente por faixa, em faixas sem sobreposição, cada um com o briefing de [varredura](briefing.md#varredura-de-uma-faixa): o layout, a faixa, a lista de alvos e o arquivo de leitura que é só dele. Pronto quando cada página a ler tem um dono, e só um.
7. **Leia assento por assento.** Quem lê, subagente ou conversa, segue a tabela [Ler](../../docs/guias/leitura-e-operacao.md#ler) e grava um resultado por página (`lido`, `sem alvo` ou `ilegível`) no arquivo de leitura, `pesquisa/imagens/<grupo>/leitura-<AAAA-MM-DD>-<de>-<ate>.md`. O [formato de entrega](briefing.md#formato-de-entrega) define os três resultados e o que cada achado traz: o número da imagem e o texto.

   A imagem recebe só recorte, ampliação e contraste global: `garimpo imagens recortar`, com `--regiao` e `--ampliar`, e `garimpo imagens contraste`. Escrita desenhada ou recomposta nunca entra: o que o papel não deixa ler é `[ilegível]`.

   Pronto quando cada imagem das faixas tem uma linha no arquivo de leitura.
8. **Leia às cegas a palavra decisiva** de cada achado que vai ser usado: a filiação, o sobrenome, o ano. Um subagente recebe só o recorte e o layout, pelo briefing de [leitura às cegas](briefing.md#leitura-às-cegas), sem os nomes esperados, e devolve a transcrição com as leituras alternativas. Pronto quando cada achado tem uma leitura feita sem os nomes e as alternativas dela no arquivo de leitura.
9. **Releia só os achados** na conversa principal: o recorte decisivo, de 1 a 3 imagens por achado. Compare a varredura, a leitura às cegas e a sua. O que concorda ganha a etiqueta [V]; o que diverge fica registrado como divergência, com as três leituras. Pronto quando cada achado está confirmado, divergente ou ilegível.
10. **Registre a cobertura** no estado de cada elo servido, em Cobertura fora do `garimpo`: uma linha por faixa lida. O resultado é `lido` quando toda página tem `lido` ou `sem alvo`, e `parcial`, com as contagens, quando há página ilegível ou pendente de acesso. A página repetida conta pelo resultado da página a que remete. A linha aponta o arquivo de leitura. Cada página `ilegível` segue [Quando um assento não se lê](../../docs/guias/leitura-e-operacao.md#quando-um-assento-não-se-lê). Pronto quando toda página das faixas escolhidas está contada numa linha de cobertura.
11. **Feche** pelo [fechamento comum](../../docs/skills/fechamento.md).

## Um catálogo

O catálogo ou o índice de um fundo é texto: entre a abertura e o fechamento, valem só os alvos (passo 2) e a cobertura (passo 10), com uma linha pelo catálogo varrido. Baixe-o inteiro para `pesquisa/catalogos/`, criada com `mkdir -p`, e varra-o localmente com cada grafia. A colheita do digitarq é `garimpo digitarq oai`, com `--saida` nessa pasta, varrida por `garimpo busca nomes`. O documento que o catálogo acha é [Cat]: existe, e falta lê-lo.

## Uma página só

O pedido de ler uma imagem ou um assento é o mesmo roteiro, encurtado. Entre a abertura e o fechamento:

1. Fixe os alvos (passo 2) e confira na cobertura e nos relatórios se a imagem já foi lida.
2. Use a imagem que o usuário anexou, salva numa pasta de `pesquisa/imagens/` sem nome de pessoa; sem ela, baixe a página (passo 5).
3. A leitura às cegas (passo 8) transcreve inteiro o assento pedido ou, quando o pedido é a página, cada assento dela. A transcrição vai para o relatório da conversa.
4. Na releitura (passo 9), compare a leitura às cegas com a sua e com os alvos.
5. A linha de cobertura (passo 10) leva o resultado da página: `lido`, `sem alvo` ou `ilegível`.

Pronto quando a página tem o resultado e cada assento lido está confirmado ou divergente, com as duas leituras no relatório.
