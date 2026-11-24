"""Testes da checagem do Markdown: cada um monta um repositório pequeno numa pasta temporária."""

import tempfile
import unittest
from pathlib import Path

from checar_docs import Commands, checar, cli_commands

# The command tree of a small made-up CLI: two groups, three commands.
COMMANDS: Commands = {"fs": {"pessoa": {}, "pais": {}}, "sessao": {"conferir": {}}}

# What a made-up CLI prints for `--help`, by command path, in clap's layout.
HELP = {
    (): (
        "CLI para agentes.\n\nUsage: garimpo [OPTIONS] <COMMAND>\n\n"
        "Commands:\n"
        "  sessao  A sessão: conferir\n"
        "  fs      A árvore pelas rotas oficiais: ler, percorrer\n"
        "          e gravar\n"
        "  help    Print this message or the help of the given subcommand(s)\n"
        "\n"
        "Options:\n"
        "  -h, --help\n"
        "          Print help\n"
    ),
    ("sessao",): (
        "Usage: garimpo sessao <COMMAND>\n\n"
        "Commands:\n"
        "  conferir  Confere a sessão\n"
        "  help      Print this message\n"
    ),
    ("fs",): (
        "Usage: garimpo fs <COMMAND>\n\n"
        "Commands:\n"
        "  pessoa         Uma pessoa, numa linha\n"
        "  catalogo-item  Um item do catálogo\n"
    ),
    ("sessao", "conferir"): "Usage: garimpo sessao conferir [OPTIONS]\n",
    ("fs", "pessoa"): "Usage: garimpo fs pessoa [OPTIONS] <ID>\n",
    ("fs", "catalogo-item"): (
        "Usage: garimpo fs catalogo-item <ID>\n\nArguments:\n  <ID>\n"
    ),
}


