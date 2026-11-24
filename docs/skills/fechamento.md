# Fechamento comum

Os passos com que toda skill do kit termina, sem o usuário pedir.

1. **Achados.** Cada achado diz que elo prova ("X é filho de Y") e a nota antes e depois. Procure o documento no relatório de referência do perfil e marque o achado como "já no relatório" ou "novo", na resposta e na coluna "No relatório" do estado. Pronto quando todo achado tem o elo e a marca.
2. **Gravações.** Proponha as gravações no FamilySearch numa tabela só, pela receita de [Gravação](../guias/familysearch.md#gravação):

   | Pessoa | Ação | Antes → depois | Motivo |
   |---|---|---|---|
   | Nome completo e ID | Criar fonte, anexar fonte ou criar nota | O que a pessoa tem hoje e o que passa a ter | O que o documento prova |

   - Antes de pôr uma fonte na tabela, confira com `garimpo fs fontes <ID>` que a pessoa ainda não a tem.
   - Abaixo da tabela, mostre cada fonte e cada nota como vai ser gravada, pelos campos de [O que cada gravação leva](../../skills/gravar-fs/SKILL.md#o-que-cada-gravação-leva): o "sim" aprova o que foi mostrado.
   - Parentesco, fusão e fatos entram nos próximos passos, como pergunta para outra rodada.
   - O fechamento só propõe. Com o "sim" do usuário, a tabela aprovada passa à skill `gravar-fs`, que grava, relê e registra.
   - Sem o que gravar, escreva "Nada a gravar no FamilySearch".

   Pronto quando cada achado lido na imagem e cada pista nova tem uma linha na tabela ou o motivo de ficar fora.
3. **Salvar.** Grave em `pesquisa/`:
   - o relatório da conversa em `relatorios/<AAAA-MM-DD>-<assunto>.md`, com os achados, a explicação e as tabelas entregues;
   - os recortes em `elos/<elo>/recortes/`;
   - cada pista nova em `elos/<elo>/pistas/`, no [formato da régua](../guias/regua-de-prova.md#formato-de-registro);
   - o `estado.md` de cada elo tocado: a data de hoje em `atualizado`, as notas, os documentos, a cobertura e os próximos passos, com esta conversa fora de Conversas abertas.

   Pronto quando outra conversa consegue retomar cada elo lendo só a pasta.
4. **Resposta.** Responda no formato de [Responder](../guias/leitura-e-operacao.md#responder): a resposta à pergunta do usuário, os achados, a tabela de gravações, os próximos passos numerados em ordem e, na última linha, uma pergunta só. Havendo tabela, a pergunta é a aprovação dela, para que um "sim" baste. Pronto quando a resposta termina nessa pergunta.

Num subagente, faça os passos 1 e 3 e devolva os achados em texto: a tabela e a pergunta são da conversa que fala com o usuário.
