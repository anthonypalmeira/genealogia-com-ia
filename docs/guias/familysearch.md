# FamilySearch em detalhe

Rotas, limites e pegadinhas do FamilySearch, observados em sessões reais de novembro de 2026 e não retestados para este guia. O kit usa a sessão do navegador do próprio usuário e, além das rotas oficiais, rotas internas do site. Leia antes [termos-e-riscos.md](termos-e-riscos.md): o uso é pessoal e não comercial, e o risco é da conta de quem usa.

As rotas internas (`/service/…`) mudam sem aviso. Quando uma falhar, registre a mudança em vez de insistir. A lista de rotas oficiais e as fontes primárias estão em [ferramentas-existentes.md](../pesquisa/ferramentas-existentes.md).

## Sessão

| Item | Regra |
|---|---|
| Login | O usuário entra no FamilySearch no navegador dele. O agente usa o identificador dessa sessão como `Authorization: Bearer` |
| Token | Fica só em memória: nunca em disco, em linha de comando, em mensagem nem em log |
| Expiração | A sessão dura menos de um dia. Com 401, peça novo login uma vez e siga com o que não depende dele |
| Concorrência | Várias conversas na mesma conta somam no mesmo limite: um baixador por conta |
| Dentro da aba | A execução na aba tem limite de tempo e de tamanho de resposta. Rode o trabalho em segundo plano, sinalize o fim e exporte o resultado para arquivo. Aba oculta estrangula temporizadores |
| Downloads do navegador | A pasta de downloads de uma aba guarda poucos arquivos e pode sumir ao fechar a aba: nunca é o destino final |

## Árvore: leitura

| Rota | Uso | Notas |
|---|---|---|
| `GET /platform/tree/persons/{id}`, mais `/sources`, `/parents`, `/children`, `/spouses`, `/notes`, `/memories` | Estado de uma pessoa | `Accept: application/x-fs-v1+json`. A resposta 204 é lista vazia. A atribuição de cada fonte mostra quem anexou e quando |
| `GET /platform/tree/persons?pids=A,B,…` | Percurso em largura, em lotes | Traz filhos e cônjuges de cada pessoa |
| `GET /platform/tree/ancestry?person=X&generations=8` | Ascendência, em blocos de 8 gerações | Só segue os pais preferidos. Aceite só números de ascendência inteiros: os outros são cônjuges. Deduplique por ID |
| `GET /platform/tree/descendancy?person=X&generations=4` | Descendência | **Para em 1.000 pessoas**: em ramos grandes, percorra em largura e salve em arquivo |
| `…/changes` | Quem mudou o quê e quando | Alto rendimento para auditar vínculos sem fonte |
| Busca na árvore com `q.givenName`, `q.surname`, `q.birthLikeDate.from`, `q.anyPlace` | Achar perfis | A sintaxe antiga `q=givenName:"X"` dá erro 400 |

O que já está na árvore é hipótese. Exemplos vistos: título de fonte que era de um homônimo; notas "Transcrição" geradas por IA de outro colaborador, com pais inventados; dicas de homônimos aceitas em massa; data do registro posta como data do evento; vínculos sem fonte criados por um só contribuidor.

## Registros indexados

- Rota: `GET /service/search/hr/v2/personas`, com `q.givenName`, `q.surname`, `q.fatherGivenName`, `q.motherGivenName`, `q.spouseGivenName`, `q.birthLikeDate.from`, `q.anyPlace` e `count`. Torne um termo obrigatório com `q.<campo>.require=on` e filtre por coleção com `f.collectionId`. `Accept: application/json`.
- A busca é difusa: datas e lugares só ordenam os resultados.
- A entrada devolvida é a persona que casou com a busca, que pode ser o pai e não o registrado.
- O índice erra: prenome trocado, ano trocado, a cor da pessoa lida como nome, a assinatura do vigário indexada como padrinho, óbito que era nascimento, lugar padronizado para um município de nome parecido. Em registros civis, o índice omite os avós que a imagem traz.
- Rendimento: médio no registro civil depois de 1889; baixo em paroquiais do séc. XIX pouco indexados.

