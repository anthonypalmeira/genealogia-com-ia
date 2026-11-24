# Leitura de imagem e operação

Como baixar, triar e ler imagens de documentos sem inchar a conversa, e como operar: sessão, cobertura, estado da pesquisa e formato da resposta. Os números vêm de cerca de 80 sessões reais: 3.767 leituras de imagem na conversa principal levaram contextos a mais de 800 mil tokens e deixaram cada resposta até 2,3 vezes mais lenta.

## Baixar

1. **Liste antes de baixar.** Grave um manifesto do filme ou do processo: número da imagem, endereço, situação (aberta ou restrita), dimensões, hash e caminho local.
2. **Nome estável**: filme mais número da imagem. Sem nomes aleatórios, sem `/tmp`, sem a pasta de downloads do navegador como destino.
3. **Download direto para a pasta da pesquisa**, num arquivo temporário renomeado quando completa. Confira se a imagem é válida e grave o hash.
4. **Retomada.** Pule a imagem só quando o arquivo existe e o hash confere com o manifesto. Imagem restrita fica anotada como pendente de acesso e não se tenta de novo.
5. **Ritmo.** Um baixador por conta, 1 a 2 segundos entre imagens, `Retry-After` obrigatório. Se o teto de requisições da sessão chegar antes do fim, pare e registre até que imagem foi. Detalhes em [familysearch.md](familysearch.md).
6. **Progresso.** Uma linha de registro a cada lote, para responder "travou?" sem adivinhar.
7. **Resolução.** Manuscrito: a original do servidor. Volume impresso grande: metade da escala basta.

Pronto quando o manifesto tem uma linha por imagem pedida, cada uma com hash ou com a pendência anotada.

