# genealogia-com-ia

Ferramentas, skills e guias para pesquisar a própria genealogia com agentes de IA (Claude, Codex e outros), com foco inicial no Brasil.

## O que é

Quem pesquisa genealogia com um agente de IA perde tempo com o que não é genealogia: cada conversa começa do zero, o agente refaz buscas, lê imagens grandes demais e dá como "offline" um documento que estava online. Este kit guarda o método que funcionou em cerca de 80 sessões reais de pesquisa:

- **Instruções para agentes** ([AGENTS.md](AGENTS.md)): o mapa do kit e as regras que valem sempre. Claude e Codex leem sozinhos.
- **Guias de referência** ([docs/guias/](docs/guias/)): [estratégias](docs/guias/estrategias.md) por tipo de problema, [fontes](docs/guias/fontes.md) por região, o [FamilySearch](docs/guias/familysearch.md) em detalhe, a [régua de prova e pistas](docs/guias/regua-de-prova.md), [leitura de imagem e operação](docs/guias/leitura-e-operacao.md) e [termos e riscos](docs/guias/termos-e-riscos.md).
- **A CLI `garimpo`**, em Rust, feita para o agente usar: sessão e ritmo do FamilySearch, árvore, buscas com registro de cobertura, imagens com manifesto e retomada, o digitarq e o desenho de linhagens. `garimpo --help` explica cada grupo de comandos.
- **Skills de agente**: em construção.

O kit traz ferramentas, método e modelos. A pesquisa de cada família fica na pasta de quem pesquisa, fora deste repositório.

## Para quem é

Para quem pesquisa a própria família com a ajuda de um agente de IA e quer que ele trabalhe com método: provar cada elo com documento, achar onde o documento está e não repetir trabalho. Os guias também servem a quem pesquisa sem agente.

## Como instalar

1. Clone o repositório:

   ```sh
   git clone https://github.com/anthonypalmeira/genealogia-com-ia.git
   ```

2. Abra o seu agente (Claude Code, Codex ou outro que leia `AGENTS.md`) na pasta clonada. Ele carrega o mapa e as regras sozinho.
3. Peça a pesquisa em português. O agente consulta os guias conforme a situação.

Para a CLI, com o Rust 1.95 e o [mise](https://mise.jdx.dev) instalados:

```sh
mise install                 # as ferramentas fixadas no mise.toml
cargo install --path .       # o binário garimpo
garimpo --help
```

O desenho em SVG (`garimpo diagrama --formato svg`) é opcional e usa o `bun` e o Pele. Instale uma vez, na pasta do kit: `cd svg && bun install --frozen-lockfile`. O Mermaid e o texto não precisam de nada disso.

As skills terão as próprias instruções de instalação quando entrarem.

## Aviso: FamilySearch e os riscos de uso

Leia antes de usar o kit com a sua conta do FamilySearch:

- **Uso pessoal e não comercial.** Os termos do FamilySearch limitam o uso do site a fins pessoais e não comerciais, e proíbem download em massa e *scraping* sem permissão escrita.
- **O risco é da sua conta.** O kit usa a sessão do seu navegador e rotas que o FamilySearch não documenta para terceiros. O FamilySearch pode limitar ou suspender a conta de quem usa.
- **Teto de requisições, sem contornar proteções.** O kit respeita os limites que o site informa, tem um teto de requisições por sessão e para diante de bloqueio. Não disfarça o agente nem contorna captcha ou proteção anti-robô.
- **Dados de pessoas vivas ficam fora.** Nada de nomes, documentos ou dados de pessoas vivas no repositório, nos exemplos ou nos testes.
- **O token nunca vai para o disco.** A sessão do FamilySearch fica só em memória.
- **O agente envia o que lê ao provedor do modelo.** Os termos também restringem transmitir conteúdo do site a terceiros: usar um modelo hospedado é um risco a mais.

Os detalhes e as fontes estão em [termos e riscos](docs/guias/termos-e-riscos.md). Este projeto não tem parceria com o FamilySearch nem é certificado por ele.

## Contribuir

Depois de editar Markdown, rode a checagem, que confere os links entre documentos, o frontmatter das skills e a ausência de CPF, e-mail e telefone:

```sh
python3 scripts/checar_docs.py
python3 -m unittest discover -s scripts
```

Os testes da CLI rodam o binário contra um FamilySearch e um digitarq falsos, locais: nenhum deles chama um serviço real.

```sh
(cd svg && bun install --frozen-lockfile && bun run typecheck)
cargo nextest run --workspace --all-features --locked
```

O **teste real** é à parte: só leituras, com a sua sessão, sob demanda e nunca na integração contínua. Serve para saber quando o FamilySearch muda uma rota. Com a sessão entregue e a ponte aberta, rode `scripts/real_test.sh`; ele sai com 3 quando falta o login.

Contribuições com dados de uma família específica não são aceitas: use exemplos genéricos.

## Licenças

- **Código:** [MIT](LICENSE).
- **Guias** em `docs/guias/`: [Creative Commons Atribuição 4.0 Internacional (CC BY 4.0)](LICENSE-CC-BY-4.0).

Créditos:

- O compilador de diagramas de linhagem (`garimpo diagrama`) é inspirado na skill `statechart` do [pstack](https://github.com/cursor/plugins/tree/main/pstack/skills/statechart), de Lauren Tan, distribuído sob a licença MIT: a mesma ideia de ler uma descrição, passá-la a uma forma intermediária, checar as regras e marcar no desenho o que as quebra. Nenhum código foi copiado.
- O SVG é desenhado pelo [Pele](https://pele.run), de kepano, sob a licença MIT, usado como dependência na versão 1.13.0.
- Os endereços das imagens no armazenamento do FamilySearch vêm das notas do [ianderso/familysearch-mcp](https://github.com/ianderso/familysearch-mcp), sob a licença MIT.