## Texto completo

- Rota: `GET /service/search/fulltext/search`, com `q.text`, `q.fullName`, `q.anyPlace`, `q.anyDate.from`, `q.anyDate.to`, `q.groupName`, `count` e `offset`. Só funciona dentro da aba logada: fora dela, responde 403.
- Sintaxe: `+termo` obriga, `-termo` exclui, `?` e `*` são curingas. **Uma frase entre aspas sem `+` é opcional**: escreva `+"nome completo"`. Tire os acentos da frase. `+termo` sem filtro de lugar ou de grupo devolve milhares de páginas.
- **Puxar o grupo inteiro:** `q.groupName=<grupo>` sem texto devolve a transcrição de cada imagem do grupo. Pagine com `offset`, **pare no total informado** (depois dele os resultados se repetem) e deduplique pelo id. Se o teto de requisições da sessão chegar antes, pare, registre até onde foi e retome depois. Depois busque localmente todos os nomes da família e as palavras "sogro", "genro", "herdeiros", "tio", "a rogo".
- As facetas de lugar mapeiam a cobertura e revelam rótulos errados.
- Em filmes de arquivos judiciais, as fichas de microfilmagem e as capas das caixas servem de sumário.
- Alto rendimento em processos judiciais de Minas, em livros católicos do Nordeste e em impressos paulistas. Baixo em paróquias de Minas.
- Não cobre tudo: há filmes de habilitações e livros de casamento sem transcrição, e o registro civil do Nordeste não tem texto. Ausência no texto completo não prova nada.
- A transcrição erra nomes, datas e lugares, e lê uma taxa em réis como se fosse um ano. Procure cada ideia de vários jeitos e leia a imagem.

## Catálogo

- Busca: `/service/search/catalog/v3/search?query=+place:"<país, estado, cidade>"`. A sintaxe de campo (`+place:`, `+keywords:`, `+title:`) é obrigatória. As notas de filme de cada item dão o filme, o grupo de imagens e o conteúdo.
- Uso: transformar "não achei" em "não foi filmado", por exemplo quando a série só começa depois da data procurada. Para dizer "offline" falta ainda conferir o acervo digitalizado do arquivo dono: veja a regra de parada em [estrategias.md](estrategias.md).
- As coleções jurídicas não estão no catálogo: chega-se a elas pelo texto completo.
- Há títulos errados: um filme rotulado "óbitos" pode trazer batismos de outra paróquia. Livros de várias paróquias filmados juntos aparecem com um só lugar.
- Estar no catálogo não garante acesso às imagens: teste uma imagem antes de delegar uma varredura.

## Imagens

| Operação | Receita |
|---|---|
| Do ARK da imagem ao identificador de artefato | `GET /ark:/61903/3:1:<id>` em JSON devolve o nome (`<grupo>_<n>`) e o identificador |
| Baixar a imagem | `…/das/v2/<artefato>/dist.jpg` com o Bearer. A resposta redireciona para o armazenamento: siga o redirecionamento, senão o arquivo vem vazio |
| Listar um grupo | `…/das/v2/dgs:<grupo>/children?count=<N grande>` devolve os artefatos em ordem. Os parâmetros de paginação são ignorados: peça tudo e fatie localmente |
| Livros sem índice | A API de *waypoints* da coleção lista as localidades, e cada folha traz os ARKs das imagens em ordem |
| Miniaturas para triagem | `…/deepzoomcloud/dz/v1/<artefato>/thumb_p200.jpg`. Não sofreram o limite de requisições. Miniatura em retrato costuma ser ficha ou capa |
| Impresso em grupo grande | `dist.jpg?scale=0.5` responde rápido e continua legível |
| Imagem restrita | Vem um JSON curto de acesso negado. Anote a faixa como pendente de acesso. Blocos restritos entre processos costumam ser as fichas |

A resolução do `dist.jpg` é a maior guardada **daquele filme** e varia de cerca de 1.400 px a 4.300 px. Pouca resolução não é motivo para parar: baixe o original, corte em metades e amplie.

## Memórias e retratos

