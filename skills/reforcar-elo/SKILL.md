---
name: reforcar-elo
description: "Busca incessante de provas para subir a nota de um elo: colaterais, inventários, dispensas, citações e o que mais houver. Use quando um elo está fraco."
---

# Reforçar elo

Sobe a nota de um elo percorrendo uma **fila** de estratégias, da que mais rende à que menos rende, até a nota-alvo ou a regra de parada. As técnicas, a ordem delas e a regra de parada são do guia de [estratégias](../../docs/guias/estrategias.md): abra-o antes de montar a fila.

**Argumentos:** o elo (um filho e o seu pai ou a sua mãe, por nome, por ID ou pela linha de `pesquisa/linhagens.md`) e a nota-alvo. Sem nota-alvo no pedido, vale a nota mínima do perfil de prova; sem ela, A.

## Passos

1. **Abra** pela [abertura comum](../../docs/skills/abertura.md).
2. **Fixe o alvo.** A nota-alvo vale para a filiação e para a identidade. O elo sem nota no estado é graduado antes, pelos passos da skill `validar-linhagem`, dos documentos às decisões do pesquisador. Pronto quando o estado traz as duas notas em vigor e a nota-alvo está no relatório da conversa.
3. **Leia o que já está baixado ou anexado**, antes de qualquer busca nova: é a estratégia que mais rende.
   - os documentos do estado sem a etiqueta [V];
   - as imagens e os textos de `pesquisa/imagens/` que a abertura deu como não lidos;
   - a imagem de cada fonte anexada ao filho e ao pai ou à mãe, listada por `garimpo fs fontes <ID>`.

   Leia pelos passos da skill `ler-acervo`, dos alvos à cobertura, e gradue o elo de novo pela [régua](../../docs/guias/regua-de-prova.md) a cada leitura. Pronto quando cada documento da pasta e cada fonte anexada está lido ou tem a pendência de acesso anotada.
4. **Monte a fila** em Próximos passos do estado. Entram:
   - o mapa da família colateral, no topo: a rota pelos [colaterais](../../docs/guias/estrategias.md#colaterais-passo-a-passo) corre junto com a direta, e o mapa dá os nomes que as outras técnicas buscam;
   - as pistas abertas do elo com acesso online;
   - as técnicas da tabela do problema do elo, em [Por tipo de problema](../../docs/guias/estrategias.md#por-tipo-de-problema), na ordem da tabela, que é a de rendimento. A linha da época e da região diz por onde começar e o que deixar para o fim;
   - no fim, as técnicas de [Baixo rendimento](../../docs/guias/estrategias.md#baixo-rendimento-nestes-casos) cuja condição vale para este elo.

   Cada item diz o documento procurado, onde ele estaria pela jurisdição da época ([fontes](../../docs/guias/fontes.md)) e a nota que daria. Fica fora da fila o que a cobertura dá como `lido` ou `negativo`; `parcial` e `pendente-de-acesso` entram.

   Pronto quando cada técnica da tabela do problema está na fila ou tem na cobertura o resultado que a tirou dela.
5. **Percorra a fila** do topo, um item por vez:
   1. Busque com o `garimpo`, com `--cobertura pesquisa/cobertura.jsonl`. A busca feita fora dele ganha uma linha em Cobertura fora do `garimpo`, no estado.
   2. Leia o que a busca achar, pelos passos da `ler-acervo`.
   3. Registre o resultado do item: achado, negativo ou pendente de acesso. O item bloqueado continua na fila: peça o desbloqueio ao usuário uma vez, siga com o seguinte e volte a ele quando o acesso abrir. Cada indício de um documento ainda não lido vira pista.
   4. Gradue o elo de novo e reordene a fila: um achado abre técnicas e nomes novos.

   Pronto quando vale uma das linhas de [Quando parar](#quando-parar) e cada item percorrido tem o resultado na cobertura.
6. **Feche** pelo [fechamento comum](../../docs/skills/fechamento.md).

## Quando parar

Confira a tabela depois de cada graduação, a começar pela do passo 2: a linha que valer encerra os passos que faltam.

| Situação | O que fazer |
|---|---|
| A filiação e a identidade chegaram à nota-alvo | Feche |
| A [regra de parada](../../docs/guias/estrategias.md#regra-de-parada) está confirmada pelas duas condições dela | Só então vem o pedido em papel: siga [Depois de parar](../../docs/guias/estrategias.md#depois-de-parar) e redija-o com a skill `pedido` |
| A fila acabou e nenhum item está pendente de acesso | Feche com a nota que ficou e o que a subiria |
| Só restam itens pendentes de acesso | O elo continua aberto: feche pedindo ao usuário cada desbloqueio, com a fila em Próximos passos |

## Threads

Duas estratégias da fila são independentes quando nenhuma espera o resultado da outra. Abra uma thread para cada uma quando as duas pedem a leitura de um acervo inteiro (um filme, o índice de um fundo) em fontes diferentes. Na mesma conta do FamilySearch os pedidos saem um por vez: duas threads ali só se revezam.

Cada thread nasce pelo procedimento de [Delegação](../../docs/skills/abertura.md#delegação) e roda a skill `ler-acervo` sobre o acervo da sua estratégia. O briefing dela diz o que muda no fechamento: a thread não escreve no estado do elo. Os achados e a cobertura dela ficam no relatório e no arquivo de leitura dela, e as pistas, um arquivo cada, em `pistas/`. O estado é só desta conversa, que relê o relatório de cada thread, leva ao estado os documentos, a cobertura e as notas, e retoma o passo 5.
