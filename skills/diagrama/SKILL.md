---
name: diagrama
description: "Desenha linhagens, árvores e provas em Mermaid. Use para pedir um desenho ou explicar uma prova indireta."
---

# Diagrama

Desenha o que a pesquisa já tem, para o usuário entender um achado sem decifrar texto. Você escolhe o que mostrar; o `garimpo diagrama` desenha e acusa os **furos**. O formato da descrição, os papéis e as regras que ele checa estão em `garimpo diagrama --help`.

**Argumentos:** o que desenhar (uma linhagem de `pesquisa/linhagens.md`, um elo com a prova dele ou um trecho da árvore) e, quando o usuário pede imagem, o tema do SVG.

## Passos

1. **Abra** pela [abertura comum](../../docs/skills/abertura.md).
2. **Escolha o que mostrar**, em até ~20 pessoas:
   - **Papéis.** A linha `direto` é uma só: a que o desenho defende. Num trecho de árvore, os outros ramos entram como `colateral` ou `contexto`.
   - **Rótulos.** O nome completo de cada pessoa e os anos que um documento lido dá. Em cada elo com fonte, o documento que o sustenta, curto e com a etiqueta de leitura: "Casamento de Zeferino, 1850 [V]". O elo sem fonte fica sem `documento`.
   - **Notas.** A `nota` de cada elo é a filiação do `estado.md` dele, e a `identidade`, a identidade, pela letra, de A a F: o sinal ou o adjetivo (`B+`, C forte) vai no rótulo do documento. O campo com `∅`, ou o elo sem estado, fica sem o campo. O desenho marca como furo a `nota` que falta; a `identidade` que falta ele não marca: ela vai na resposta do passo 5.
   - **Agrupar.** Irmãos que entram pelo mesmo documento viram uma pessoa só ("Zeferino e mais 3 irmãos"). A linhagem que passa de ~20 pessoas se parte em desenhos por trecho, com a pessoa da emenda repetida nos dois. Quem não muda a leitura do desenho fica de fora.
   - **Prova indireta.** O parente que prova entra como `colateral`, com a `relacao` que o documento declara: veja o triângulo em [indirect-proof.json](indirect-proof.json).

   Pronto quando o relatório da conversa lista as pessoas, o papel de cada uma e o que ficou agrupado ou de fora.
3. **Escreva a descrição** em `pesquisa/relatorios/<AAAA-MM-DD>-<assunto>.json`. Pronto quando cada pessoa e cada elo escolhidos estão no arquivo.
4. **Desenhe**, com o mesmo nome e a extensão de cada formato:

   ```sh
   garimpo diagrama <descrição>.json --saida <desenho>.mmd
   garimpo diagrama <descrição>.json --formato texto --saida <desenho>.txt
   garimpo diagrama <descrição>.json --formato svg --tema <tema> --saida <desenho>.svg
   ```

   O SVG só sai a pedido. Se ele falhar, entregue o Mermaid e o texto e repasse o que o erro manda instalar.

   O furo que é erro da descrição se corrige nela. O furo que é da linhagem fica no desenho: é o que o usuário precisa ver.

   Pronto quando o comando sai com 0 e cada furo listado está corrigido ou explicado na resposta.
5. **Entregue** o Mermaid num bloco `mermaid`, o texto simples para colar numa mensagem e, se pedido, o caminho do SVG. Diga em uma linha o que o desenho mostra: o elo mais fraco, os furos e cada elo com identidade `∅`, que o desenho não marca. Pronto quando a resposta traz os formatos pedidos.
6. **Feche** pelo [fechamento comum](../../docs/skills/fechamento.md).
