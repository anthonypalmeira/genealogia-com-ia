# Abertura comum

Os passos com que toda skill do kit começa, antes de qualquer busca.

1. **Perfil.** Leia `pesquisa/perfil.md`; sem ele, [monte a pasta](../../modelos/pesquisa/README.md#montar-a-pasta). A pessoa raiz é o ponto de partida quando o pedido não dá outro. O perfil de prova ajusta a régua. O relatório de referência é onde se confere se um achado já está no relatório. O idioma vale para as respostas e para os arquivos salvos. Pronto quando o perfil está lido e não tem campo por preencher.
2. **Login.** Rode `garimpo sessao conferir` e trate a falta de sessão pela regra de [Sessão e acesso](../guias/leitura-e-operacao.md#sessão-e-acesso). Pronto quando a sessão vale ou o pedido de login está feito.
3. **Estado.** Ache os elos do pedido em `pesquisa/linhagens.md`; o elo que falta ali, [abra](../../modelos/pesquisa/README.md#abrir-um-elo). Para cada elo, responda às cinco perguntas da retomada:

   | Pergunta | Onde está a resposta |
   |---|---|
   | O que já temos? | `estado.md`: as notas, os documentos, a decisão do pesquisador. As pistas em `pistas/` |
   | O que foi buscado e deu negativo? | `garimpo cobertura consultar <grafia> --cobertura pesquisa/cobertura.jsonl`, para cada grafia do nome de cada pessoa do elo, e a seção Cobertura fora do `garimpo` do estado |
   | O que não foi lido? | As linhas `parcial` dessas duas fontes, os documentos do estado sem a etiqueta [V] e as imagens de `pesquisa/imagens/` que nenhuma linha de cobertura dá como lidas |
   | O que está pendente de acesso? | As linhas `pendente-de-acesso` dessas duas fontes |
   | Que conversas estão abertas? | A seção Conversas abertas do estado |

   Procure também cada grafia no resto de `pesquisa/`: outra conversa pode ter achado a pessoa em outro elo. Pronto quando cada elo tem as cinco respostas, nem que seja "nada".
4. **Delegação.** Reparta o trabalho pela regra abaixo.

## Delegação

Decida sozinho, com o estado retomado:

| Trabalho | Dono |
|---|---|
| Dois ou mais elos independentes, cada um pedindo leitura ou busca | Uma thread nova por elo |
| Mais de ~20 páginas de imagem | Subagentes, em faixas sem sobreposição: [Delegar a leitura](../guias/leitura-e-operacao.md#delegar-a-leitura) |
| O resto | A conversa principal |

Dois elos são independentes quando nenhum espera o resultado do outro.

Para cada thread nova:

1. Anote a conversa em Conversas abertas do estado do elo.
2. Abra a thread com a skill a rodar e a pasta do elo. O estado já está na pasta: o briefing não o repete. Onde o agente não abre threads, use um subagente em segundo plano com o mesmo briefing.
3. A thread termina pelo próprio fechamento. A conversa principal relê só o estado de cada elo.

Pronto quando cada elo e cada faixa de páginas tem um dono.
