# Ferramentas existentes para pesquisa genealógica (foco: FamilySearch, Brasil e Portugal)

Pesquisa feita em 2026-11-23, com leitura de documentação oficial, código-fonte, arquivos de licença e termos de uso. Não fiz login em nada nem testei endpoints autenticados. As poucas requisições que fiz foram anônimas e a páginas ou APIs públicas, e estão marcadas como "requisição anônima".

Convenções:
- **Documentado** significa que a afirmação está na fonte citada logo ao lado.
- **Inferência** marca uma conclusão minha.
- **(secundária)** marca uma afirmação que só tem fonte secundária.
- **"Mantida"** quer dizer que houve commit ou atualização nos últimos 6 meses, isto é, depois de 2026-05-23. Datas de commit e licenças vêm da API do GitHub (`gh api repos/...`) ou do arquivo `LICENSE`.

## Resumo

1. A API oficial do FamilySearch ([developers.familysearch.org](https://developers.familysearch.org/main/docs/getting-started)) cobre árvore, fontes, memórias, lugares, datas e escrita, mas **não tem busca de registros históricos** ("The API provides no searchability of historical records", [compatibility-review-process](https://developers.familysearch.org/main/docs/compatibility-review-process)).
2. A chave de produção exige empresa ou ONG registrada, revisão de compatibilidade e contratos assinados. A própria FamilySearch diz que **projetos pessoais, ferramentas open-source sem modelo de negócio e extensões de navegador** tendem a ser recusados (mesma página).
3. Quem tem chave não pode usar o conteúdo do FamilySearch "em conexão com IA", o que inclui mandá-lo como entrada para um modelo, sem permissão escrita ([ai-disclosure-addendum](https://developers.familysearch.org/main/docs/ai-disclosure-addendum)). Quem usa endpoints não documentados tem a chave desativada.
4. Os Termos de Uso do site proíbem download em massa, *scraping* e contornar controles sem permissão escrita, e limitam o uso a fins pessoais e não comerciais ([legal/terms](https://www.familysearch.org/en/legal/terms)).
5. Mesmo assim, já existem pelo menos 8 servidores MCP para o FamilySearch. A maioria usa a sessão do navegador e endpoints internos `/service/...`, que é o mesmo caminho que este projeto usa hoje.
6. Os melhores precedentes para copiar ideias são [cowork-genealogy](https://github.com/PioneerAIAcademy/cowork-genealogy), [ianderso/familysearch-mcp](https://github.com/ianderso/familysearch-mcp), [maxwellmezadre/familysearch-mcp](https://github.com/maxwellmezadre/familysearch-mcp) (em português, com escrita sob aprovação), [G4rutti/familysearch-mcp](https://github.com/G4rutti/familysearch-mcp) e [ex-ussr-kinship-research](https://github.com/DenisSergeevitch/ex-ussr-kinship-research).
7. Em Rust, dá para usar como dependência o [`gedcomx`](https://crates.io/crates/gedcomx) (MIT) e o [`oxidged`](https://crates.io/crates/oxidged) (MIT, GEDCOM 5.5.1/7.0/7.1). **Não há cliente FamilySearch em Rust mantido.**
8. Para HTR em português dos séculos XVI a XIX, só o **Transkribus** tem modelos prontos, e ele é proprietário e pago por crédito. Kraken, eScriptorium e Loghi são abertos, mas não têm modelo de português manuscrito pronto.
9. Em Portugal, o **digitarq** tem OAI-PMH público (oai_dc e EAD). No Brasil, **SIAN e Hemeroteca não têm API documentada**. Internet Archive e Geni têm APIs oficiais. A Data API da HathiTrust foi aposentada em 2024.
10. Lacunas: nenhuma ferramenta cobre varrer um livro de imagens respeitando o limite de taxa, ler assentos luso-brasileiros com HTR aberto, nem buscar em fontes brasileiras sem API.

## Tabela de ferramentas

Todas as linhas foram conferidas na fonte primária: repositório e `LICENSE`, documentação oficial ou página do fabricante. Onde li só os metadados e não o código, isso está dito.

### FamilySearch oficial: API, SDKs e especificações

| Nome | Tipo | Linguagem | Licença | Mantida? | Endpoints e autenticação | O que aproveitar | Link |
|---|---|---|---|---|---|---|---|
| FamilySearch API | API REST | — | Termos e contratos próprios | Sim (changelog de 2026-11-09) | `/platform/...`. OAuth 2 com *authorization code* e PKCE; `unauthenticated_session` só para lugares e datas | Modelo de dados, *throttling*, ETags, ARKs, *matches* e *hints* | [docs](https://developers.familysearch.org/main/docs/getting-started) |
| fs-js-lite | SDK | JavaScript | MIT | Sim (2026-09) | `/platform`, OAuth com PKCE, trata *throttling* | Referência de cliente fino | [repo](https://github.com/FamilySearch/fs-js-lite) |
| fs-php-lite | SDK | PHP | Apache-2.0 | Sim (2026-08) | Idem | — | [repo](https://github.com/FamilySearch/fs-php-lite) |
| gedcomx-java | Modelo e serialização | Java | Apache-2.0 | Sim (2026-10) | Não é cliente HTTP: tem só módulos de modelo e serialização (model, atom, date, fileformat e `extensions/familysearch/familysearch-api-model`) | Referência do modelo GEDCOM X e das extensões FS | [repo](https://github.com/FamilySearch/gedcomx-java) |
| gedcomx-php | SDK | PHP | Apache-2.0 (`LICENSE`) | Sim (2026-08) | — | — | [repo](https://github.com/FamilySearch/gedcomx-php) |
| familysearch-javascript-sdk | SDK | JavaScript | MIT | **Descontinuado** ("This project has been deprecated", README) | — | Nada | [repo](https://github.com/FamilySearch/familysearch-javascript-sdk) |
| GEDCOM X (especificação) | Especificação | — | Apache-2.0 | Sim (2026-04) | — | Formato canônico da árvore | [repo](https://github.com/FamilySearch/gedcomx) |
| GEDCOM 7 (especificação) | Especificação | — | Apache-2.0 | Sim (v7.0.18, 2026-02) | — | Formato de exportação local | [repo](https://github.com/FamilySearch/GEDCOM) |

### Bibliotecas em Rust

| Nome | Tipo | Linguagem | Licença | Mantida? | Endpoints e autenticação | O que aproveitar | Link |
|---|---|---|---|---|---|---|---|
| gedcomx / gedcomx_file | Biblioteca | Rust | MIT | Sim (0.1.8, 2026-06) | Não acessa rede. Serializa XML e JSON com serde e yaserde | **Dependência** para ler e escrever GEDCOM X | [repo](https://github.com/ephraimkunz/gedcomx-rs) |
| oxidged | Biblioteca e CLI | Rust | MIT | Sim (criada em 2026-10, commits em 2026-11) | — | **Dependência** para GEDCOM 5.5.1, 7.0 e 7.1 sem perda | [repo](https://github.com/trois-six/oxidged) |
| ged_io | Biblioteca | Rust | MIT | **Arquivada**, migrou para o oxidged | — | Nada | [repo](https://github.com/ge3224/ged_io) |
| rust-gedcom | Biblioteca | Rust | MIT | Não (2023) | — | Nada | [repo](https://github.com/pirtleshell/rust-gedcom) |
| familysearch-rust | Cliente | Rust | **Sem licença** | Não (2017) | — | Só mostra que não existe cliente FS em Rust mantido | [repo](https://github.com/webnard/familysearch-rust) |

### Exportadores, sincronizadores e programas certificados

| Nome | Tipo | Linguagem | Licença | Mantida? | Endpoints e autenticação | O que aproveitar | Link |
|---|---|---|---|---|---|---|---|
| getmyancestors | Exportador para GEDCOM | Python | **GPL-3.0-or-later** (`LICENSE`) | Sim (2026-08) | Faz POST de usuário e senha no formulário `ident.familysearch.org/login` e depois troca *authorization code* por token com um **client_id de terceiros**. Chama `api.familysearch.org/platform/tree/...`, `/platform/memories/...` e `/service/tree/tree-data/reservations/...` (interno). Lê 200 pessoas por lote | Ordem de chamadas e mapeamento GEDCOM X → GEDCOM. **Não copiar código** (GPL) | [repo](https://github.com/Linekio/getmyancestors) |
| genealogia-plantilla | Kit de pesquisa com GEDCOM | Python | MIT | Sim (2026-11) | Sessão "modelada no getmyancestors", com `openid` no *scope*, um regulador por `X-PROCESSING-TIME` e cache | O regulador de taxa e a regra de que nenhuma ferramenta escreve sem proposta aceita | [repo](https://github.com/victormico/genealogia-plantilla) |
| FSG_Sync, depois gramps-FamilySearchCoreIntegration | Sincronização com o Gramps | Python | FSG_Sync sem licença; o fork do Gramps é GPL-2.0 | FSG_Sync parado ("API Key is no longer valid"); o fork é ativo (2026-11) | Chave de app própria | Mostra que uma chave pode ser revogada | [FSG_Sync](https://github.com/SourceAirbender/FSG_Sync), [core](https://github.com/SourceAirbender/gramps-FamilySearchCoreIntegration) |
| PersonFS | *Gramplet* para o Gramps | Python | GPL-3.0 | Arquivado, migrou para `gramps-kromprogramoj` | — | — | [repo](https://github.com/jmichault/PersonFS) |
| Gramps | Programa de árvore | Python | GPL | — | Aparece como "FamilySearch Compatible: Family Tree" na galeria. A página do Gramps sobre isso respondeu 403 (Cloudflare) | Único certificado open-source que encontrei | [galeria](https://www.familysearch.org/en/innovate/solutions-gallery) |
| RootsMagic | Programa de árvore | — | Proprietário | Na galeria | API oficial, certificado | Fluxo de sincronização (inspiração de interface) | [galeria](https://www.familysearch.org/en/innovate/solutions-gallery), [site](https://rootsmagic.com/) |
| Ancestral Quest | Programa de árvore | — | Proprietário | Na galeria | Idem | Idem | [site](https://www.ancquest.com/index.htm) |
| Legacy Family Tree | Programa de árvore | — | Proprietário | Na galeria (Family Tree, GEDCOM 7) | Idem | — | [site](https://legacyfamilytree.com/Index.asp) |
| Family Historian | Programa de árvore | — | Proprietário | Na galeria (Family Tree, GEDCOM 7, User Tree) | Idem | — | [site](https://www.family-historian.co.uk/tour) |
| MacFamilyTree | Programa de árvore | — | Proprietário | Na galeria | Idem | — | [site](https://www.syniumsoftware.com/macfamilytree) |
| AncestralFire | Serviço web com IA | — | Proprietário, *freemium* | Na galeria | Login pelo FamilySearch; diz que "Nothing changes on FamilySearch until you click Approve" e que funciona dentro de Claude, ChatGPT e Gemini | Prova de que um app certificado com IA existe e de que a escrita pode ficar sob aprovação | [site](https://ancestral-fire.com/) |
| TreeMonk, Family Tree Validator, Puzzilla, Relative Finder | Visualizadores e verificadores | — | Proprietários | Na galeria | API oficial | Ideias de relatórios: descendência, erros, lacunas | [galeria](https://www.familysearch.org/en/innovate/solutions-gallery) |

### Extensões, baixadores de imagens e rastreadores de coleções

| Nome | Tipo | Linguagem | Licença | Mantida? | Endpoints e autenticação | O que aproveitar | Link |
|---|---|---|---|---|---|---|---|
| WikiTree Sourcer | Extensão de navegador | JavaScript | MIT | Sim (2026-11) | Lê o `fssessionid` do navegador e o envia como `Authorization: Bearer` a `www.familysearch.org/service/tree/links/sources/...` e ao JSON do registro (`fs_fetch.mjs`) | Montagem de citações a partir de um registro do FS e de mais de 100 sites | [repo](https://github.com/RobPavey/wikitree-sourcer) |
| familysearch-image-downloader (nikborovets) | Baixador de imagens | Python | **Sem licença** | Sim (2026-11) | `fssessionid` do navegador, *tiles* DeepZoom (`.../deepzoomcloud/dz/v1/{ark}/image_files/`) e grupos via `sg30p0.../das/v2/{group}`. Pausa de 0,12 a 0,35 s com variação aleatória | Retomada, gravação atômica, entrada em lote. Só como inspiração (sem licença) | [repo](https://github.com/nikborovets/familysearch-image-downloader) |
| familysearch-image-downloader (thefirans) | Baixador (Streamlit) | Python | **Sem licença** | Sim (2026-06) | Lê os cookies do Chrome, Edge ou Brave | — | [repo](https://github.com/thefirans/familysearch-image-downloader) |
| familysearchDwHTR | Baixador de filme com HTR | Python | GPL-3.0 | Não (2020) | Selenium e login no site | Precedente de "baixar e transcrever" | [repo](https://github.com/braytac/familysearchDwHTR) |
| FSFilmDownloader | Extensão do Chrome | JavaScript | Sem licença | Não ("No longer works!!!") | — | Mostra que downloaders de filme quebram | [repo](https://github.com/PieterVeldman/FSFilmDownloader) |
| FamilySearchLists | Rastreador de coleções | Python | **Sem licença** | Sim (2026-11) | `/search/webservice/collectionListData` (interno) | Ideia de detectar coleções novas ou alteradas | [repo](https://github.com/mclow/FamilySearchLists) |

### Servidores MCP, skills e agentes

| Nome | Tipo | Linguagem | Licença | Mantida? | Endpoints e autenticação | O que aproveitar | Link |
|---|---|---|---|---|---|---|---|
| cowork-genealogy | MCP, plugin e agentes | TypeScript | Apache-2.0, com CLA | Sim (2026-11) | OAuth oficial com *authorization code*, *loopback* `127.0.0.1:1837` e `scope=offline_access`. Usa um client id embutido (`config/familysearch.json`) cujo nome sugere chave interna do FS (inferência). Tem ferramentas de texto completo, registros, imagens e catálogo; faz OCR por OpenRouter/Gemini | 31 agentes (GPS, avaliação de fonte, conflito, *proof*), 60 ferramentas (`volume-search`, `volume-bisect`, `image-transcribe`), ADRs, orçamento de requisições | [repo](https://github.com/PioneerAIAcademy/cowork-genealogy) |
| ianderso/familysearch-mcp | MCP | Python | MIT | Sim (2026-11) | Não embute client id; pede o token do app do próprio usuário. Lugares, coleções e *waypoints* funcionam sem token. Busca usando `service/search/hr/v2/personas` e `.../fulltext/search` com o Bearer **e um User-Agent de navegador** | `docs/API-NOTES.md` (rotas verificadas, Accept por recurso, ARKs, semântica da busca), `compare_person` com "pacotes" de mudança | [repo](https://github.com/ianderso/familysearch-mcp) |
| maxwellmezadre/familysearch-mcp | CLI e MCP | TypeScript (Bun) | MIT | Sim (2026-10) | `fssessionid` como Bearer em `www.familysearch.org/platform/...`, importado do navegador | Em PT-BR. **Toda escrita passa por aprovação humana** (*elicitation*). Cache SQLite, lista de lacunas, exportação GEDCOM e CSV | [repo](https://github.com/maxwellmezadre/familysearch-mcp) |
| G4rutti/familysearch-mcp | MCP (fork que combina harrywesterman, twonky4 e izzybps) | TypeScript | MIT | Sim (2026-11) | Cookies do navegador (Brave). Usa `/service/tree/tree-data/...`, `/service/search/{hr,tree}/v2/personas`, `fulltext/search` (com `q.groupName`), `catalog/v3/search`, *waypoints* e `deepzoomcloud`. Espaçamento ≥4 s com variação, pausa de 15 min no "error 15" ou 429 | `AUDIT.md` em português: cookies presos ao domínio, IDs validados, downloads confinados a uma pasta | [repo](https://github.com/G4rutti/familysearch-mcp) |
| JonathanReiss15/familysearch-mcp | MCP | TypeScript | MIT | Sim (2026-07) | Cookies do Brave, porque o Chromium do Playwright "is blocked by FamilySearch bot detection". Exige também os cookies `reese84` e `incap_ses_*` | Base dos forks acima | [repo](https://github.com/JonathanReiss15/familysearch-mcp) |
| cartwrightdj/familysearch-mcp | MCP | TypeScript | MIT | Não (2026-04) | OAuth oficial com `FS_APP_KEY` próprio e `/platform/tree/*` | Exemplo mínimo do caminho oficial | [repo](https://github.com/cartwrightdj/familysearch-mcp) |
| igormel81/genealogy-mcp | MCP | Python | MIT | Sim (2026-11) | Monta links de busca do FS; não chama a API | Catálogo de 179 arquivos, matriz de variantes de nome, regra de "dois fatos independentes" | [repo](https://github.com/igormel81/genealogy-mcp) |
| ex-ussr-kinship-research | Skill | Python e Markdown | MIT | Sim (2026-11) | Usa uma sessão de navegador ou um MCP conectado | Protocolo de evidência: status em vez de porcentagem, `independence_group`, script de auditoria, pedidos a arquivos | [repo](https://github.com/DenisSergeevitch/ex-ussr-kinship-research) |
| genealogy-agent-skills | Skills e plugin | Python e Markdown | MIT | Sim (2026-11) | — | Empacotamento para vários hosts (Claude, Codex, Gemini, Cursor) | [repo](https://github.com/rfsbraz/genealogy-agent-skills) |
| genealogy-agents | Regras e ferramentas para vários agentes | Python | MIT | Sim (2026-11) | Ponte para um Chrome logado via `chrome-devtools-mcp` | `AGENTS.md` com regras de evidência, leitura de manuscrito por dois modelos, *lint* de palavras de certeza | [repo](https://github.com/houlette/genealogy-agents) |
| Genealogy-MCP (gedcom-mcp, gramps-mcp, wikitree-mcp) | MCP | Python | **AGPL-3.0** | Sim (2026-11) | API pública da WikiTree, Gramps Web, arquivo local | Só inspiração (AGPL) | [GitLab](https://gitlab.com/genealogy-mcp) |
| PeWu/wikitree-mcp | MCP | TypeScript | Apache-2.0 | Não ("not planning to continue") | API da WikiTree | — | [repo](https://github.com/PeWu/wikitree-mcp) |
| GedcomMCP | MCP | Python | MIT | Sim (2026-09) | Arquivo GEDCOM local | — | [repo](https://github.com/airy10/GedcomMCP) |
| raphink/geni-mcp | MCP | TypeScript | **Sem licença** | Não (2026-04) | API da Geni (só li os metadados) | — | [repo](https://github.com/raphink/geni-mcp) |
| transkribus-mcp-server | MCP | TypeScript | **FSL-1.1-MIT** (código-fonte disponível, mas não é OSI) | Sim (2026-11) | API legada TrpServer e API de *processing* do Transkribus | Mapa da API do Transkribus | [repo](https://github.com/lazyants/transkribus-mcp-server) |
| internet-archive-skills | Skill oficial do Internet Archive | — | **AGPL-3.0** | Não (2026-01) | APIs do archive.org | Exemplo de skill oficial de uma instituição | [repo](https://github.com/internetarchive/internet-archive-skills) |

### Leitura de manuscritos (HTR)

| Nome | Tipo | Linguagem | Licença | Mantida? | Endpoints e autenticação | O que aproveitar | Link |
|---|---|---|---|---|---|---|---|
| Transkribus | Plataforma de HTR (SaaS) | — | Proprietário | Sim | API REST com OIDC Bearer nos planos Scholar, Team e Org | Modelos públicos de português (ver Q4) | [preços](https://www.transkribus.org/pricing), [API](https://docs.transkribus.org/reference) |
| eScriptorium | Plataforma web de HTR | Python e Vue | MIT (`LICENSE`) | Sim (2026-11) | Auto-hospedado | Interface para corrigir e treinar | [GitLab](https://gitlab.com/scripta/escriptorium) |
| Kraken | Motor de OCR/HTR | Python | Apache-2.0 | Sim (2026-11) | Local | Treinar um modelo de português manuscrito | [repo](https://github.com/mittagessen/kraken) |
| Loghi | Kit de HTR | Python e Shell | MIT | Sim (2026-10) | Local (Docker) | Alternativa ao Kraken | [repo](https://github.com/knaw-huc/loghi) |
| FamilySearch Full-Text Search | HTR do próprio FS | — | Serviço interno | Sim | Sem API pública. A rota do site é `/service/search/fulltext/search` | Já cobre o português | [blog](https://www.familysearch.org/en/blog/what-is-full-text-search) |

## Q1. API oficial do FamilySearch

### O que oferece hoje

A referência oficial, cujo índice completo está em [llms.txt](https://developers.familysearch.org/main/llms.txt), traz estes grupos de endpoints:

- **Family Tree:** pessoas, com leitura em lote de até 200 ([readpersons](https://developers.familysearch.org/main/reference/readpersons.md)); pais, filhos, cônjuges e famílias; `ancestry` e `descendancy`; busca na árvore; *matches* e *not-a-match*; *merge*; relacionamentos; **fontes**, com criação, anexação e leitura de *source references*; notas; histórico de alterações; retratos; discussões; e a "Source Box" (pastas de fontes do usuário).
- **User Trees**, árvores do usuário, antes chamadas de CET. Chegaram em 2026 com grupos, upload, sincronização e histórico ([newsletter 2026 Q4](https://developers.familysearch.org/main/changelog/2026-q4-familysearch-developer-newsletter.md)).
- **Memories:** ler memórias, personas e comentários, e enviar foto, história, PDF e áudio ([índice](https://developers.familysearch.org/main/llms.txt)).
- **Padrões:** lugares (busca, descrições, jurisdições, tipos), datas padronizadas, nomes e vocabulários controlados.
- **Registros históricos:** a coleção "FamilySearch Historical Records Archive" existe em `/platform/collections/records` e devolve modelos de link para `image-metadata`, `image-records`, `waypoints` e `records/personas/{pid}/matches` ([exemplo](https://developers.familysearch.org/main/docs/read-familysearch-historical-records-archive.md)). Mas:
  - "**The API provides no searchability of historical records.**" ([compatibility-review-process](https://developers.familysearch.org/main/docs/compatibility-review-process)).
  - Os apps de *record hinting* têm de mandar o usuário ao site, "because third-party applications do not have adequate access privileges to the FamilySearch historical records collection. This is primarily due to legal issues" ([compatibility-checklist](https://developers.familysearch.org/main/docs/compatibility-checklist)).
  - Os *matches* de uma pessoa aceitam `collection=records` ([readtreepersonmatches](https://developers.familysearch.org/main/reference/readtreepersonmatches.md)). Os links que voltam já trazem o `cid=partner-{client_id}` ([linking-to-familysearch-pages](https://developers.familysearch.org/main/docs/linking-to-familysearch-pages)).
- **Imagens:** a referência atual não documenta nenhum endpoint para baixar imagens de registros. Só há modelos de link e o retrato da pessoa ([readpersonportrait](https://developers.familysearch.org/main/reference/readpersonportrait.md)). *Inferência:* `sg30p0.../das/v2/...` e `deepzoomcloud` não estão documentados.
- **Escrita:** criar e alterar pessoas, relacionamentos, fontes, notas, memórias e discussões, e fazer *merge*. Cada tipo de escrita exige uma certificação própria, a "Write compatibility" ([compatibility-review-process](https://developers.familysearch.org/main/docs/compatibility-review-process)).
- **Persistência:** ARKs são os únicos links estáveis ("Only two categories of links are guaranteed to remain stable", [linking-to-familysearch-pages](https://developers.familysearch.org/main/docs/linking-to-familysearch-pages)). ETags fracas em pessoas, relacionamentos e histórico permitem GET condicional ([caching](https://developers.familysearch.org/main/docs/caching)).

### Chave de app e tipos de app

- A chave é pedida pelo formulário do portal e chega por e-mail ([página inicial](https://developers.familysearch.org/)).
- "Your app key is automatically enabled to access Beta and Production access has additional requirements" ([getting-started](https://developers.familysearch.org/main/docs/getting-started)). O Beta é "an old snapshot of production data".
- As chaves são separadas por ambiente, e as URIs de *callback* precisam ser registradas por e-mail ao devsupport ([api-key-management](https://developers.familysearch.org/main/docs/api-key-management), [developer-support](https://developers.familysearch.org/main/docs/developer-support)).
- Tipos de app: *web*, *desktop/native* e *mobile*. Apps nativos e móveis **precisam** usar PKCE e o navegador do sistema, e o *redirect* recomendado é `http://127.0.0.1:{porta}` ([native-and-mobile-apps](https://developers.familysearch.org/main/docs/native-and-mobile-apps)). Há clientes públicos, sem `client_secret`, e confidenciais ([authorization-code-flow](https://developers.familysearch.org/main/docs/authorization-code-flow)).
- Para chegar à produção:
  1. assinar o "Compatible Product Affiliate Agreement", a "Security Assessment" e o "Production App Key Request and Use Agreement";
  2. passar por revisão de produto, técnica e de segurança;
  3. obter certificação separada para cada tipo de leitura e de escrita.

  "**Only a legal, registered business or non-profit organization will be eligible**" ([compatibility-review-process](https://developers.familysearch.org/main/docs/compatibility-review-process)).
- A mesma página lista o que tende a ser recusado: "Personal, individual, or student academic projects", "**Open-source tools without a business marketing model of executable code**", "Only a prototype", "Extensions deliverable as only browser plug-ins" e "Using AI to generate information uploaded to FamilySearch without full disclosure and approval".
- Divergência entre fontes: o projeto ianderso diz que "An individual developer key reaches only the **Integration sandbox**" ([AUTH.md](https://github.com/ianderso/familysearch-mcp/blob/main/docs/AUTH.md)), enquanto a doc atual fala em Beta. *Inferência:* o portal mudou em 2026, e a doc oficial é a que vale.

### Fluxos de autenticação

Conforme [authentication](https://developers.familysearch.org/main/docs/authentication) e [getaccesstoken](https://developers.familysearch.org/main/reference/getaccesstoken.md):

- **Authorization Code**, para web, desktop e mobile, com PKCE obrigatório nos nativos. Endpoints: `https://ident.familysearch.org/cis-web/oauth2/v3/authorization` e `.../token`. O código expira em 5 minutos e só pode ser usado uma vez. O *scope* `openid` habilita o OIDC ([authorization-code-flow](https://developers.familysearch.org/main/docs/authorization-code-flow)).
- **Unauthenticated Session** (`grant_type=unauthenticated_session` com `ip_address`): "can only be used for API calls that do not require a user to log in". Só Places e Standardized Dates aceitam ([authentication](https://developers.familysearch.org/main/docs/authentication)).
- **Client Credentials**: "not available for general use".
- **Password**: "No longer supported" ([obtain-access-token-with-username-and-password](https://developers.familysearch.org/main/docs/obtain-access-token-with-username-and-password)).
- **Refresh tokens**: "not available for general use" ([authorization-code-flow](https://developers.familysearch.org/main/docs/authorization-code-flow)), embora o exemplo de resposta na mesma página mostre um `refresh_token`.
- **Duração do token**: "expires 24 hours after it is issued or after it is unused for 60 continuous minutes" ([getaccesstoken](https://developers.familysearch.org/main/reference/getaccesstoken.md)).
- **Regras sobre o token**:
  - o checklist exige que "Each user must obtain a FamilySearch access token";
  - "No permanent storage of FamilySearch API Session ID is permitted";
  - o cookie de sessão "must be a secure cookie" ([compatibility-checklist](https://developers.familysearch.org/main/docs/compatibility-checklist)).

  *Inferência:* quando o projeto usa o `fssessionid` como Bearer, está usando o mesmo tipo de credencial que o checklist proíbe guardar. A CLI não deve persistir esse valor em disco.

### Limites de taxa documentados

Fonte: [throttling](https://developers.familysearch.org/main/docs/throttling).

- O limite é "**per-user basis**", somando todas as sessões e produtos. Sem usuário logado, conta a *app key*.
- O que se mede é tempo de processamento, não número de requisições: "requests could be given 18 seconds of execution time within a 1 minute window. Different endpoints may have different throttling windows".
- Ao estourar, o servidor responde 429 com `Retry-After`. Todas as respostas trazem `X-PROCESSING-TIME` em milissegundos.
- Existe um endpoint para testar o *throttling*: `GET /platform/throttled?processingTime=60001`.
- *Inferência:* o 429 com Retry-After de 600 s que observamos no servidor de imagens é a mesma política aplicada a outro serviço. A doc não cita números para imagens.

### O que os termos permitem e proíbem

**Termos de Uso do site** ([legal/terms](https://www.familysearch.org/en/legal/terms), "Updated 2023-09-01"):
- O usuário pode "view, download, and print materials from this site only for your personal, noncommercial use".
- Sem permissão escrita, não pode usar software "to perform a bulk download, to scrape content, or to otherwise harvest any information or materials from this site".
- "You agree not to circumvent any of the controls or usage rules we have implemented."
- Também é proibido "decompile, emulate, reverse engineer … or reconstruct this site" sem permissão.
- Os termos não falam em IA nem em API.

**Programa de parceiros** (só se aplica a quem assina o acordo):
- Sobre APIs não documentadas: "A solution that uses undocumented APIs will not be listed in the App Gallery … and the app key issued for that app will be disabled" ([compatibility-review-process](https://developers.familysearch.org/main/docs/compatibility-review-process)).
- Sobre IA: "you may **never** use FamilySearch Group data … in connection with AI. This includes feeding FSI Content into an AI system as input" (mesma página e [ai-disclosure-addendum](https://developers.familysearch.org/main/docs/ai-disclosure-addendum)).
- Sobre dados: só se pode mostrar informação de vivos ao próprio usuário. Dados de falecidos (ou de quem nasceu há 110 anos ou mais) podem ser compartilhados com qualquer usuário autenticado. O cache do navegador deve ser limpo ao fim da sessão. Tarefas longas são permitidas só enquanto o usuário inicia a tarefa (mesma página).
- Desde 2017, "ordinance crawlers" não são aceitos ([app-approval-considerations](https://developers.familysearch.org/main/docs/app-approval-considerations)).

**robots.txt** ([robots.txt](https://www.familysearch.org/robots.txt), requisição anônima): proíbe `/ark:/61903/1:`, `/ark:/61903/2:`, `/tree/person`, `/tree/pedigree`, `/search/all-collections/results/` e outros caminhos.

**Inferência:** um projeto open-source e pessoal não se encaixa no programa de parceiros. Tem duas saídas:
1. usar a API oficial com a própria chave, o que dá o Beta e, talvez, nunca a produção;
2. usar a sessão do navegador e endpoints internos, que é o que o projeto faz hoje e o que fazem os MCPs comunitários. Isso fica fora do que é suportado e esbarra nas cláusulas de *bulk download*, *scraping* e "circumvent".

A escolha é do mantenedor. O risco recai sobre a conta do usuário. O [AUDIT.md do G4rutti](https://github.com/G4rutti/familysearch-mcp/blob/main/AUDIT.md) registra o mesmo risco ("Risco de conta").

## Q2. Bibliotecas e SDKs

- **SDKs oficiais.** A página [tools-sdks](https://developers.familysearch.org/main/docs/tools-sdks) lista gedcomx-java, fs-php-lite, gedcomx-php e fs-js-lite. Também cita um SDK Python da comunidade marcado "Not Active". Todos os oficiais tiveram commit em 2026 (tabela acima).
  - O **fs-js-lite** (MIT) "is designed for use in a web browser". Tem seções sobre PKCE, *redirects* e *throttling* ([README](https://github.com/FamilySearch/fs-js-lite)).
  - O antigo `familysearch-javascript-sdk` está descontinuado ([README](https://github.com/FamilySearch/familysearch-javascript-sdk)).
- **GEDCOM X.** A especificação está em [FamilySearch/gedcomx](https://github.com/FamilySearch/gedcomx) (Apache-2.0), e as extensões do FS em [gedcomx-familysearch-extensions](https://github.com/FamilySearch/gedcomx-familysearch-extensions) (Apache-2.0, parada desde 2017).
  - Java: [gedcomx-java](https://github.com/FamilySearch/gedcomx-java), Apache-2.0.
  - PHP: [gedcomx-php](https://github.com/FamilySearch/gedcomx-php), Apache-2.0.
  - JS: [rootsdev/gedcomx-js](https://github.com/rootsdev/gedcomx-js), MIT, parada desde 2017.
- **Rust:**
  - [`gedcomx` 0.1.8 e `gedcomx_file`](https://github.com/ephraimkunz/gedcomx-rs) (MIT, edição 2024): serializa XML (yaserde) e JSON (serde). É a única implementação de GEDCOM X em Rust ([crates.io](https://crates.io/crates/gedcomx)).
  - Para GEDCOM 5.5.1/7.x, o [`oxidged`](https://github.com/trois-six/oxidged) (MIT) lê sem perda, escreve conforme a especificação e converte 5.5.1↔7.x "as the FamilySearch migration guide says". Recebeu o histórico de `ged_io` (arquivado, aponta para ele) e de `rust-gedcom`.
  - Não há cliente Rust da API FamilySearch mantido. O único que encontrei é de 2017 e não tem licença ([webnard/familysearch-rust](https://github.com/webnard/familysearch-rust)).

## Q3. Ferramentas da comunidade

### Exportadores

**getmyancestors** ([repo](https://github.com/Linekio/getmyancestors); é fork de `freeseek/getmyancestors`, que hoje tem só um *initial commit*):
- Licença GPL-3.0-or-later (`LICENSE`). Último commit em 2026-08-04.
- **Autenticação** (`classes/session.py`):
  1. faz GET em `www.familysearch.org/auth/familysearch/login` para pegar o `XSRF-TOKEN`;
  2. faz POST de `username` e `password` em `ident.familysearch.org/login`;
  3. faz GET em `/cis-web/oauth2/v3/authorization` com o *scope* `openid profile email qualifies_for_affiliate_account country`;
  4. faz POST em `/token` com `grant_type=authorization_code`.
- O `client_id` padrão e o *redirect* vêm de uma página de demonstração de terceiros. Esse mesmo `client_id` aparece como exemplo na doc oficial de OIDC ([authorization-code-flow](https://developers.familysearch.org/main/docs/authorization-code-flow)).
- Usa User-Agent falso do Firefox (`fake_useragent`).
- **Endpoints:** `api.familysearch.org/platform/tree/persons?pids=` (200 por vez), `/sources`, `/notes`, `/changes`, `/couple-relationships/...`, `/platform/memories/memories/...` e o interno `/service/tree/tree-data/reservations/person/{id}/ordinances`.
- *Inferência:* o login pelo formulário é o caminho "password" que a doc diz não suportar mais, só que feito por fora da API.
- O [genealogia-plantilla](https://github.com/victormico/genealogia-plantilla/blob/main/tools/fs/session.py) diz que copiou essa sessão e acrescentou `openid`, um regulador por `X-PROCESSING-TIME` e cache. Diz também que a versão 1.1.2 do getmyancestors quebrou porque o IdP passou a exigir `openid` "in spring 2026".

### Extensões e visualizadores

- **WikiTree Sourcer** (MIT, mantida): é a extensão aberta mais útil que encontrei. No FamilySearch, lê o `fssessionid` e o envia como Bearer a `/service/tree/links/sources/` e ao JSON do registro (`extension/site/fs/browser/fs_fetch.mjs`). Depois gera citações. Isso confirma, por código, a técnica que já usamos.
- Visualizadores de descendência e mapas como Puzzilla, Relative Finder, TreeMonk e Family Tree Validator são **fechados**. Aparecem na [galeria de soluções](https://www.familysearch.org/en/innovate/solutions-gallery), que hoje lista 91 soluções.
- Não encontrei extensão aberta e mantida específica de FamilySearch para pistas ou mapas.

### Baixadores de imagens

Todos usam a sessão do navegador e endpoints internos de imagem. Nenhum tem licença aberta, exceto o familysearchDwHTR (GPL-3.0, parado desde 2020).

- O **nikborovets** (mantido) monta *tiles* DeepZoom na maior resolução, retoma downloads e grava de forma atômica. O ritmo padrão é 0,12 a 0,35 s entre requisições (`downloader.py`). *Inferência:* nesse ritmo ele bateria no limite que observamos.
- O **FSFilmDownloader** diz "No longer works!!!", o que mostra que esses caminhos internos mudam.

### Programas certificados que sincronizam

A [galeria](https://www.familysearch.org/en/innovate/solutions-gallery) lista 15 "Family Tree Management Apps" ("These apps and websites can sync information with FamilySearch Family Tree"):

> AncestralFire, Ancestral Quest, Familienbuch, Family Historian, FamilyLoupe, Gramps, Heartwood, Legacy Family Tree, One Great Family, MacFamilyTree, RootsFinder, RootsMagic, The MagiKey, TreeMonk e TreeViz.

- Todos são proprietários, exceto o **Gramps**, que aparece como "FamilySearch Compatible: Family Tree".
- A integração do Gramps vive num fork GPL-2.0 ([gramps-FamilySearchCoreIntegration](https://github.com/SourceAirbender/gramps-FamilySearchCoreIntegration)). O addon anterior parou com "API Key is no longer valid" ([FSG_Sync](https://github.com/SourceAirbender/FSG_Sync)). Não consegui ler a página do Gramps sobre o assunto (403).
- O **AncestralFire** é um app certificado que funciona dentro de Claude, ChatGPT e Gemini e só escreve "until you click Approve" ([site](https://ancestral-fire.com/)).

## Q4. IA e agentes em genealogia

### Servidores MCP e skills para o FamilySearch

Ver a tabela. Há três famílias de autenticação:

1. **OAuth oficial com chave própria:**
   - [cartwrightdj](https://github.com/cartwrightdj/familysearch-mcp);
   - [ianderso](https://github.com/ianderso/familysearch-mcp), que não embute client id e documenta por quê: emprestar um id alheio "can get that id rate-limited or revoked", e postar senha no formulário "breaks whenever the sign-in flow changes" ([AUTH.md](https://github.com/ianderso/familysearch-mcp/blob/main/docs/AUTH.md));
   - [cowork-genealogy](https://github.com/PioneerAIAcademy/cowork-genealogy), com *loopback* e `offline_access`.
2. **Sessão do navegador como Bearer:** [maxwellmezadre](https://github.com/maxwellmezadre/familysearch-mcp), WikiTree Sourcer.
3. **Cookies completos e endpoints `/service/`:** [JonathanReiss15](https://github.com/JonathanReiss15/familysearch-mcp) e o fork [G4rutti](https://github.com/G4rutti/familysearch-mcp). Exigem os cookies anti-bot `reese84` e `incap_ses_*`, e o Chromium do Playwright é bloqueado (README do JonathanReiss15).

Achado útil para a CLI, documentado pelo ianderso e não verificado por mim: `service/search/hr/v2/personas` e `service/search/fulltext/search` aceitam o mesmo Bearer, "but only with a browser `User-Agent`. The token alone gets 403 (a firewall page)" ([API-NOTES.md](https://github.com/ianderso/familysearch-mcp/blob/main/docs/API-NOTES.md)). *Inferência:* isso explica o 403 que tivemos com curl na busca de texto completo.

O mesmo arquivo registra:
- que `/platform/records/personas` "answers from a partial index";
- que lugares, coleções e *waypoints* respondem sem token;
- a semântica dos parâmetros `q.*`.

Skills e regras que valem como inspiração de método:
- [ex-ussr-kinship-research](https://github.com/DenisSergeevitch/ex-ussr-kinship-research): status de evidência em vez de porcentagem, `independence_group` para que cópias não contem como provas independentes, e auditoria por script.
- [genealogy-agents](https://github.com/houlette/genealogy-agents): "two-reader rule" para manuscritos e *lint* de palavras de certeza.
- [cowork-genealogy](https://github.com/PioneerAIAcademy/cowork-genealogy): agentes `source-evaluation`, `conflict-resolution`, `proof-conclusion` e `where-else-to-search`; ferramentas `volume-search` e `volume-bisect`; [ADRs](https://github.com/PioneerAIAcademy/cowork-genealogy/tree/main/docs/adrs) como "put guardrails at the write boundary".
- [igormel81/genealogy-mcp](https://github.com/igormel81/genealogy-mcp): catálogo de arquivos por jurisdição histórica.

### GEDCOM, WikiTree, Geni, Ancestry e arquivos

- **GEDCOM:** [GedcomMCP](https://github.com/airy10/GedcomMCP) (MIT) e [Genealogy-MCP/gedcom-mcp](https://gitlab.com/genealogy-mcp/gedcom-mcp) (AGPL-3.0).
- **WikiTree:** [PeWu/wikitree-mcp](https://github.com/PeWu/wikitree-mcp) (Apache-2.0, abandonado) e [Genealogy-MCP/wikitree-mcp](https://gitlab.com/genealogy-mcp/wikitree-mcp) (AGPL-3.0).
- **Geni:** [raphink/geni-mcp](https://github.com/raphink/geni-mcp), sem licença.
- **Ancestry:** não encontrei MCP para o Ancestry.com. O repositório `atorenherrinton/ancestry-mcp` é sobre outra coisa (linhagens em Supabase). *Inferência:* o Ancestry não publica API aberta; não achei documentação oficial.
- **Arquivos:** [internet-archive-skills](https://github.com/internetarchive/internet-archive-skills) é a skill oficial do Internet Archive (AGPL-3.0), recomendada na página [bots](https://archive.org/developers/bots.html).

### Leitura de manuscritos

**Transkribus** (proprietário):
- Preços ([pricing](https://www.transkribus.org/pricing)):
  - plano gratuito com "50 free credits per month";
  - Scholar a €99 por ano (900 créditos);
  - Team a €449 por ano (1.500 créditos);
  - HTR custa "1 credit / page".
- A API existe nos planos Scholar, Team e Organisation, e "API jobs use credits … at half the rate" (mesma página).
- Modelos públicos de português:
  - "Portuguese Handwriting 16th-19th c." (ID 53270, CER 5,2%). Foi treinado em 6.226 páginas de processos da Inquisição de 1536 a 1821 e "works with … fill-in forms, documents from non-inquisitorial areas" ([modelo](https://www.transkribus.org/models/portuguese-handwriting-16th-19th-centuries)).
  - "Portuguese handwriting M2" (ID 45090, CER 8%, 2.766 páginas, período não informado) ([modelo](https://www.transkribus.org/models/transkribus-portuguese-handwriting-m2)).
  - *Inferência:* nenhum dos dois foi treinado em registros paroquiais brasileiros. É preciso medir o CER numa amostra nossa.
- A página de preços não diz se os documentos enviados são usados para treinar modelos.

**eScriptorium e Kraken** (livres):
- eScriptorium: MIT ([LICENSE](https://gitlab.com/scripta/escriptorium)). Kraken: Apache-2.0 ([repo](https://github.com/mittagessen/kraken)). Os modelos ficam no Zenodo.
- O modelo genérico moderno, McCATMuS (CC-BY-4.0, [Zenodo](https://zenodo.org/records/13788177)), foi treinado em francês, espanhol, italiano, inglês, latim, alemão e occitano, **sem português** ([dataset card](https://huggingface.co/datasets/CATMuS/modern)).
- Servem para português dos séculos XVII a XIX só se treinarmos ou ajustarmos um modelo.

**Loghi** (MIT, [repo](https://github.com/knaw-huc/loghi)):
- O modelo genérico "should work ok on 17th and 18th century" (README), mas foi feito para material holandês (inferência a partir da instituição, KNAW HuC).
- Uma fonte secundária cita "a Loghi early modern Portuguese model" ([UNM Reliable AI](http://amaranth.unm.edu/reliable-ai//sessions/transcribing-handwritten-documents/), secundária). Não achei esse modelo na fonte primária.

**HTR do FamilySearch:**
- A busca de texto completo cobre "English, Spanish, and Portuguese", e a página avisa: "you may see some transcription errors" ([blog, 2025-10-18](https://www.familysearch.org/en/blog/what-is-full-text-search)).
- Nos slides oficiais, Portugal é o 5º e o Brasil o 7º país com mais imagens; o português é a 4ª língua; são "2.5 billion images" ([slides](https://cms-z-assets.familysearch.org/76/11/2391b338343eba9cf348fa73cccc/familysearch-full-text-search-your-golden-path-to-ancestral-discovery-slides.pdf)).
- Não há API pública: a única rota é a interna, usada pelos MCPs.

**LLMs de visão:** o cowork-genealogy transcreve com `google/gemini-3.7-flash` via OpenRouter (`auth/config.ts`). *Inferência:* para quem assinou o acordo de parceiro, isso seria vedado pela cláusula de IA; para uso pessoal, os termos do site não falam do assunto.

## Q5. Outras APIs úteis para Brasil e Portugal

| Serviço | Oficial e documentada? | O que tem | Autenticação e limites | Fonte |
|---|---|---|---|---|
| WikiTree | Sim | `https://api.wikitree.com/api.php` com `getProfile`, `getAncestors`, `getDescendants`, `searchPerson` e outros | Perfis públicos sem login. Para privados, `clientLogin` (o usuário entra no site e volta com `authcode`; sessão por cookie). Pede `appId` | [wikitree-api](https://github.com/wikitree/wikitree-api), [authentication.md](https://github.com/wikitree/wikitree-api/blob/main/authentication.md) |
| Geni | Sim | `https://www.geni.com/api`: perfil, união, documentos, `immediate-family`, `profile/search` | OAuth com `authorize` e `request_token`. Token de 24 h no exemplo; *refresh* rotativo de 90 dias. Limites informados pelos cabeçalhos `X-API-Rate-*` | [help](https://www.geni.com/platform/developer/help), [oauth](https://www.geni.com/platform/developer/help/oauth_server_side), [rate limits](https://www.geni.com/platform/developer/help/rate_limits) |
| digitarq (Torre do Tombo e arquivos distritais) | OAI-PMH documentado em [boletim da DGArq de 2011](https://arquivos.dglab.gov.pt/wp-content/uploads/sites/16/2014/01/DGArqBolt-17.pdf). Não achei REST documentado | `https://digitarq.arquivos.pt/oai`: `Identify`, `ListMetadataFormats` (`oai_dc` e `ead`), `ListSets` com um conjunto por arquivo distrital (ADAVR, ADBGC…) | Anônimo (requisição anônima em 2026-11-23). O site é uma SPA Next.js | [Identify](https://digitarq.arquivos.pt/oai?verb=Identify) |
| Arquivo Nacional (SIAN) | **Não** tem API | Consulta em `consulta.an.gov.br`, com proteção anti-bot (página de desafio F5 TSPD numa requisição anônima). Só há dados abertos estáticos (CSV e JSON do SIAN/REGENT) | O acesso remoto exige cadastro e aceite de termos | [dados.gov.br](https://dados.gov.br/dados/conjuntos-dados/sistema-de-informacoes-do-arquivo-nacional---sianregent), [servicos.gov.br](https://www.servicos.gov.br/api/v1/servicos/1079) |
| Hemeroteca Digital Brasileira (BN) | **Não** tem API documentada | DocReader (produto comercial DocPro) com URLs `memoria.bn.gov.br/docreader/{bib}/{pagina}` | Há artigo que cita OAI-PMH na BNDigital, mas sem endpoint publicado (secundária) | [tutorial BN](https://www.gov.br/bn/pt-br/atuacao/colecoes-e-servicos-aos-leitores/publicacoes-seriadas/tutorial-basico_hdb_v5.pdf), [FEBAB](http://repositorio.febab.org.br/files/original/8/2211/1321-1334-1-PB.pdf) (secundária) |
| Internet Archive | Sim | Metadata API, IAS3, Changes, Views, Wayback, OCR (hOCR). A biblioteca oficial `internetarchive` expõe a Scrape API (`/services/search/v1/scrape`) e uma busca de texto completo "beta" (`be-api.us.archive.org/ia-pub-fts-api`) | Exige User-Agent que identifique o bot, a versão e, para agentes de IA, o modelo. Respeitar 429 e `Retry-After`; o exemplo usa `-j4 --delay 1` | [developers](https://archive.org/developers/index-apis.html), [bots](https://archive.org/developers/bots.html), [search.py](https://github.com/jjjake/internetarchive/blob/master/internetarchive/search.py) (AGPL-3.0) |
| HathiTrust | Bibliographic API: sim. **Data API aposentada em 2024-07-17** | `catalog.hathitrust.org/api/volumes/brief/{tipo}/{id}.json` (respondeu 200 numa requisição anônima). Não há API pública de busca de texto completo | O HTRC encerrou Analytics e Data Capsules em 2026-09-30. O Extracted Features 2.5 é a última versão | [aposentadoria](https://www.hathitrust.org/press-post/retirement-data-api/), [bib_api](https://old.www.hathitrust.org/bib_api.html), [HTRC FAQ](https://www.hathitrust.org/about/research-center/htrc-transition-faq/) |
| FamilySearch Research Wiki | MediaWiki padrão. **Não** consta na doc de desenvolvedor | `https://www.familysearch.org/en/wiki/api.php` (MediaWiki 1.43; respondeu numa requisição anônima) | — | Requisição anônima em 2026-11-23. O cowork-genealogy lê a wiki em tempo de execução ([ADR-0012](https://github.com/PioneerAIAcademy/cowork-genealogy/blob/main/docs/adrs/ADR-0012-read-locality-and-record-type-guidance-from-the-wiki.md)) |

## Q6. Conclusão para o projeto

### Replicar (reimplementar em Rust)

1. **Cliente da API oficial** (`/platform/tree/*`, `sources`, `matches`, `places`, `ancestry`, `descendancy`), usando os tipos do crate `gedcomx`. Motivo: é a parte documentada e estável. Não há cliente em Rust, e os SDKs oficiais são JS, PHP e Java ([tools-sdks](https://developers.familysearch.org/main/docs/tools-sdks)).
2. **Autenticação em duas vias**, que o usuário escolhe:
   - (a) OAuth com PKCE e *loopback* `127.0.0.1`, como a doc exige para apps nativos ([native-and-mobile-apps](https://developers.familysearch.org/main/docs/native-and-mobile-apps)) e como o cowork-genealogy já faz;
   - (b) o `fssessionid` do navegador, mantido só em memória. O checklist proíbe guardar o Session ID de forma permanente.

   Motivo: (a) é o caminho suportado; (b) é o que funciona hoje para um projeto pessoal.
3. **Regulador de taxa por `X-PROCESSING-TIME` e `Retry-After`**, com orçamento por janela, pausa longa após 429 e nada de novas tentativas em bloqueio anti-bot. Motivo: a política oficial mede tempo de processamento por usuário ([throttling](https://developers.familysearch.org/main/docs/throttling)). Genealogia-plantilla, G4rutti e cowork-genealogy chegaram, cada um por conta própria, ao mesmo desenho.
4. **Cache local com ETag e GET condicional** ([caching](https://developers.familysearch.org/main/docs/caching)), mais um log de buscas, inclusive das que não acharam nada. Motivo: menos requisições, e a pesquisa fica reproduzível (igormel81, ex-ussr).
5. **Escrita na árvore só sob aprovação humana, com ETag e `If-Match`**, como no maxwellmezadre, no AncestralFire e nos "pacotes" do `compare_person` do ianderso. Motivo: a árvore é compartilhada, e o FS exige revisão para qualquer app que escreve.
6. **Mapa das rotas internas** (busca de registros, texto completo, catálogo, *waypoints*, DGS, imagens), escrito a partir de observação própria e do [API-NOTES.md](https://github.com/ianderso/familysearch-mcp/blob/main/docs/API-NOTES.md) (MIT), atrás de um módulo isolado e desligável. Motivo: hoje é a única forma de buscar e ver imagens; isolar o módulo limita o estrago quando a rota mudar.

### Usar como dependência

- **`gedcomx`** (MIT), para os tipos GEDCOM X.
- **`oxidged`** (MIT), para importar e exportar GEDCOM 5.5.1 e 7.x. Ressalva: o projeto tem só um mês. Fixar a versão.
- As especificações GEDCOM X e GEDCOM 7 (Apache-2.0) como referência normativa.
- Opcionalmente, a **API do Transkribus**, como serviço externo para HTR de português, configurado pelo usuário e pago por ele.

### Só inspiração (ler, não copiar)

- **getmyancestors** (GPL-3.0): mapeamento para GEDCOM e ordem das chamadas.
- **Genealogy-MCP**, **internet-archive-skills** e a biblioteca `internetarchive` (AGPL-3.0).
- Projetos **sem licença**: nikborovets, thefirans, FamilySearchLists, raphink/geni-mcp, FSG_Sync, docs da wikitree-api. Sem licença, todos os direitos são reservados.
- **cowork-genealogy** (Apache-2.0 com CLA; o código pode ser reaproveitado com atribuição): o desenho dos agentes, as ferramentas de livro (`volume-search`, `volume-bisect`) e os ADRs.
- **ex-ussr-kinship-research** e **genealogy-agents** (MIT): protocolo de evidência e regras do `AGENTS.md`.
- **WikiTree Sourcer** (MIT): formatos de citação.
- **digitarq OAI-PMH**: fonte para a skill "achar onde um documento está" em Portugal.

### Evitar

- **Login automatizando o formulário com usuário e senha** (getmyancestors). A doc diz que o *grant* `password` não é mais suportado, e o ianderso aponta que isso quebra quando o fluxo de login muda.
- **Emprestar o `client_id` de outro projeto.** O volume de uso pode fazer a chave dele ser limitada ou revogada ([AUTH.md](https://github.com/ianderso/familysearch-mcp/blob/main/docs/AUTH.md)), e o FSG_Sync mostra que chaves morrem.
- **Contornar a proteção anti-bot** (`--disable-blink-features=AutomationControlled`, cookies `reese84`/`incap_ses_*`, User-Agent falso). Esbarra em "circumvent any of the controls" ([terms](https://www.familysearch.org/en/legal/terms)).
- **Download em massa e varredura sem limite.** "Bulk download" e "scrape" precisam de permissão escrita. A CLI deve ter teto por sessão e baixar só o que a pesquisa pede.
- **Guardar o Session ID ou o token em disco** sem necessidade, e **publicar imagens ou dados** do FS, que pelos termos são só para uso pessoal.
- **Enviar dados do FS a IA sem avisar o usuário.** Para parceiros, isso é proibido; para nós, deve ser no mínimo uma escolha explícita e documentada no `AGENTS.md` (inferência).

### Lacunas que nenhuma ferramenta cobre

1. **Varrer um livro de imagens (DGS) inteiro respeitando o limite**, com retomada, orçamento e índice próprio por imagem. Os baixadores não respeitam o limite, e o cowork-genealogy limita a 20 imagens por filme.
2. **HTR aberto para assentos paroquiais luso-brasileiros dos séculos XVII a XIX.** Não há modelo Kraken ou Loghi de português manuscrito, e os do Transkribus vêm da Inquisição.
3. **Fontes brasileiras sem API** (SIAN, Hemeroteca, arquivos estaduais e cúrias). Nenhum MCP ou skill as cobre.
4. **Cliente OAI-PMH e EAD para o digitarq** voltado à genealogia, por exemplo para localizar um livro de uma freguesia num arquivo distrital.
5. **Régua de prova em português**, ligada a fontes do FamilySearch: GPS adaptado, com `independence_group` e "dois fatos além do nome". Existe em inglês e russo, não em português.
6. **Pedidos a arquivos e cúrias brasileiras e portuguesas.** O ex-ussr faz isso para a ex-URSS.
7. **Cliente FamilySearch em Rust**, oficial ou não.
