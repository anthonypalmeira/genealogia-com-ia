---
name: pedido
description: "Redige pedidos curtos a cartórios, cúrias e arquivos e acompanha as respostas. Use quando a prova só existe em papel."
---

# Pedido

Redige o pedido de um documento que só existe em papel e o acompanha até a resposta. O pedido sai daqui como **rascunho**: quem envia é o usuário. Quem guarda o quê e o formato do texto estão em [Fontes offline e como pedir](../../docs/guias/fontes.md#fontes-offline-e-como-pedir).

**Argumentos:** o documento a pedir, ou o elo que ele provaria, e o destinatário e o canal, se o usuário já os tem. Ou a notícia de um pedido já feito ("enviei", "responderam"): nesse caso, entre a abertura e o fechamento, siga só [Acompanhar](#acompanhar).

## Passos

1. **Abra** pela [abertura comum](../../docs/skills/abertura.md).
2. **Confira que é caso de pedido**, pelos três passos de [Antes de recomendar um pedido](../../docs/guias/fontes.md#antes-de-recomendar-um-pedido). Cada documento a pedir tem a sua pista, no [formato de registro](../../docs/guias/regua-de-prova.md#formato-de-registro): é nela que ficam a existência, a referência e as bases conferidas. O que o usuário já pediu está em `pesquisa/pedidos/`. O documento que ainda pode estar online volta para a skill `reforcar-elo`. Pronto quando a pista de cada documento tem `existencia`, a referência e, em `buscas`, as bases conferidas.
3. **Calcule a jurisdição e o período** de cada item.
   - **Jurisdição da época:** a paróquia, o termo, a comarca ou o bispado a que o lugar pertencia no ano do ato, com a fonte que o diz, em `onde_estaria` da pista. Dela sai a instituição que guarda o livro hoje, pela tabela [Quem guarda o quê](../../docs/guias/fontes.md#quem-guarda-o-quê), com o contato conferido no site oficial.
   - **Período:** pela regra "Para cartório" do [formato do pedido](../../docs/guias/fontes.md#formato-do-pedido), com o documento que fixa cada limite.

   Pronto quando cada item tem a instituição, a jurisdição com a fonte e o período com o documento de cada limite.
4. **Ponha o preço** de cada item e o total, com o subconjunto de melhor custo. O preço vem da tabela oficial da instituição ou do que ela informou ao usuário, com a fonte ao lado. O que ninguém confirmou entra como "a confirmar", e o pedido pergunta o orçamento. Pronto quando cada item tem o preço com a fonte ou "a confirmar", e o total soma os confirmados.
5. **Redija** pelo [formato do pedido](../../docs/guias/fontes.md#formato-do-pedido), na saída do canal: e-mail, carta em PDF ou WhatsApp para o genealogista contratado. Gere o PDF com o conversor que a máquina tiver; sem nenhum, entregue o texto da carta e diga que falta converter. Pronto quando o texto foi mostrado ao usuário na forma da saída.
6. **Registre** o pedido em `pesquisa/pedidos/<AAAA-MM-DD>-<instituição>.md`, copiado do [modelo](../../modelos/pesquisa/pedido.md), com a situação `rascunho`. A carta em PDF, quando houver, fica ao lado, com o sufixo `-carta`. A pista de cada item passa a `estado: em-pedido` e `acesso: pedido`, e os Próximos passos do elo citam o pedido. Pronto quando o arquivo do pedido tem os itens, os preços e o texto, e cada item aponta a sua pista.
7. **Feche** pelo [fechamento comum](../../docs/skills/fechamento.md). Enviar o rascunho é o primeiro dos próximos passos.

## Acompanhar

Cada notícia muda a situação no arquivo do pedido e ganha uma linha no Histórico dele:

| Notícia | Situação | O que registrar |
|---|---|---|
| O usuário enviou | `enviado` | A data e o canal |
| A instituição respondeu | `respondido` | A data e, por item: achou, não achou, orçamento ou recusa |
| Chegou tudo o que foi pedido, ou a recusa é final | `cumprido` | O que chegou e onde foi salvo |

- O documento que chega fica ao lado do arquivo do pedido, com o sufixo `-recebido-<n>` e o caminho na tabela Documentos do estado do elo. É lido pelos passos da skill `ler-acervo` e gradua o elo pela régua. A pista dele passa a `confirmada` ou `negativa`.
- "Como estão os pedidos?" se responde com uma tabela dos arquivos de `pesquisa/pedidos/`, do enviado há mais tempo ao mais novo: a instituição, os itens, a situação e a data da última notícia.

Pronto quando a situação de cada pedido tocado e a data da notícia estão no arquivo dele.
