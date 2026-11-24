---
name: gravar-fs
description: "Grava achados no FamilySearch depois de um plano aprovado, com releitura e registro. Use quando o usuário autorizar."
---

# Gravar no FamilySearch

Leva à árvore o que a pesquisa já leu, em fontes e notas, por um **plano** único que o usuário aprova de uma vez. A árvore é compartilhada: o que entra fica público, no nome do usuário. O que vira fonte, o que vira nota e as pegadinhas das rotas estão em [Gravação](../../docs/guias/familysearch.md#gravação).

**Argumentos:** a tabela aprovada no fechamento de outra skill, ou os achados e os elos a gravar.

## Passos

1. **Abra** pela [abertura comum](../../docs/skills/abertura.md). Vindo do fechamento de outra skill na mesma conversa, repita só o passo Login.
2. **Monte o plano**, uma linha por gravação, na tabela do [fechamento comum](../../docs/skills/fechamento.md). O documento lido na imagem vira fonte em todas as pessoas que ele cita e que estão na árvore; a hipótese, o próximo passo e o pedido viram nota. Confira o que cada pessoa já tem, com `garimpo fs fontes <ID>` e `garimpo fs notas <ID>`: a linha que já está na árvore sai do plano.

   Abaixo da tabela, escreva cada fonte e cada nota como vai ser gravada, pelos campos de [O que cada gravação leva](#o-que-cada-gravação-leva).

   Pronto quando cada achado [V] e cada pista a gravar tem uma linha ou o motivo de ficar fora, e cada fonte tem uma linha por pessoa citada.
3. **Separe parentesco, fusão e fatos.** Ficam fora do plano e entram nos próximos passos do fechamento, uma pergunta por alteração ("trocar o pai de X de A para B?"), com o documento que a sustenta. São de outra rodada: o "sim" do plano não vale para elas. O `garimpo` não grava essas alterações: com o "sim" a uma delas, o usuário a faz no site do FamilySearch, e você lhe entrega o que mudar, em que pessoa e com que motivo. Pronto quando toda alteração de parentesco, fusão ou fato está nos próximos passos, fora do plano.
4. **Peça a aprovação do plano**, uma vez: a tabela, o conteúdo de cada linha e uma pergunta só. O "sim" vale para o que foi mostrado, e é a rodada. A aprovação dada no fechamento segue valendo enquanto nenhuma linha e nenhum campo mudar; o que mudou ou não foi mostrado pede novo "sim". Pronto quando o usuário disse "sim" à tabela e ao conteúdo como estão.
5. **Grave** na ordem do plano, um comando por vez, todos com a mesma `--rodada` e o mesmo `--registro`:

   ```sh
   garimpo fs criar-fonte <ID> --rodada "<AAAA-MM-DD> <resumo do plano>" --registro pesquisa/alteracoes.jsonl \
     --motivo "<motivo>" --titulo "<título>" --citacao "<citação>" --url "<link>" --nota "<transcrição>"
   garimpo fs anexar-fonte <ID> --rodada "<a mesma>" --registro pesquisa/alteracoes.jsonl \
     --motivo "<motivo>" --fonte <id da fonte>
   garimpo fs criar-nota <ID> --rodada "<a mesma>" --registro pesquisa/alteracoes.jsonl \
     --motivo "<motivo>" --assunto "<assunto>" --texto "<texto>"
   ```

   - O id da fonte é o segundo campo da linha que `criar-fonte` imprime.
   - `ja-existe` quer dizer que nada foi gravado: a linha do plano está cumprida.
   - Num erro, siga o próximo passo que ele traz. A fonte criada e não anexada se termina com `anexar-fonte`: repetir o `criar-fonte` criaria outra.
   - Parou por login, bloqueio ou teto: as linhas que faltam ficam em Próximos passos do estado, e a retomada segue da linha em que parou, com a mesma rodada.

   Pronto quando cada linha do plano tem o resultado: `anexada`, `criada`, `ja-existe` ou pendente, com o motivo.
6. **Releia e registre.** Para cada pessoa do plano, rode `garimpo fs fontes <ID> --json` e `garimpo fs notas <ID>` e compare com o plano o que a releitura traz: de cada fonte, o título, a citação, o link e a transcrição; de cada nota, o assunto e o texto. O registro é `pesquisa/alteracoes.jsonl`, que os comandos escrevem: a gravação que só tem ali a linha `"conferida": false` foi aceita e não apareceu na releitura. Pronto quando cada linha gravada confere com o plano ou está relatada como divergente.
7. **Feche** pelo [fechamento comum](../../docs/skills/fechamento.md). No passo Gravações, a tabela é o plano com o resultado de cada linha.

## O que cada gravação leva

| Gravação | Campo | Conteúdo |
|---|---|---|
| Fonte | `--titulo` | O tipo do documento, a pessoa, o lugar e o ano: "Batismo de Belarmino Vidigal, Vila Exemplo, 1820" |
| | `--citacao` | O arquivo, o fundo, o livro e a folha; o filme e o número da imagem |
| | `--url` | O endereço da **imagem** do documento: o ARK no FamilySearch ou a página da imagem no arquivo dono. É por ele que a fonte abre na imagem e que o `garimpo` reconhece a fonte que já existe |
| | `--nota` | A **transcrição** do trecho que prova, com `[ilegível]` e `[?]`, a etiqueta de leitura, e o que o documento prova e não prova |
| | `--motivo` | O que o documento prova para aquela pessoa: "prova que X é filho de Y" |
| Nota | `--assunto` | `Hipótese de pesquisa (não comprovada)`, `Próximos passos` ou `Pedido a <arquivo>` |
| | `--texto` | Na hipótese, o indício com a referência e a etiqueta de leitura, e o documento que a confirmaria. Nos próximos passos, o que falta, por ordem de utilidade. No pedido, o que foi pedido, a quem, quando e a situação |

O documento que só existe em papel ou em arquivo local ainda não tem endereço. Se os [termos do acervo](../../docs/guias/termos-e-riscos.md#outros-acervos) deixam publicar a imagem, o usuário sobe a digitalização como memória no FamilySearch e passa o endereço dela; senão, a fonte fica nos próximos passos.

Dados de pessoas vivas e do requerente ficam fora de todos os campos.
