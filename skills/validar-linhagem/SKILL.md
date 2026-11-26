---
name: validar-linhagem
description: "Gradua cada elo de uma linhagem pela régua de prova e aponta os mais fracos. Use para 'qual a nota?' ou 'quão forte é esta linhagem?'."
---

# Validar linhagem

Dá nota a cada elo de uma linhagem com o que já foi lido. Ler o que falta e buscar documento novo é da skill `reforcar-elo`.

Abra a [régua de prova](../../docs/guias/regua-de-prova.md) antes de graduar: a escala, as etiquetas de leitura, a nota por tipo de documento e as âncoras de identidade estão lá.

**Argumentos:** uma linhagem de `pesquisa/linhagens.md`; ou as pessoas de uma linhagem nova, em ordem, da mais nova à mais antiga; ou um elo só, que é uma linhagem de um elo. Para "da pessoa X ao ancestral Y" sem as pessoas do meio, ache antes o caminho com a skill `caminhos`.

## Passos

1. **Abra** pela [abertura comum](../../docs/skills/abertura.md).
2. **Fixe os elos.** Uma linhagem nova entra em `pesquisa/linhagens.md`, com um elo por par de pessoas vizinhas. Pronto quando cada elo tem a sua linha em `linhagens.md` e a sua pasta com `estado.md`.
3. **Junte os documentos de cada elo** na tabela Documentos do estado, cada um com a etiqueta de como foi visto:
   - o que a pasta do elo e os relatórios de conversas anteriores já trazem;
   - os documentos de cada elo no relatório de referência e na pasta de provas indicada no perfil. Copie a referência, o trecho e a etiqueta de leitura que o relatório ou o índice da pasta dá, sem reler os documentos. Em **Onde está**, indique a origem da leitura e o arquivo local, quando houver. Marque **No relatório** como `sim` só no documento que o relatório de referência traz;
   - as fontes anexadas ao filho e ao pai ou à mãe: `garimpo fs fontes <ID>`. O título de uma fonte é hipótese: a régua diz o que vale uma fonte que ninguém leu na imagem;
   - as notas das duas pessoas: `garimpo fs notas <ID>`. Nota de outro usuário é pista, e vai para `pistas/`.

   Pronto quando cada documento da pasta do elo, do relatório de referência, da pasta de provas e cada fonte anexada está na tabela, com a etiqueta, e cada nota de outro usuário virou pista.
4. **Gradue a filiação** de cada elo: o documento declara que o filho é filho daquele pai ou daquela mãe? Aplique as quatro perguntas da régua, a nota do tipo de documento com o que sobe e o que desce, e o perfil de prova. Pronto quando cada elo tem a nota de filiação, o documento que a sustenta e o que subiria a nota.
5. **Gradue a identidade** de cada elo, à parte: o filho que o documento deste elo nomeia é a pessoa que o elo de baixo dá como pai ou mãe? No primeiro elo, é a pessoa de partida? Registre as âncoras na seção Identidade do estado. Pronto quando cada elo tem a nota de identidade e as âncoras que a sustentam.
6. **Aplique as decisões do pesquisador**, pela regra de [Decisão do pesquisador](../../docs/guias/regua-de-prova.md#decisão-do-pesquisador). A decisão já registrada no estado é a nota em vigor, mostrada ao lado da nota da régua. A nota que o usuário dá nesta conversa e que diverge da régua entra na seção Decisão do pesquisador do estado. Pronto quando toda divergência entre a régua e o pesquisador está no estado do elo.
7. **Entregue** uma tabela com uma linha por elo, do mais novo ao mais antigo:

   | Filho → pai ou mãe | Filiação | Identidade | Documento que sustenta cada nota | O que subiria a nota |
   |---|---|---|---|---|
   | Nomes completos | A nota em vigor, e a da régua quando divergem | Idem | Referência, etiqueta e origem da leitura ("leitura antiga", com o relatório ou arquivo, ou "leitura desta conversa") | O documento a ler ou a achar |

   Depois, os **elos mais fracos**: os de nota mais baixa na escala da régua, de filiação ou de identidade, com o que subiria cada um. Num elo F, diga qual documento o contradiz.

   Pronto quando a tabela tem uma linha por elo e a resposta nomeia os elos mais fracos.
8. **Feche** pelo [fechamento comum](../../docs/skills/fechamento.md).