class ChecarDocs(unittest.TestCase):
    def setUp(self) -> None:
        pasta = tempfile.TemporaryDirectory()
        self.addCleanup(pasta.cleanup)
        self.raiz = Path(pasta.name)

    def problemas(self, arquivos: dict[str, str]) -> list[tuple[str, int]]:
        """Grava os arquivos, roda a checagem e devolve (arquivo, linha) de cada problema."""
        for nome, texto in arquivos.items():
            destino = self.raiz / nome
            destino.parent.mkdir(parents=True, exist_ok=True)
            destino.write_text(texto, encoding="utf-8")
        markdown = [Path(nome) for nome in arquivos if nome.endswith(".md")]
        return [
            (str(p.arquivo), p.linha) for p in checar(self.raiz, markdown, COMMANDS)
        ]

    def test_link_para_arquivo_que_nao_existe_e_apontado(self) -> None:
        achados = self.problemas(
            {"AGENTS.md": "# Mapa\n\nVeja o [guia](docs/guias/fontes.md).\n"}
        )
        self.assertEqual(achados, [("AGENTS.md", 3)])

    def test_links_validos_externos_e_dentro_de_codigo_passam(self) -> None:
        achados = self.problemas(
            {
                "AGENTS.md": "# Mapa\n",
                "docs/guias/fontes.md": (
                    "# Fontes\n\n## Minas Gerais\n\n## Como usar `garimpo`\n"
                    "\n## Pedido\n\n## Pedido\n"
                ),
                "docs/guias/estrategias.md": (
                    "# Estratégias, passo a passo\n"
                    "Veja [fontes](fontes.md#minas-gerais), [o mapa](/AGENTS.md),\n"
                    "[a CLI](fontes.md#como-usar-garimpo), [o 2.º](fontes.md#pedido-1),\n"
                    "[o site](https://example.org/a.md) e\n"
                    "[o topo](#estratégias-passo-a-passo).\n"
                    "\n"
                    "```md\n"
                    "[exemplo](nao-existe.md)\n"
                    "```\n"
                    "\n"
                    "Escreva `[texto](caminho.md)`.\n"
                ),
            }
        )
        self.assertEqual(achados, [])

    def test_ancora_que_nao_existe_e_apontada(self) -> None:
        achados = self.problemas(
            {
                "docs/guias/fontes.md": "# Fontes\n\n## Minas Gerais\n",
                "docs/guias/estrategias.md": (
                    "# Estratégias\n"
                    "Veja [São Paulo](fontes.md#são-paulo).\n"
                    "Veja a [regra de parada](#regra-de-parada).\n"
                ),
            }
        )
        self.assertEqual(
            achados,
            [("docs/guias/estrategias.md", 2), ("docs/guias/estrategias.md", 3)],
        )

    def test_skill_sem_frontmatter_ou_sem_description_e_apontada(self) -> None:
        achados = self.problemas(
            {
                "skills/pedido/SKILL.md": "---\nname: pedido\n---\n\n# Pedido\n",
                "skills/diagrama/SKILL.md": "# Diagrama\n",
            }
        )
        self.assertEqual(
            achados, [("skills/pedido/SKILL.md", 1), ("skills/diagrama/SKILL.md", 1)]
        )

    def test_skill_com_nome_diferente_da_pasta_e_apontada(self) -> None:
        achados = self.problemas(
            {
                "skills/pedido/SKILL.md": (
                    "---\nname: pedidos\ndescription: Redige pedidos curtos.\n---\n"
                )
            }
        )
        self.assertEqual(achados, [("skills/pedido/SKILL.md", 2)])

    def test_valor_que_quebra_o_yaml_e_apontado(self) -> None:
        # `: ` sem aspas e aspas sem fechar quebram o YAML, e a skill não carrega.
        achados = self.problemas(
            {
                "skills/pedido/SKILL.md": (
                    "---\n"
                    "name: pedido\n"
                    "description: Redige pedidos a arquivos: cartas e e-mails.\n"
                    "---\n"
                ),
                "skills/diagrama/SKILL.md": (
                    "---\n"
                    "name: diagrama\n"
                    'description: "Desenha linhagens: árvores e provas."\n'
                    "disable-model-invocation: true\n"
                    "---\n"
                ),
                "skills/caminhos/SKILL.md": (
                    '---\nname: "caminhos"\ndescription: "Varre a árvore.\n---\n'
                ),
                "skills/perfil/SKILL.md": (
                    '---\nname: "perfil""\ndescription: Configura a pesquisa.\n---\n'
                ),
            }
        )
        self.assertEqual(
            achados,
            [
                ("skills/pedido/SKILL.md", 3),
                ("skills/caminhos/SKILL.md", 3),
                ("skills/perfil/SKILL.md", 2),
            ],
        )

    def test_cpf_email_e_telefone_sao_apontados(self) -> None:
        achados = self.problemas(
            {
                "docs/guias/pedido.md": (
                    "# Modelo de pedido\n"
                    "CPF do requerente: 123.456.789-09\n"
                    "Responder para fulano.tal@exemplo.com.br\n"
                    "Telefone do cartório: (31) 3555-0100\n"
                    "WhatsApp: 31 95555-0100\n"
                    "Só os dígitos: 12345678909\n"
                    "Fixo sem hífen: (31) 3555 0100 ou 31 35550100\n"
                    "Link de conversa: wa.me/5531955550100\n"
                )
            }
        )
        self.assertEqual(
            achados, [("docs/guias/pedido.md", linha) for linha in range(2, 9)]
        )

    def test_anos_cotas_e_versoes_nao_sao_dados_pessoais(self) -> None:
        achados = self.problemas(
            {
                "docs/guias/fontes.md": (
                    "Registros da Igreja Católica, 1706-1999; listas de 1831-1832.\n"
                    "Filme 001234567, coleção 7654321, pesquisa de 2026-11-23.\n"
                    "Fixe a versão: `react@18.3.1`. Decreto 181/1890. R$ 12,15.\n"
                    "Em 12 volumes, 1562-1736.\n"
                )
            }
        )
        self.assertEqual(achados, [])

    def test_cli_command_that_does_not_exist_is_reported(self) -> None:
        found = self.problemas(
            {
                "docs/skills/abertura.md": (
                    "# Abertura\n"
                    "Leia os retratos com `garimpo fs retratos <ID>`.\n"
                    "\n"
                    "```sh\n"
                    "garimpo sessao conferir && garimpo arvore subir\n"
                    "garimpo --json fs pessoa ID\n"
                    "garimpo --teto-sessao 10 fs retratos ID\n"
                    "```\n"
                )
            }
        )
        self.assertEqual(
            found, [("docs/skills/abertura.md", line) for line in (2, 5, 6, 7)]
        )

    def test_cli_commands_with_arguments_and_prose_about_the_cli_pass(self) -> None:
        found = self.problemas(
            {
                "docs/skills/abertura.md": (
                    "# Abertura\n"
                    "O garimpo espera sozinho as pausas.\n"
                    "O `garimpo` grava a busca.\n"
                    "Rode `garimpo fs pessoa <ID> --json` e `garimpo fs pais ID`.\n"
                    "O grupo `garimpo fs` lê a árvore: `garimpo fs <comando> --help`.\n"
                    "A ajuda: `garimpo --help` e `garimpo fs --help`.\n"
                    "O estado fica em `~/.garimpo`, ou em `GARIMPO_DIR`.\n"
                    "\n"
                    "```sh\n"
                    "# Confere a sessão antes de subir.\n"
                    'printf %s "$TOKEN" | garimpo sessao conferir\n'
                    "```\n"
                )
            }
        )
        self.assertEqual(found, [])

    def test_command_tree_is_read_from_the_help_of_each_command(self) -> None:
        self.assertEqual(
            cli_commands(HELP.__getitem__),
            {
                "sessao": {"conferir": {}, "help": {}},
                "fs": {"pessoa": {}, "catalogo-item": {}},
                "help": {},
            },
        )


if __name__ == "__main__":
    unittest.main()
