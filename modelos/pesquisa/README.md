# Modelo da pasta de pesquisa

A pesquisa de cada usuário fica numa pasta dele, fora deste repositório. Este modelo diz o que a pasta guarda e onde, para que qualquer conversa retome um elo lendo só a pasta. Os outros arquivos desta pasta são os modelos a copiar.

Para as skills, a pasta é `pesquisa/`: um link, na pasta em que a conversa roda, para a pasta do usuário. No clone do kit, o git ignora o link.

## O que a pasta guarda

| Caminho | Conteúdo |
|---|---|
| `perfil.md` | Pessoa raiz, objetivo e perfil de prova, relatório de referência e idioma. Configurado uma vez |
| `linhagens.md` | **Fonte única** das linhagens: que elos formam cada uma, em ordem |
| `elos/<elo>/estado.md` | O estado datado do elo e a **fonte única** das notas dele. No topo, `filiacao` e `identidade` são as notas em vigor, na escala da régua: a decisão do pesquisador, quando há; senão, a nota da régua |
| `elos/<elo>/pistas/<pista>.yaml` | Uma pista por arquivo, no [formato da régua](../../docs/guias/regua-de-prova.md#formato-de-registro) |
| `elos/<elo>/recortes/` | Recortes dos documentos lidos |
| `imagens/<filme>/` | As imagens baixadas de um filme, que servem a vários elos, com o manifesto, as miniaturas e as folhas de contato, o texto puxado (`texto.jsonl`), o mapa do filme (`mapa.md`) e as leituras página a página (`leitura-<AAAA-MM-DD>-<de>-<ate>.md`). Num repositório git, as imagens ficam fora |
| `cobertura.jsonl` | O registro de todas as buscas da pesquisa, inclusive as feitas fora do `garimpo`. Uma mesma busca pode servir a vários elos. Veja [Registrar na cobertura](#registrar-na-cobertura) |
| `alteracoes.jsonl` | O registro do que o `garimpo` gravou no FamilySearch. Passe `--registro pesquisa/alteracoes.jsonl` a toda gravação |
| `pedidos/<AAAA-MM-DD>-<instituição>.md` | Um pedido a um cartório, uma cúria ou um arquivo, com a situação e o histórico, copiado de [pedido.md](pedido.md). A carta em PDF e cada documento que chegar ficam ao lado, com o mesmo nome e os sufixos `-carta` e `-recebido-<n>` |
| `catalogos/` | Catálogos e índices de fundos baixados inteiros, para varrer localmente |
| `varreduras/` | Varreduras da árvore, para não refazer, e os retratos das pessoas delas, em `retratos/` |
| `relatorios/<AAAA-MM-DD>-<assunto>.md` | O relatório de cada conversa. A descrição e os desenhos de um diagrama ficam ao lado, com o mesmo nome |

## Registrar na cobertura

Toda busca do `garimpo` leva `--cobertura pesquisa/cobertura.jsonl` e entra sozinha. A busca feita fora dele e a leitura página a página entram com:

```sh
garimpo cobertura registrar "<onde>" "<consulta>" --resultado <resultado> --cobertura pesquisa/cobertura.jsonl
```

A consulta leva o que foi procurado e todas as grafias de cada pessoa procurada: é pela grafia que a [abertura](../../docs/skills/abertura.md) acha a linha. Numa leitura página a página, leva também a faixa de imagens e o arquivo de leitura.

## Nome da pasta de um elo

`<filho>--<pai-ou-mae>`, com os nomes completos em minúsculas, sem acentos e com hífens: `belarmino-vidigal--honorato-vidigal`. Homônimos levam o ano de nascimento: `belarmino-vidigal-1820--honorato-vidigal`.

## Montar a pasta

1. Pergunte ao usuário onde a pasta fica, fora do kit, e crie-a. Na pasta em que a conversa roda, rode `ln -s <caminho da pasta> pesquisa`; se a conversa já roda dentro da pasta da pesquisa, `ln -s . pesquisa`.
2. Copie para a pasta o que ela ainda não tem de [perfil.md](perfil.md) e [linhagens.md](linhagens.md).
3. Preencha o perfil com o que o pedido já disse e pergunte o resto numa mensagem só.
4. Na mesma mensagem, pergunte que notas o pesquisador já decidiu antes de usar o kit. Para cada decisão, [abra o elo](#abrir-um-elo) se preciso e registre-a na seção Decisão do pesquisador do `estado.md`, com a data e o lugar de origem (por exemplo, relatório e página). No topo do estado, ponha a nota decidida em `filiacao` ou `identidade`, conforme o caso.

Pronto quando `pesquisa/perfil.md` não tem mais nenhum campo entre `<` e `>` e cada decisão anterior está no estado do elo.

## Abrir um elo

1. Crie `pesquisa/elos/<elo>/` e copie [estado.md](estado.md) para lá. As notas começam em ∅.
2. Ponha o elo na linhagem dele em `pesquisa/linhagens.md`.

Pronto quando a linha de `linhagens.md` aponta para uma pasta com `estado.md`.
