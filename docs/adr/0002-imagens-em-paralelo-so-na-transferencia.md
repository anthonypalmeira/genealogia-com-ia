# Só a transferência das imagens corre em paralelo

O FamilySearch limita o uso por conta: dois ou três baixadores na mesma conta deram 429, e cerca de 1.500 imagens cheias em 30 minutos deram uma pausa de 10 minutos. Baixar um filme imagem por imagem, por outro lado, é lento, porque cada uma tem alguns megabytes. Decidimos que o `garimpo imagens baixar` faz os pedidos ao FamilySearch um por vez, pelo mesmo regulador das outras rotas (trava da conta, orçamento, teto da sessão, `Retry-After`), e que só a transferência dos bytes corre em paralelo.

Isso é possível porque a imagem em resolução original chega em dois passos: o FamilySearch responde ao pedido com um redirecionamento para um endereço assinado do armazenamento, e é desse endereço que os bytes vêm. O primeiro passo é rápido e conta no limite da conta; o segundo é demorado e já não é um pedido ao FamilySearch. Cada redirecionamento segura o pedido seguinte da conta por um intervalo (1,5 s por padrão, cerca de 40 imagens por minuto), abaixo do ritmo que já bloqueou uma conta.

## Opções consideradas

- **Vários pedidos ao FamilySearch ao mesmo tempo**: é o que deu 429 nas sessões medidas.
- **Tudo em série, com a transferência dentro da vez da conta**: seguro, mas a conta fica parada enquanto os bytes de outro servidor chegam, e as outras conversas esperam à toa.

## Consequências

- A sessão vai só no pedido ao FamilySearch. O endereço assinado já é a credencial: mandar a sessão junto faz o armazenamento recusar, e o endereço nunca aparece em mensagem de erro.
- Em `garimpo fs retratos`, o pedido de cada retrato também passa pelo regulador da conta e, quando o FamilySearch redireciona para a imagem, segura o pedido seguinte pelo intervalo das imagens. A transferência da miniatura vem da CDN, fora da vez da conta e sem a sessão.
- O ganho do paralelo é limitado pelo intervalo: com transferências mais curtas que ele, o download anda no ritmo do intervalo.
- As miniaturas de filme vêm na própria resposta, sem redirecionamento: seguem em série, sem o intervalo, e contam no teto da sessão.
- Os endereços do armazenamento vêm das notas de outro projeto e não foram conferidos com a conta logada: veja o teste real em `scripts/real_test.sh`.
