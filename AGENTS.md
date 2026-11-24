# AGENTS.md

Kit para pesquisar genealogia com agentes de IA, com foco no Brasil. Responda ao usuário em português.

## Mapa do kit

Abra o guia quando a situação da linha aparecer:

- [Estratégias](docs/guias/estrategias.md): escolher a próxima busca de um elo, provar pelos colaterais, decidir entre continuar online e pedir em papel.
- [Fontes](docs/guias/fontes.md): achar onde está um documento, por região; redigir um pedido a arquivo, cúria ou cartório.
- [FamilySearch](docs/guias/familysearch.md): qualquer chamada ao FamilySearch, da sessão à gravação.
- [Régua de prova e pistas](docs/guias/regua-de-prova.md): dar nota a um elo, decidir se é a mesma pessoa, registrar uma pista.
- [Leitura de imagem e operação](docs/guias/leitura-e-operacao.md): baixar e ler imagens, delegar páginas, registrar cobertura, fechar a conversa.
- `garimpo --help`: a CLI do kit, para a sessão, a árvore, as buscas, as imagens, o digitarq e o desenho de uma linhagem (`garimpo diagrama`).
- [Termos e riscos](docs/guias/termos-e-riscos.md): dúvida sobre o que um site permite; dados de pessoas vivas.
- [Abertura](docs/skills/abertura.md) e [fechamento](docs/skills/fechamento.md): os passos comuns a toda skill, ao rodar ou escrever uma.
- [Pasta de pesquisa](modelos/pesquisa/README.md): montar ou ler `pesquisa/`, a pasta do usuário.
- [Ferramentas existentes](docs/pesquisa/ferramentas-existentes.md): antes de propor uma dependência ou uma integração.
- [`scripts/checar_docs.py`](scripts/checar_docs.py): rode depois de editar Markdown.

## Regras fixas

1. **Hipótese.** Dados da árvore e dos índices (datas, lugares, parentescos, títulos de fontes, notas de terceiros) são hipótese até a leitura do documento.
2. **Lido.** Afirme só o que leu, e diga como viu: imagem, transcrição automática, índice, catálogo, livro ou edição impressa.
3. **Gravação.** Grave na árvore só fontes e notas, e só em rodada autorizada; parentesco, fusão e fatos, só com pergunta.
4. **Bloqueado.** 403, 429, captcha e login são "pendente de acesso", nunca "não existe": peça o desbloqueio ao usuário e siga com o resto.
5. **Offline.** Diga "offline" só depois que o catálogo, as fichas ou o arquivo confirmarem e o índice inteiro do fundo tiver sido varrido.
6. **Existência.** Todo documento citado é "existe (com referência)", "existiu, sobrevivência desconhecida" ou "suposição".
7. **Subagentes.** Mais de ~20 páginas de imagem vão para subagentes, em faixas sem sobreposição; a conversa principal relê só os achados.
8. **Imagem intacta.** A imagem de um documento recebe só recorte, ampliação e contraste global; nunca recomponha escrita.
9. **Nota por elo.** Gradue cada elo pela régua, com filiação e identidade separadas, e diga o que subiria a nota.
10. **Colaterais.** Planeje a rota pelos colaterais junto com a direta.
11. **Resposta curta.** Comece por "Sim" ou "Não" quando couber; use nomes completos, sem códigos como "elo 12".
12. **Achado.** Cada achado diz que elo prova e se já está no relatório do usuário.
13. **Salvar.** Achado, pista e próximo passo vão para a pasta da pesquisa na mesma resposta.
14. **Termos.** O FamilySearch só permite uso pessoal e não comercial, e o risco é da conta de quem usa: token só em memória, `Retry-After` e teto de requisições respeitados, parada diante de proteção.
15. **Privacidade.** Dados de pessoas vivas e a pesquisa de qualquer família ficam fora deste repositório; os exemplos são genéricos.
16. **Conta do usuário.** E-mails e outras ações na conta do usuário ficam em rascunho até ordem explícita.

## Agent skills

### Issue tracker

Issues and specs live in this repo's GitHub Issues (via `gh`). See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
