#!/bin/sh
# Teste real do garimpo: confere, só com leituras, que as rotas do FamilySearch
# ainda respondem como o garimpo espera. Roda sob demanda, com a sessão de quem
# roda, e nunca na integração contínua.
#
# Antes: entregue a sessão (`garimpo sessao entregar`) e, para o texto
# completo, abra a ponte (`garimpo ponte abrir`). Uso: scripts/real_test.sh
#
# Sai com o código do primeiro comando que falha: 3 é login necessário (peça o
# login ao usuário e rode de novo), 4 é bloqueio, 7 é ponte sem aba. São cerca
# de dez requisições, dentro do teto da sessão.
#
# Os alvos padrão são públicos e vêm das notas do projeto
# ianderso/familysearch-mcp (MIT): uma pessoa falecida em 1799, de perfil
# somente leitura, e um filme aberto. Troque-os pelas variáveis abaixo.
set -eu

garimpo=${GARIMPO:-cargo run --quiet --release --}
person=${GARIMPO_REAL_PESSOA:-KNDX-MKG}
film=${GARIMPO_REAL_FILME:-008126335}
group=${GARIMPO_REAL_GRUPO:-$film}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

step() {
    echo "== $1" >&2
}

step "sessão"
$garimpo sessao conferir

step "árvore: $person e os pais"
$garimpo fs pessoa "$person"
$garimpo fs pais "$person"

step "duas imagens do filme $film"
$garimpo imagens baixar "$film" --de 1 --ate 2 --pasta "$work/imagens"
downloaded=$(grep -c '"situacao":"baixada"' "$work/imagens/manifesto.jsonl" || true)
if [ "$downloaded" != 2 ]; then
    echo "teste real: esperava 2 imagens baixadas no manifesto, vieram $downloaded" >&2
    exit 1
fi

step "texto completo do grupo $group (pela ponte)"
pulled=$($garimpo busca grupo "$group" --saida "$work/grupo.jsonl" --cobertura "$work/cobertura.jsonl")
echo "$pulled"
# A linha traz as imagens no arquivo e o total do grupo: iguais e acima de zero.
# `cut -s` gives nothing for a line with no TAB, such as one of --json.
count=$(printf '%s' "$pulled" | cut -s -f1)
total=$(printf '%s' "$pulled" | cut -s -f2)
for number in "$count" "$total"; do
    case "$number" in
        '' | *[!0-9]*)
            echo "teste real: esperava imagens e total em números (rode sem --json), veio: $pulled" >&2
            exit 1
            ;;
    esac
done
if [ "$total" -lt 1 ] || [ "$count" != "$total" ]; then
    echo "teste real: o texto completo veio com $count de $total imagens" >&2
    exit 1
fi

echo "teste real: tudo certo" >&2
