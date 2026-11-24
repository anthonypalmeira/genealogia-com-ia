---
name: caminhos
description: "Varre a árvore atrás de ancestrais-alvo e ordena os caminhos pela força das provas. Use para 'chego a algum…?'."
---

# Caminhos

Diz se a pessoa de partida chega a algum ancestral-alvo e por onde. A árvore só indica os caminhos: a prova de cada elo vem da graduação da skill `validar-linhagem`.

**Argumentos:** o alvo, o limite e a pessoa de partida, que por padrão é a pessoa raiz do perfil.

## Passos

1. **Abra** pela [abertura comum](../../docs/skills/abertura.md).
2. **Fixe o alvo e o limite.**
   - O alvo é uma **lista** que o usuário fornece (nomes ou IDs, na mensagem ou num arquivo) ou um **critério**: um fato, um lugar ou um selo de retrato.
   - O limite é um número de gerações ou um ano de corte. Sem limite no pedido, o corte é o ano de nascimento do alvo mais antigo; sem data nenhuma, 8 gerações.

   Pronto quando o alvo e o limite estão escritos, uma linha cada, no relatório da conversa.
3. **Varra a árvore** em largura, para um arquivo da pasta da pesquisa, até o limite do passo 2:

   ```sh
   mkdir -p pesquisa/varreduras
   # Limite em gerações:
   garimpo fs ascendencia <ID> --geracoes <N> --saida pesquisa/varreduras/ascendencia-<ID>.jsonl
   # Limite em ano de corte:
   garimpo fs ascendencia <ID> --ano-corte <ANO> --saida pesquisa/varreduras/ascendencia-<ID>-corte-<ANO>.jsonl
   ```

   - Repetir o comando com `--geracoes` maior aprofunda a mesma varredura.
   - Varredura interrompida por login, bloqueio ou teto é pendente de acesso, não resposta.
   - Cada pessoa que o comando avisa que o FamilySearch não devolveu é um ramo por verificar: registre-a como pendente de acesso.

   Pronto quando o comando sai com 0.
4. **Ache os alvos na varredura.** A tabela das pessoas, uma por linha, com a geração, os nomes e os fatos que a árvore dá:

   ```sh
   garimpo fs varredura pesquisa/varreduras/<arquivo da varredura>
   ```

   - **Limite:** valem as pessoas da geração 1 em diante, até a geração N ou nascidas do ano de corte em diante, mesmo que o arquivo vá além. Com ano de corte, sem data de nascimento vale a de batismo, e a pessoa sem nenhuma das duas entra com a marca "sem data".
   - **Lista:** case pelo ID. Sem ID, procure cada grafia do nome e compare datas e lugares: o que casar é candidato, e a identidade dele se gradua no passo 6.
   - **Fato ou lugar:** filtre pelo critério as pessoas dentro do limite.
   - **Selo de retrato:** o selo só aparece na imagem do retrato. Baixe os retratos das pessoas da varredura e junte-os em folhas:

     ```sh
     garimpo fs retratos pesquisa/varreduras/<arquivo da varredura> --pasta pesquisa/varreduras/retratos
     garimpo imagens folha <retratos, em ordem> --saida pesquisa/varreduras/folha-retratos-<n>.jpg
     ```

     Ache nas folhas os retratos com o selo e confirme cada um no retrato original. Quem tem o selo segue como lista, pelo ID. `fs retratos` interrompido por login, bloqueio ou teto deixa o critério pendente de acesso, como a varredura; quem ele dá como pendente de acesso entra nos ramos por verificar.

   Pronto quando cada alvo da lista está como achado (com o ID), candidato ou "não aparece na árvore até o limite", e o critério passou por todas as pessoas dentro do limite.
5. **Liste os caminhos**, com as datas impossíveis marcadas:

   ```sh
   garimpo fs caminhos pesquisa/varreduras/<arquivo da varredura> <ID do alvo>…
   ```

   O elo que sai com aviso tem a data ou o parentesco errado na árvore: continua no caminho, com a marca.

   Pronto quando cada alvo achado tem todos os seus caminhos listados.
6. **Gradue cada caminho.** Registre cada caminho como linhagem e gradue os elos dele pelos passos da skill `validar-linhagem`, dos elos às decisões do pesquisador; a abertura, a entrega e o fechamento são os desta skill. Antes de graduar um elo novo, retome o estado dele pelo passo Estado da abertura. Um elo que serve a vários caminhos é graduado uma vez. Num alvo de lista, gradue também a identidade dele, pelas mesmas âncoras, e registre-a no estado do último elo: a pessoa da árvore é a da lista? Pronto quando cada elo de cada caminho tem as duas notas.
7. **Entregue** os caminhos ordenados pela nota mais baixa de cada um, de filiação ou de identidade, da mais alta à mais baixa; no empate, vem antes o caminho com menos elos nessa nota:

   | Alvo | Caminho | Elo mais fraco | Datas impossíveis |
   |---|---|---|---|
   | Nome completo. Na lista: achado pelo ID ou candidato, e a nota de identidade. No critério: o fato ou o lugar que casou | Nomes completos, da pessoa de partida ao alvo | O elo e a nota | O elo e o aviso |

   Um alvo que não foi achado "não aparece na árvore até o limite": a árvore é incompleta, e a resposta diz só isso. A resposta cita sempre os ramos por verificar.

   Pronto quando a tabela tem uma linha por caminho e cada alvo da lista tem resposta.
8. **Feche** pelo [fechamento comum](../../docs/skills/fechamento.md).
