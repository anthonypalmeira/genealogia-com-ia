# A ponte do navegador é um processo local com que a aba fala direto

As rotas internas do site do FamilySearch (`/service/search/…`) respondem 403 fora de um navegador, e o kit não se disfarça de navegador. Decidimos que a ponte é um processo local, aberto por `garimpo ponte abrir`, que escuta em `127.0.0.1`: um trecho de JavaScript, avaliado uma vez na aba logada, pede trabalho a esse processo, confirma que recebeu, faz cada requisição de dentro da página e devolve a resposta. Escolhemos esse desenho porque nele o corpo das respostas não passa pelo contexto do agente, o agente só avalia um trecho na aba e o comando é o mesmo com ou sem ponte: uma rota do site vai pela aba quando a ponte está aberta e direto quando não está.

## O que foi medido

Teste de 2026-11-24, no navegador do T3 (Chrome 152 sobre Electron 44), com a aba em `www.familysearch.org` sem login:

- **CSP.** O site só envia `frame-ancestors`. Sem `connect-src`, a página pode chamar `http://127.0.0.1`.
- **Rede local.** A aba fez GET e POST de 300 KB a um processo local, sem aviso de permissão. O navegador do T3 não enviou o *preflight* de Private Network Access; a ponte responde a ele mesmo assim, para os navegadores que o exigem.
- **Aba em segundo plano.** Um laço de 40 segundos (20 esperas de 2 segundos) rodou inteiro numa aba que não estava à mostra. O laço não usa temporizadores, que uma aba oculta estrangula: ele só espera requisições pendentes.
- **Limites da avaliação.** `preview_evaluate` tem 15 segundos por chamada e 64 KB no resultado. O trecho devolve `ligada` na hora, e nenhuma resposta volta pela avaliação.
- **Ponta a ponta.** O trecho real, avaliado numa aba de um site falso local, atendeu buscas do `garimpo`, inclusive depois de 25 segundos parado.

Não foi verificado com o FamilySearch logado: a aba do teste não tinha sessão.

## Opções consideradas

- **Ponte mediada pelo agente**: a CLI enfileira as requisições, o agente roda lotes curtos na aba e devolve as respostas. Cada página do texto completo traz até 100 imagens com 9 a 21 mil caracteres cada: tudo isso passaria pelo contexto do agente, em pedaços de 64 KB, e cada comando viraria várias idas e vindas. Fica como alternativa se um navegador barrar a rede local.
- **Levar os cookies e o User-Agent do navegador para a CLI**: é disfarce, e os termos do FamilySearch proíbem contornar controles.
- **Exportar as respostas pela pasta de downloads do navegador**: ela guarda cerca de 20 arquivos e some quando a aba fecha.

## Consequências

- O token não sai da aba. Com a ponte, as rotas do site dispensam `garimpo sessao entregar`.
- Toda requisição pela ponte passa pelo mesmo regulador das diretas: trava da conta, orçamento, teto da sessão e `Retry-After`.
- A ponte só atende a página da origem de `--fs-site` (CORS) que conhece o segredo do endereço. Os comandos falam com ela por um soquete Unix na pasta de estado.
- Ela faz um pedido por vez, só GET e só texto. Baixar imagens pela aba vai exigir respostas binárias.
- Se a aba navega ou fecha, o laço acaba: o comando sai com 7 e o agente avalia o trecho de novo. Um pedido só conta como feito depois que a aba confirma o recebimento, porque a aba que sumiu pode deixar para trás a requisição em que pedia trabalho.