O `garimpo imagens baixar` faz os passos 2 a 6, e o `garimpo imagens listar` o primeiro: veja [Imagens com o `garimpo`](familysearch.md#imagens-com-o-garimpo). Para o digitarq, `garimpo digitarq baixar` grava o mesmo manifesto.

## Ler

| Passo | O que fazer |
|---|---|
| 0. Já foi lido? | Procure na cobertura e nos relatórios da pesquisa antes de abrir a imagem |
| 1. Original | Leia a imagem na resolução original do servidor. A captura de tela do visualizador tem resolução baixa demais |
| 2. Triar | Primeiro o texto completo e as fichas de microfilmagem. Depois miniaturas, folhas de contato e tiras de margem. **A triagem localiza; nunca prova ausência** |
| 3. Recortar | A página em metades com pequena sobreposição; depois o assento, delimitado; depois a linha ou a palavra ampliada 2 a 3 vezes. Delimitar o assento evita atribuir a ele uma palavra do assento vizinho |
| 4. Limite de leitura | O modelo lê até 2.576 px no lado maior. Recorte em vez de reduzir a página inteira: uma página de 4.300 px reduzida perde cerca de 40% da resolução. Com mais de 20 imagens num pedido, vale outro teto por imagem (medido: 3.000 px) |
| 5. Realçar | Contraste global (níveis, tons de cinza), sempre sobre a imagem inteira. O original fica intocado, e cada operação fica registrada |
| 6. Delegar | Mais de ~20 páginas vão para subagentes: veja [Delegar a leitura](#delegar-a-leitura) |
| 7. Ler às cegas | Na palavra decisiva (filiação, sobrenome, ano), leia sem receber o nome esperado. Use outras palavras da mesma mão como amostra de letras, marque `[ilegível]` e `[?]`, registre as leituras alternativas e só então compare com o alvo |
| 8. Confirmar | A conversa principal abre só o recorte decisivo, de 1 a 3 imagens, e relê antes de gravar. Divergências entre leituras ficam registradas como divergências |
| 9. Etiquetar | Marque como o achado foi visto, com as etiquetas da [régua de prova](regua-de-prova.md), e diga a resolução e a origem da imagem |

### Quando um assento não se lê

Pouca resolução na triagem não é motivo para parar: baixe o original. Um assento só é **inconclusivo** depois de original, recortes, realce e duas leituras cegas. Então:

1. Procure outra cópia: a segunda via, um registro duplicado, o processo matrimonial, uma transcrição publicada.
2. Ponha o assento na lista de ilegíveis a pedir ao arquivo.
3. Peça ao usuário o original ou uma fotografia melhor.

### Delegar a leitura

1. Baixe as páginas, dentro do teto da sessão, e deduplique por hash: galerias e microfilmes repetem páginas.
2. Leia a capa e o fim (há índice?). Amostre algumas imagens, date pelos cabeçalhos e interpole.
3. Divida em faixas sem sobreposição, um subagente por faixa.
4. O briefing leva: o layout do documento, a faixa, a lista dos alvos com as grafias, as palavras de parentesco, o que já foi lido e o formato de entrega. Cada fato do briefing traz a fonte, e o subagente o trata como hipótese.
5. O subagente devolve texto, não imagens: um resultado por página (lido, ilegível ou sem alvo), e cada achado com o número da imagem e o trecho.
6. A conversa principal confere na imagem cada achado que vai usar.

Pronto quando toda página da faixa tem resultado e há uma lista de ilegíveis.

Para varrer uma lista, o leitor recebe os alvos. Para a palavra decisiva, lê às cegas (passo 7).

### A imagem continua sendo o documento

A imagem recebe só três operações: **recorte**, **ampliação** e **contraste global**. Mostre a folha inteira seguida do recorte ampliado, com a transcrição ao lado e a origem de cada complemento.

Fora de questão:

- desenhar, pintar, completar ou recompor letras, à mão ou por IA;
- apagar manchas, dobras ou trechos do papel;
- montar uma imagem com o texto da transcrição;
- apresentar qualquer um desses resultados como reprodução do documento.

Numa das sessões, cinco horas de "recomposição" de um testamento desbotado deram uma imagem que continuava parecendo falsa. Quem avalia pode tratar o dossiê inteiro como adulterado. O que é ilegível no papel entra como `[ilegível]`, com a leitura de outra fonte citada ao lado.

O `garimpo imagens` só tem essas três operações (`recortar`, com `--ampliar`, e `contraste`) e a folha de contato, que junta imagens reduzidas sem escrever nada sobre elas. O original baixado nunca é alterado: cada derivado é um arquivo novo, e o nome dele diz a operação e o trecho (`<nome>.r<x>-<y>-<largura>x<altura>.a2.jpg`, `<nome>.contraste.jpg`).

Escolher quais documentos e páginas entram num relatório é decisão do autor. Tapar ou alterar parte de uma imagem é alteração do documento.

## Operar

### Sessão e acesso

- Confira a sessão do FamilySearch antes de um trabalho longo. Se ela faltar, peça o login ao usuário uma vez e siga com o que não depende dele.
- Peça cedo o que só o usuário faz: login, captcha, consulta no navegador dele.
- O token fica só em memória.
- Bloqueio (403, 429, captcha) entra na cobertura como pendente de acesso.

### Cobertura

Registre cada busca: a consulta, o lugar, os anos, o total, os itens vistos e o resultado. Os resultados possíveis são **lido**, **negativo** e **pendente de acesso**. Leia todos os resultados de uma busca antes de dá-la como negativa: a paginação repete, e uma busca lida pela metade continua aberta, com os itens vistos anotados. O `garimpo` grava essa busca aberta como `parcial`.

Antes de buscar, pergunte à cobertura se aquilo já foi buscado. Nas sessões, o mesmo catálogo vazio foi consultado em quatro conversas.

### A pesquisa do usuário

A pesquisa fica numa pasta do usuário, fora deste repositório. A unidade é o elo:

- o **estado** do elo, com data: a nota pela régua, a decisão do pesquisador, os documentos que o sustentam, o que falta;
- a **cobertura**;
- as **pistas**, no formato da régua;
- as imagens, fora do git, com o manifesto dentro.

Uma única fonte lista as linhagens e os seus elos. Procure nas pesquisas anteriores por todas as grafias do nome antes de começar.

Nada fica só no chat, no navegador ou em `/tmp`: achado, explicação e próximo passo são salvos na mesma resposta.

### Responder

- A primeira palavra responde à pergunta: "Sim" ou "Não", quando couber.
- Nomes completos, com o parentesco de cada pessoa citada. Sem códigos como "elo 12" ou "V2".
- Cada achado diz que elo prova ("X é filho de Y"), a nota antes e depois, e se já está no relatório do usuário.
- Diga o tipo do achado na primeira linha: imagem lida, menção, índice ou "onde estaria".
- Separe o novo do que já se sabia.
- Entregue no formato do destino: texto puro para e-mail, até 7 linhas para WhatsApp, uma página para carta.
- Homônimos da mesma família levam ano e papel.
- Se a frase do usuário admite duas leituras, pergunte.

### Fechar a conversa

Termine toda conversa de pesquisa, com ou sem skill, pelo [fechamento comum](../skills/fechamento.md).

### Na conta do usuário

E-mails e outras ações na conta do usuário ficam em rascunho, e só saem com ordem explícita e com o texto mostrado antes. O agente não cria contas em nome do usuário.
