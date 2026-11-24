# Briefings de leitura

O que a skill `ler-acervo` entrega a cada subagente. O subagente não vê a conversa: o briefing é tudo o que ele sabe. Preencha os campos entre `<` e `>` e envie o texto.

## Varredura de uma faixa

Envie este texto seguido da seção [Formato de entrega](#formato-de-entrega) inteira:

```text
Leia as imagens <de> a <ate> do filme <grupo>, em <pasta do filme>, assento por assento. Só estas: as outras faixas têm dono.

Documento: <tipo de livro, paróquia ou cartório e anos, pelo mapa do filme>.
Layout: <assentos por página, onde fica a margem, onde fica o nome do registrado>.
Alvos: <cada pessoa com todas as grafias, e de onde vem cada dado>. Trate-os como hipótese.
Palavras de parentesco: <a lista>.
Período: <de que ano a que ano>.
Já lido: <as imagens da faixa que a cobertura dá como lidas>.

Como ler: a seção Ler de <caminho de docs/guias/leitura-e-operacao.md>. A imagem recebe só recorte, ampliação e contraste global: `garimpo imagens recortar`, com `--regiao` e `--ampliar`, e `garimpo imagens contraste`. Transcreva os nomes de cada assento como estão na imagem, com `[ilegível]`, `[?]` e as leituras alternativas, e só depois compare com os alvos.

Seus arquivos: <pasta do filme>/leitura-<AAAA-MM-DD>-<de>-<ate>.md, no formato de entrega abaixo; os recortes que fizer; e um arquivo por pista em <pasta do elo>/pistas/, no formato de registro de <caminho de docs/guias/regua-de-prova.md>. O estado do elo é da conversa principal.

Devolva em texto, sem imagens: cada achado, a lista de ilegíveis e quantas páginas tiveram cada resultado.
```

## Leitura às cegas

O briefing não leva nome nenhum: nem no texto, nem no caminho de um arquivo. A pasta de um elo traz os nomes das pessoas; a pasta do filme, não.

```text
Transcreva <o assento: a posição dele na página> da imagem <arquivo na pasta do filme>. Você não recebe os nomes esperados, de propósito.

Documento: <tipo de livro e século>.
Layout: <assentos por página, onde fica a margem>.

Como ler: a seção Ler de <caminho de docs/guias/leitura-e-operacao.md>. A imagem recebe só recorte, ampliação e contraste global: `garimpo imagens recortar`, com `--regiao` e `--ampliar`, e `garimpo imagens contraste`. Use outras palavras da mesma mão como amostra de letras.

Devolva em texto, sem imagens: a transcrição, com `[ilegível]` e `[?]`; as leituras alternativas de cada palavra duvidosa, da mais provável à menos; e a resolução da imagem lida.
```

## Formato de entrega

O arquivo de leitura de uma faixa traz a lista de alvos e uma linha por imagem:

```markdown
# Leitura das imagens <de> a <ate> do filme <grupo>, em <AAAA-MM-DD>

Alvos: <a lista, como veio no briefing>

| Imagem | Resultado | Achado ou o que não se lê |
|---|---|---|
| <número> | <`lido`, `sem alvo` ou `ilegível`> | <o assento, a transcrição do trecho, as leituras alternativas e o alvo que casou> |
```

- `lido`: a página traz um alvo, e o achado vai na terceira coluna.
- `sem alvo`: a página foi lida inteira e nenhum alvo aparece.
- `ilegível`: a página, ou um assento dela, não se lê; a terceira coluna diz qual.

A imagem que não foi lida também ganha linha, com o motivo no lugar do resultado: `pendente de acesso`, ou `repetida`, com o número da imagem lida.

A conversa principal acrescenta a cada achado a leitura às cegas e a releitura dela.
