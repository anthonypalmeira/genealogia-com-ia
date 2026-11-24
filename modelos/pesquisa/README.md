# Modelo da pasta de pesquisa

A pesquisa de cada usuário fica numa pasta dele, fora deste repositório. Este modelo diz o que a pasta guarda e onde, para que qualquer conversa retome um elo lendo só a pasta. Os outros arquivos desta pasta são os modelos a copiar.

No kit, a pasta é `pesquisa/`: um link na raiz do repositório, ignorado pelo git, para a pasta do usuário.

## O que a pasta guarda

| Caminho | Conteúdo |
|---|---|
| `perfil.md` | Pessoa raiz, objetivo e perfil de prova, relatório de referência e idioma. Configurado uma vez |
| `linhagens.md` | **Fonte única** das linhagens: que elos formam cada uma, em ordem |
| `elos/<elo>/estado.md` | O estado datado do elo e a **fonte única** das notas dele. No topo, `filiacao` e `identidade` são as notas em vigor, na escala da régua: a decisão do pesquisador, quando há; senão, a nota da régua |
| `elos/<elo>/pistas/<pista>.yaml` | Uma pista por arquivo, no [formato da régua](../../docs/guias/regua-de-prova.md#formato-de-registro) |
| `elos/<elo>/recortes/` | Recortes dos documentos lidos |
| `imagens/<filme>/` | As imagens baixadas de um filme, que servem a vários elos, com o manifesto. Num repositório git, só o manifesto é versionado |
| `cobertura.jsonl` | O registro de buscas do `garimpo`, um só para a pesquisa, porque a mesma busca serve a vários elos. Passe `--cobertura pesquisa/cobertura.jsonl` a toda busca |
| `varreduras/` | Varreduras da árvore, para não refazer |
| `relatorios/<AAAA-MM-DD>-<assunto>.md` | O relatório de cada conversa |

A cobertura de um elo tem duas partes: as buscas do `garimpo`, que `garimpo cobertura consultar` responde pelo nome das pessoas, e a seção Cobertura fora do `garimpo` do estado, com o que foi buscado em outros acervos e lido página a página.

## Nome da pasta de um elo

`<filho>--<pai-ou-mae>`, com os nomes completos em minúsculas, sem acentos e com hífens: `belarmino-vidigal--honorato-vidigal`. Homônimos levam o ano de nascimento: `belarmino-vidigal-1820--honorato-vidigal`.

## Montar a pasta

1. Pergunte ao usuário onde a pasta fica, fora do kit, e crie-a. Na raiz do kit, rode `ln -s <caminho da pasta> pesquisa`.
2. Copie [perfil.md](perfil.md) e [linhagens.md](linhagens.md) para a pasta.
3. Preencha o perfil com o que o pedido já disse e pergunte o resto numa mensagem só.

Pronto quando `pesquisa/perfil.md` não tem mais nenhum campo entre `<` e `>`.

## Abrir um elo

1. Crie `pesquisa/elos/<elo>/` e copie [estado.md](estado.md) para lá. As notas começam em ∅.
2. Ponha o elo na linhagem dele em `pesquisa/linhagens.md`.

Pronto quando a linha de `linhagens.md` aponta para uma pasta com `estado.md`.
