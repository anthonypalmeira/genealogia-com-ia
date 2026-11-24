# Termos e riscos

O que os termos do FamilySearch e de outros acervos dizem, o que o kit faz para limitar o risco e o que continua sendo decisão de quem usa. As citações e os links das fontes primárias estão em [ferramentas-existentes.md](../pesquisa/ferramentas-existentes.md). Isto não é aconselhamento jurídico.

## O que os termos do FamilySearch dizem

- O usuário pode ver, baixar e imprimir materiais do site só para **uso pessoal e não comercial**.
- Sem permissão escrita, é proibido usar software para download em massa, para *scraping* ou para colher informação do site.
- É proibido contornar os controles e as regras de uso do site.
- A API oficial não tem busca de registros históricos nem download de imagens, e a chave de produção exige empresa ou organização registrada. Um projeto pessoal e open-source não se encaixa no programa de parceiros.
- Sem permissão, é proibido publicar conteúdo do site em outro site ou rede e transmiti-lo ou distribuí-lo a outros sites.
- Para os parceiros, é proibido usar conteúdo do FamilySearch com IA sem permissão escrita. Os termos do site, que valem para o usuário comum, não falam de IA.

## O que o kit faz

O kit usa a sessão do navegador do próprio usuário e, além das rotas oficiais, rotas internas do site, como fazem as ferramentas da comunidade. Isso fica fora do que o FamilySearch suporta.

| Risco | O que o kit faz |
|---|---|
| Download em massa | Teto de requisições por sessão; baixa só o que a pesquisa pede |
| Sobrecarga e bloqueio da conta | Respeita `Retry-After` e o tempo de processamento; pausa longa depois de um 429; um baixador por conta |
| Contornar proteções | Não contorna: sem User-Agent falso, sem cookies anti-robô, sem automação escondida. Diante de bloqueio, para e avisa |
| Vazamento da sessão | O token fica só em memória: nunca em disco, em comando nem em mensagem |
| Dados de pessoas vivas | Ficam fora do repositório, dos exemplos e dos testes |
| Erro público na árvore | Só fontes e notas, em rodada autorizada, com releitura e registro; parentesco, fusão e fatos, só com pergunta |
| Rotas internas que mudam | Ficam num módulo isolado, que pode ser desligado |

## O que continua com quem usa

- **O risco é da conta de quem usa.** O FamilySearch pode limitar ou suspender uma conta.
- Quem tem chave de aplicativo pode usar a API oficial nas rotas que ela cobre.
- Um agente que lê uma imagem ou um registro envia esse conteúdo ao provedor do modelo, fora do computador de quem usa. Os [termos do site](https://www.familysearch.org/en/legal/terms) dizem: "You may not transmit or distribute content from this site to other sites." Usar o kit com um modelo hospedado esbarra nessa cláusula: é um risco a mais, que quem usa aceita.
- Imagens e dados do FamilySearch são para uso pessoal: não os publique.

## Outros acervos

| Acervo | Regra |
|---|---|
| Casa Setecentista de Mariana | Os termos proíbem reproduzir ou publicar as imagens sem autorização do IPHAN. Uso pessoal, sim; subir a imagem para uma árvore pública, não |
| Internet Archive | Pede que o agente se identifique no User-Agent e respeite 429 e `Retry-After` |
| Sites com captcha, login ou página anti-robô | O kit só orienta: o usuário abre no navegador dele |
| Projetos e bases de terceiros | Sem licença, todos os direitos são reservados: leia e cite, não copie |

## Pessoas vivas

- Nenhum nome, identificador, documento ou dado de pessoa viva entra neste repositório.
- Os guias e os testes usam exemplos genéricos ou casos públicos antigos.
- A pesquisa de cada família fica na pasta do próprio usuário, fora do kit.
- Num pedido a um arquivo, os dados do requerente são campos que o usuário preenche.
- A checagem [`scripts/checar_docs.py`](../../scripts/checar_docs.py) recusa Markdown com CPF, e-mail ou telefone.
