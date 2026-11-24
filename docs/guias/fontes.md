# Fontes: onde está cada documento

Catálogo de fontes por região, com acesso, receita e pegadinhas, e o caminho para o que só existe em papel. O FamilySearch tem guia próprio: [familysearch.md](familysearch.md). O que os termos de cada site permitem: [termos-e-riscos.md](termos-e-riscos.md).

As receitas vêm de sessões reais de novembro de 2026 e não foram retestadas para este guia. Endereços e rotas mudam: quando uma receita falhar, registre o que mudou.

Regras de acesso que valem para todas as fontes:

- **Jurisdição da época.** Procure o registro onde ele foi feito: a paróquia-mãe, o termo, a comarca e o bispado daquele ano, e não o município de hoje.
- **Bloqueio é pendência.** Se o site recusa o agente (403, 429, captcha, página anti-robô, login), registre "pendente de acesso", peça ao usuário para abrir no navegador dele e siga com outra fonte.
- **Índice é pista.** Índices e transcrições de terceiros localizam o documento. A nota vem da leitura da imagem.

## Por onde começar

| Situação | Primeiro | Depois |
|---|---|---|
| Minas Gerais, séc. XVIII até cerca de 1835 | Catálogo da Casa Setecentista de Mariana, com todos os nomes da família | Texto completo da coleção jurídica de Minas no FamilySearch; listas nominativas do Arquivo Público Mineiro |
| Minas Gerais, 1840 a 1940 | Certidões e memórias já anexadas na árvore; registro civil | Diário oficial *Minas Geraes*; cartório ou arquivo do fórum |
| São Paulo, sécs. XVI–XVIII | Série impressa *Inventários e Testamentos* e os índices de inventários | Juízo dos Resíduos do Arquivo do Estado; *de genere* da Cúria; séries impressas de sesmarias e atas |
| Ancestral português com descendente ou colateral habilitado (padre, familiar do Santo Ofício, cavaleiro) | API do digitarq: a busca indexa os pais e avós citados | *De genere* de dioceses portuguesas no FamilySearch |
| Nordeste, séc. XIX | Texto completo das coleções católicas no FamilySearch | Registro civil depois de 1888: a imagem dá os avós |
| Açores | Biblioteca Digital dos Açores | Batismo dos filhos, que dá a naturalidade dos pais |
| Espanha, séc. XVI | Catálogo do FamilySearch, testando antes se as imagens abrem | PARES e arquivos regionais |
| A prova decisiva só existe em papel | [Fontes offline e como pedir](#fontes-offline-e-como-pedir) | Regra de parada, em [estrategias.md](estrategias.md) |

## Minas Gerais

| Fonte | O que tem | Acesso e receita | Pegadinhas |
|---|---|---|---|
| Casa Setecentista de Mariana (arquivo do IPHAN, visualizador da UFV) | Inventários, testamentos, ações cíveis, justificações e sesmarias do termo de Mariana, que no séc. XVIII ia muito além da cidade. Foi a fonte de maior rendimento em Minas: os inventários deram quase todas as provas A | O catálogo são planilhas públicas, uma por série: baixe todas e varra localmente com os nomes da família, inclusive padrinhos e vizinhos. As imagens ficam no visualizador oficial, que pede conta gratuita: o usuário entra e consulta por lá, com a série, o códice e o auto que o catálogo deu | Nem toda série está digitalizada. Não há livros de notas online. A ficha pode dar "Mariana" como lugar de um documento de outra freguesia. Os termos proíbem reproduzir ou publicar as imagens sem autorização do IPHAN: guarde para uso pessoal e peça cópia autorizada quando for publicar |
| Arquivo Público Mineiro, sistema SIAAPM | Listas nominativas de 1831–32 e 1838–40, por fogo; Registro Paroquial de Terras de 1854–56; requerimentos com relações de almas | Público, sem login. Ficha: `modules/<módulo>/brtacervo.php?cid=<n>`. Galeria com todas as imagens: `modules/<módulo>_docs/viewcat.php?cid=<n>&num=200`. Busca: `modules/<módulo>/search.php?query=…`, com os acentos em Latin-1 | O HTML vem em Latin-1. As imagens têm cerca de 1.200 px, o que deixa idades duvidosas. As listas não indexam nomes: busque pelo lugar e leia as páginas. A galeria repete páginas: deduplique por hash antes de dividir a leitura. Sem palavras de parentesco, a lista vale C |
| Diário oficial *Minas Geraes* | Editais de citação de herdeiros, concessões de terras e nomeações, de 1892 a 1943 e de 2005 em diante | API pública do próprio site: autenticação anônima, busca paginada por texto e período, e download da página | A busca é por frase exata sobre um OCR que perde letras acentuadas. O achado é uma **menção**: prova que o inventário ou a escritura existiu e diz a quem pedir a cópia |
| Acervo histórico do Tribunal de Justiça de Minas | Descrição de processos judiciais antigos, por comarca | Catálogo público | Esteve fora do ar nas sessões: tente de novo antes de pedir a um fórum |
| Índice estadual do registro civil (e-CIVIL) | Índice dos registros civis do estado | Exige login do usuário | Não foi testado |
| Índices de terceiros (páginas de genealogistas com índices de batismos, casamentos, óbitos, censos e inventários de uma paróquia) | Nome, pais ou cônjuge, data, livro e folha | Páginas e PDFs públicos; converta de Latin-1 e busque localmente | A ordem pode ser pelo prenome dos pais. Há lacunas de anos. Raramente citam o arquivo. Servem para datar e limitar a busca, nunca como prova |

## São Paulo

| Fonte | O que tem | Acesso e receita | Pegadinhas |
|---|---|---|---|
| Arquivo Público do Estado, Juízo dos Resíduos (acervo digitalizado) | Cerca de mil contas de testamento, de 1653 a 1857, com o PDF inteiro de cada processo | O índice exige login gov.br, feito pelo usuário. Monte o índice inteiro, por vila e por ano, e cruze com a descendência da árvore. Cada resultado traz o código do processo, que dá o endereço do PDF | O ano do índice é o **das contas**, não o do testamento. O índice erra o gênero do testador. Consulte sempre antes de propor um pedido ao arquivo |
| Arquivo Público do Estado, outras bases | Juízo dos Órfãos (só parte descrita), descrição de fundos | Abertas | A série de inventários dos Órfãos não está online. Servem para fundamentar um pedido |
| Índice de inventários não publicados do 1.º Ofício (página de genealogia com uma lista por letra) | Inventariado, inventariante, ano e número de ordem | Páginas públicas em Latin-1 | A partir de certo número de ordem, os processos são do Juízo dos Resíduos e não dos microfilmes: o índice diz em qual dos dois procurar. O "inventariante" pode estar errado |
| Transcrições e resumos de inventários feitos por projetos voluntários | Texto de inventários e testamentos do séc. XVII | Páginas públicas, uma por inventário | A grafia é literal, com a preposição colada ao sobrenome e abreviaturas: busque com variantes. Nas sessões, uma prova ficou mais de uma hora escondida num arquivo já baixado porque a busca usou a grafia moderna. Confira no impresso ou no manuscrito antes de anexar |
| Série impressa *Inventários e Testamentos* (Arquivo do Estado) | Inventários paulistas dos sécs. XVI–XVII | OCR de boa parte dos volumes no Internet Archive; os volumes sem OCR estão em imagens no FamilySearch | A página do PDF não é a página impressa. Num conjunto baixado, o número do arquivo não é o número do volume |
| Atas da Câmara de São Paulo, *Registro Geral*, *Sesmarias*, *Cartas de Datas* (séries impressas) | Vereanças, cargos, terras, "cunhado" e "genro" em cartas de sesmaria | PDFs da Câmara; HathiTrust e Google Livros para as outras séries | A filiação quase nunca aparece. A edição impressa omite notas marginais e erra nomes |
| Revistas de genealogia e teses universitárias | Artigos que citam a cota do original | Sites das revistas e repositórios de teses | Leia as notas de rodapé atrás da cota. Cópias em arquivos da web podem vir truncadas: confira o tamanho |

De 1676 a 1745, São Paulo pertencia ao bispado do Rio de Janeiro: processos eclesiásticos desse período podem estar no Rio.

## Nordeste

Quase tudo que rendeu está no FamilySearch (coleções católicas e registro civil): veja [familysearch.md](familysearch.md). Fora dele:

| Fonte | O que tem | Pegadinhas |
|---|---|---|
| Arquivos diocesanos e cúrias | Paroquiais não filmados, processos de dispensa e banhos. Até 1892, boa parte do Nordeste dependia do bispado de Olinda | Sem catálogo online: é pedido |
| Fóruns e arquivos dos tribunais estaduais | Inventários | Listas de inventários publicadas em blogs dizem que o processo existe, não o que ele contém |
| Sites de genealogia regionais por assinatura | Listas de inventários por comarca | Só com a assinatura do usuário |
| Bibliotecas digitais de tribunais superiores | Processos que subiram em recurso | Baixe o arquivo do documento, não a página que o apresenta |

Nome igual não é a mesma pessoa: uma lista de inventários levou a um homônimo.

## Portugal e Açores

| Fonte | O que tem | Acesso e receita | Pegadinhas |
|---|---|---|---|
| Torre do Tombo, pelo digitarq | Inquisição (processos, denunciações, visitações), habilitações do Santo Ofício com testemunhas que nomeiam avós e bisavós, Leitura de Bacharéis, ordens militares | API pública, sem login. Busca: `/api/docs/search?query=<texto ou cota>&searchType=ALL_WORD` (ou `ALL_PHRASE`). Metadados: `/api/docs/frozen/<docId>`. Lista de imagens: `/api/rdigital/<docId>?fromIndex=0&max=2000`. Imagem: `/api/rdigital/dissemination?fileId=<id>`. Há também OAI-PMH em `/oai`. Espere cerca de 0,3 s entre pedidos. O `garimpo digitarq` faz tudo isso (`buscar`, `documento`, `imagens`, `baixar`, `oai`), com a espera e o registro na cobertura; a colheita do `oai` é varrida por `garimpo busca nomes` | Alguns documentos têm duas representações: escolha uma, ou o download traz metade dos arquivos. O tamanho da imagem varia. Há processos sem imagem. A fórmula "sem mácula de infecta nação" é de limpeza de sangue e não prova origem alguma |
| Biblioteca Digital dos Açores | Livros paroquiais por ilha e freguesia | Sem login. Cada livro tem um código com a freguesia, o tipo (B de batismos, C de casamentos) e os anos; o número da página do visualizador é o número da imagem | Resolução baixa, em página dupla: recorte e amplie. A naturalidade dos pais costuma estar no batismo do filho |
| Índices de casamentos por ilha (páginas de voluntários) | Casamentos por freguesia, com a imagem e a série | PDF público | Mostram homônimos da mesma freguesia: confira os pais |
| Arquivos distritais, pelo digitarq | Paroquiais do continente | Parte dos registros migrados aparece como "a aguardar publicação" | Enquanto não publicam, é consulta presencial ou num centro do FamilySearch |

Prova de naturalidade, do mais forte para o mais fraco: "natural de" no batismo de um filho; declaração no próprio testamento; "freguês de", que é moradia e não nascimento.

## Espanha

| Fonte | O que tem | Pegadinhas |
|---|---|---|
| PARES (arquivos estatais, incluindo a Inquisição no Archivo Histórico Nacional) | Catálogo e parte das imagens | Bloqueia por IP depois de muitos pedidos (429 por horas): um pedido por vez, ou pedido de reprodução ao arquivo |
| Arquivos regionais com busca online (chancelarias, protocolos notariais) | Fidalguias, pleitos, escrituras | Úteis para descartar homônimos |
| Arquivos diocesanos | Datas iniciais dos livros de cada paróquia | Consulte antes de pedir: o batismo procurado pode ser anterior ao primeiro livro |
| FamilySearch | Índice de paroquiais | As imagens costumam ser restritas fora dos centros. O índice automático traz datas absurdas e ignora filtros |

## Nacional e online

| Fonte | O que tem | Acesso e receita | Pegadinhas |
|---|---|---|---|
| Internet Archive | Séries impressas com OCR | Metadados e busca pelas APIs oficiais. Baixe só o arquivo de texto do item e busque sem acentos. O site pede um User-Agent que identifique o agente | A página do PDF não é a página impressa |
| HathiTrust | Séries paulistas e documentos históricos | Busca dentro do volume e texto por página, pelo navegador do usuário | As imagens costumam ser negadas: só se lê o texto. Volumes "search-only" dão só a contagem. Muitos pedidos seguidos dão 429 |
| Google Livros | Livros sem cópia em outro lugar | Busca por frase e imagem da página | Captcha depois de poucas dezenas de consultas, resolvido pelo usuário. Só alguns trechos por termo |
| Hemeroteca Digital (Biblioteca Nacional) | Jornais por década | Busca por frase exata | A imagem fica atrás de captcha. Sirva-se dela para localizar edição e página; o usuário abre a imagem |
| Arquivo Nacional (SIAN) | Códices e fundos federais | Sem API; consulta com cadastro | Esteve em manutenção nas sessões |
| Cópias em arquivos da web | Sites fora do ar | Endereço da cópia arquivada | Confira o tamanho: há cópias truncadas |
| Silva Leme e Pedro Taques em texto, árvores e transcrições de terceiros | Mapa de citações | Web | Repetem uns aos outros e raramente citam documento: valem como pista |
| Busca na web | Onde estão os originais | — | Os resumos dos buscadores inventam detalhes: confira no site oficial |

## Fontes offline e como pedir

### Quem guarda o quê

| Instituição | Guarda | Canal |
|---|---|---|
| Arquivos eclesiásticos e cúrias | Processos matrimoniais, dispensas (no casamento entre primos, nomeiam o tronco comum), *de genere*, livros não filmados | Sem índice de nomes online. Pesquisa presencial com agendamento ou pedido por carta. O catálogo pode arquivar o processo sob outro lugar |
| Cartórios de registro civil | Certidões de inteiro teor, de 1889 em diante | E-mail ou balcão. Cobram por período de busca, por ato e por pessoa |
| Fóruns e arquivos judiciais | Inventários, testamentos e partilhas | Carta ou e-mail, com o número do processo quando houver |
| Arquivos públicos estaduais | Séries não digitalizadas | Pedido com a cota exata. Originais muito antigos podem estar fora de circulação: peça pelo nome os processos que as fichas dão como não filmados |
| Órgão que guarda um acervo digitalizado com restrição de uso | Cópias autorizadas e licença para publicar | Contato no site oficial |
| Arquivos distritais portugueses e arquivos espanhóis | Paroquiais e processos | Formulário de pedido de reprodução |

Confirme cada contato no site oficial da instituição antes de escrever.

### Antes de recomendar um pedido

1. **Prove que o documento existe ou existiu**: catálogo, índice ou menção. Escreva se ele "existe (com referência)", "existiu, sobrevivência desconhecida" ou é "suposição". Separe "foi pedido", "foi feito" e "sobreviveu".
2. **Prove que não está online**: catálogo do FamilySearch, acervo digitalizado do arquivo dono do índice, texto completo.
3. **Pergunte o que o usuário já pediu.**

Pronto quando o documento tem estado, referência e a lista das bases conferidas.

### Formato do pedido

- **Curto.** Um parágrafo e uma lista numerada com o que o arquivo precisa para achar: cota ou códice, livro, ano, folha, nomes e o motivo em uma linha.
- **No formato do canal.** E-mail em texto puro; carta em PDF de uma página; mensagem de WhatsApp com até 7 linhas.
- **Para cartório.** Tire cada limite do período de um documento: o registro civil começa em 1889, a idade mínima para casar, cerca de 100 anos de vida. Confira na imagem as datas que fixam os limites. Feche os intervalos em múltiplos de 5 anos, ponha o preço por item e o total, e destaque o subconjunto de melhor custo.
- **Peça "inteiro teor"**: o extrato é derivado.
- **Dados pessoais** entram só como campos a preencher pelo usuário.
- **Para um genealogista contratado**: liste o que já temos, os homônimos descartados e o que cada item provaria.
- **Envio.** Deixe como rascunho. Envie só com ordem explícita do usuário e com o texto mostrado antes.
- **Registro.** Anote cada pedido com a situação: rascunho, enviado, respondido, cumprido.