- `/platform/tree/persons/{id}/memories` lista as memórias da pessoa. Certidões de inteiro teor enviadas por parentes deram muitas provas A.
- Memórias privadas respondem 401 ou 403.
- Selos em retratos (por exemplo, de uma certificação) só aparecem na imagem do retrato. São pista, não documento.
- Uma certidão moderna é a transcrição do cartório: o livro original não foi visto.

## Gravação

Só em rodada autorizada pelo usuário, e só fontes e notas. Parentesco, fusão e fatos, só com pergunta.

| Ação | Receita |
|---|---|
| Criar fonte | `POST /platform/sources/descriptions`, com título, citação, link e notas. O ID da fonte vem num cabeçalho da resposta: leia `Location` e `X-Entity-Id` |
| Anexar a fonte a uma pessoa | `POST /platform/tree/persons/{id}`, com a referência à fonte e o motivo |
| Criar nota | `POST /platform/tree/persons/{id}/notes`, com assunto, texto e motivo |

1. Leia as fontes da pessoa e o histórico do dia: a fonte já existe? Outra conversa está editando?
2. Mostre o plano em tabela, "antes → depois", com o motivo de cada linha, e espere o "sim" do usuário para essa tabela.
3. Grave. Só o que foi lido na imagem vira fonte. A fonte leva citação, link, transcrição e o que prova, e vai para todas as pessoas que o documento cita. Pista vira nota "Hipótese de pesquisa (não comprovada)". Pendência vira nota com o que falta, por ordem de utilidade.
4. Releia com GET e registre a alteração: pessoa, fonte, ação, motivo.

Pronto quando o GET confere cada linha do plano.

Pegadinhas: cabeçalho com travessão ou aspas curvas falha (codifique); a mensagem de anexação não pode ser corrigida depois; ao mudar uma data, mande junto o lugar padronizado, senão ele se perde; não mexa em fontes de outros usuários.

## Limites e ritmo

| Limite | Observado |
|---|---|
| Baixadores em paralelo | Dois ou três na mesma conta deram 429 |
| Volume | Cerca de 1.500 imagens cheias em 30 minutos deram `Retry-After: 600` |
| Ritmo que funcionou | Árvore: até 2 pedidos por segundo. Imagens: 1 a 2 segundos entre elas |
| Tetos | Descendência: 1.000 pessoas. Ascendência: 8 gerações por chamada |

Respeite sempre o `Retry-After`. Depois de um 429 repetido, pare e avise o usuário. Diante de bloqueio anti-robô, pare: o kit não contorna proteções.

## Cobertura por região

O que as sessões encontraram, por coleção. Confirme no catálogo antes de confiar.

| Região | O que há | Lacunas e rótulos errados |
|---|---|---|
| Minas Gerais | Coleção jurídica com processos de várias comarcas (só pelo texto completo); coleção católica com cerca de 300 localidades; registro civil com índice | Muitas localidades faltam na coleção católica. Grupos rotulados com um município trazem processos de outro. As imagens do registro civil costumam ser restritas |
| São Paulo | Inventários e testamentos do 1.º Ofício dos sécs. XVII–XVIII; série impressa *Inventários e Testamentos*; cerca de 400 processos *de genere* da Cúria, sem transcrição; batismos antigos da Sé | Os microfilmes dos inventários param num certo número de ordem. Casamentos da Sé só a partir do séc. XVIII. Microfilmes desbotados, com imagens repetidas |
| Paraíba e Rio Grande do Norte | Coleções católicas; registro civil cujos nascimentos de 1888 a 1905 trazem os quatro avós | Lacunas de décadas em batismos, casamentos e óbitos; livros com título errado; índice que omite os avós |
| Ceará e Pernambuco | Registro civil; registro de dispensas do bispado de Olinda | Paroquiais de algumas vilas não foram filmados. Dispensas fora de ordem |
| Portugal | *De genere* de algumas dioceses, liberados | Paroquiais de vários distritos só abrem num centro do FamilySearch ou no arquivo distrital |
| Açores e Espanha | Índice | Imagens restritas |
