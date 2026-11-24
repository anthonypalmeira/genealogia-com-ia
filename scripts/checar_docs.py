"""Checagem mecânica do Markdown do repositório.

Confere quatro coisas: os links relativos entre documentos resolvem (arquivo e
âncora), o frontmatter de cada SKILL.md é válido, todo comando do `garimpo`
citado como código existe na ajuda da CLI, com as opções que leva, e nenhum
arquivo traz CPF, e-mail ou telefone.

Uso: python3 scripts/checar_docs.py (compila o `garimpo` com o cargo)
Sai com 1 e lista `arquivo:linha: problema` quando acha algo.
"""

import json
import re
import subprocess
import sys
from collections import Counter
from collections.abc import Callable, Iterable, Iterator, Mapping, Sequence
from dataclasses import dataclass
from functools import partial
from pathlib import Path
from urllib.parse import unquote

LINK = re.compile(r"\]\(([^)\s]+)[^)]*\)")
TITULO = re.compile(r"#+\s+(.*)")
CAMPO = re.compile(r"([A-Za-z][\w-]*):\s*(.*)")
# Valor de uma linha: entre aspas (sem a mesma aspa dentro) ou sem aspas.
VALOR = re.compile(r"""\"([^"]*)\"|'([^']*)'|([^"'].*)""")
CODIGO_NA_LINHA = re.compile(r"`[^`]*`")
COM_ESQUEMA = re.compile(r"[a-z][a-z0-9+.-]*:", re.IGNORECASE)
# A quoted argument: a value, whatever it holds.
QUOTED = re.compile(r"\"[^\"]*\"|'[^']*'")
# `garimpo`, then the words that follow it, up to the first that is neither an
# option nor a bare word, such as a placeholder, and then the rest of the
# invocation, with each quoted argument whole, up to the end of the code, of
# the subshell or of the command.
CLI_CITATION = re.compile(
    r"(?<![\w./-])garimpo((?: +(?:-[\w=-]+|[a-z][a-z0-9-]*))*)"
    rf"((?:{QUOTED.pattern}|[^`|;&)\"'])*)"
)
# An option among the arguments of an invocation, up to its value or to a
# redirection.
CITED_OPTION = re.compile(r"(?<!\S)--?[A-Za-z][^\s=<>]*")
# One entry of the `Commands:` list of a help text; a wrapped description is
# indented deeper.
SUBCOMMAND = re.compile(r"^  ([a-z][a-z0-9-]*)", re.MULTILINE)
# One entry of an options list of a help text, with its short form when it has
# one; the description comes below it, indented deeper.
OPTION = re.compile(r"^ {2,6}(?:(-[A-Za-z]), )?(--[a-z][a-z0-9-]*)", re.MULTILINE)
# O telefone exige DDD para não confundir com intervalos de anos (1831-1840).
DADOS_PESSOAIS = {
    "CPF": re.compile(r"(?<!\d)\d{3}\.?\d{3}\.?\d{3}-?\d{2}(?!\d)"),
    "e-mail": re.compile(r"[\w.+-]+@[\w-]+(\.[\w-]+)*\.[A-Za-z]{2,}"),
    "telefone": re.compile(
        r"(?<!\d)(?:\+?55 ?\d{2} ?|\(\d{2}\) ?|\d{2}[ .-])9?\d{4}[ .-]?\d{4}(?!\d)"
    ),
}


@dataclass(frozen=True)
class Command:
    """A CLI command, as its help describes it: the options it takes, in the
    short and the long form, and its subcommands by name."""

    options: frozenset[str]
    subcommands: Mapping[str, "Command"]


@dataclass(frozen=True)
class Problema:
    arquivo: Path
    linha: int
    mensagem: str

    def __str__(self) -> str:
        return f"{self.arquivo}:{self.linha}: {self.mensagem}"


def checar(raiz: Path, arquivos: Iterable[Path], cli: Command) -> Iterator[Problema]:
    """Confere cada arquivo Markdown, dado por caminho relativo à raiz."""
    for arquivo in arquivos:
        linhas = (raiz / arquivo).read_text(encoding="utf-8").splitlines()
        yield from links_quebrados(raiz, arquivo, linhas)
        yield from wrong_citations(arquivo, linhas, cli)
        for numero, linha in enumerate(linhas, start=1):
            for tipo, padrao in DADOS_PESSOAIS.items():
                if padrao.search(linha):
                    yield Problema(
                        arquivo,
                        numero,
                        f"parece {tipo}: troque por um campo a preencher",
                    )
        if arquivo.name == "SKILL.md":
            yield from frontmatter_invalido(arquivo, linhas)


def fora_de_blocos(linhas: Iterable[str]) -> Iterator[tuple[int, str]]:
    """Linhas numeradas, sem os blocos de código."""
    em_bloco = False
    for numero, linha in enumerate(linhas, start=1):
        if linha.lstrip().startswith(("```", "~~~")):
            em_bloco = not em_bloco
        elif not em_bloco:
            yield numero, linha


def ancoras(linhas: Iterable[str]) -> Iterator[str]:
    """Âncora de cada título, como o GitHub gera: minúsculas, sem pontuação, com
    hífens, e um sufixo numerado quando o título se repete."""
    vistas: Counter[str] = Counter()
    for _, linha in fora_de_blocos(linhas):
        if titulo := TITULO.match(linha):
            ancora = re.sub(r"[^\w -]", "", titulo[1].strip().lower()).replace(" ", "-")
            yield f"{ancora}-{vistas[ancora]}" if vistas[ancora] else ancora
            vistas[ancora] += 1


def links_quebrados(
    raiz: Path, arquivo: Path, linhas: Sequence[str]
) -> Iterator[Problema]:
    """Links relativos precisam apontar para um arquivo e uma âncora que existem."""
    for numero, linha in fora_de_blocos(linhas):
        for alvo in LINK.findall(CODIGO_NA_LINHA.sub("", linha)):
            if COM_ESQUEMA.match(alvo):
                continue
            caminho, _, ancora = unquote(alvo).partition("#")
            base = raiz if caminho.startswith("/") else raiz / arquivo.parent
            destino = base / caminho.lstrip("/") if caminho else raiz / arquivo
            if not destino.exists():
                yield Problema(arquivo, numero, f"link quebrado: {alvo}")
            elif ancora and destino.suffix == ".md":
                titulos = destino.read_text(encoding="utf-8").splitlines()
                if ancora not in ancoras(titulos):
                    yield Problema(arquivo, numero, f"âncora quebrada: {alvo}")


def wrong_citations(
    file: Path, lines: Sequence[str], cli: Command
) -> Iterator[Problema]:
    """Every `garimpo …` written as code must name a command the CLI has, with
    options that the command takes."""
    outside = {number for number, _ in fora_de_blocos(lines)}
    # The start of an invocation that a `\` at the end of the line carries on
    # to the next one, and the line it began on.
    carried, start = "", 0
    for number, line in enumerate(lines, start=1):
        # Inside a fenced block the whole line is code; outside, only the spans.
        code = "".join(CODIGO_NA_LINHA.findall(line)) if number in outside else line
        start = start if carried else number
        if number not in outside and code.endswith("\\"):
            carried += code[:-1]
            continue
        code, carried = carried + code, ""
        for words, rest in CLI_CITATION.findall(code):
            command, name = cli, "garimpo"
            for word in words.split():
                # After a command without subcommands come its arguments.
                if not command.subcommands:
                    break
                # An option here may take a value, which would read as a
                # command: only the help options may come before the command.
                if word in ("-h", "--help", "-V", "--version"):
                    break
                if word not in command.subcommands:
                    yield Problema(
                        file,
                        start,
                        f"`{word}` vem antes do comando: ponha as opções depois dele"
                        if word.startswith("-")
                        else f"o garimpo não tem o comando `{word}`",
                    )
                    break
                command, name = command.subcommands[word], f"{name} {word}"
            # The options of a group that stands before a placeholder belong to
            # a command that the citation does not name.
            if not command.subcommands:
                for option in CITED_OPTION.findall(words + QUOTED.sub('""', rest)):
                    if option not in command.options:
                        yield Problema(
                            file, start, f"`{name}` não tem a opção `{option}`"
                        )


def read_command(
    help_of: Callable[[tuple[str, ...]], str], path: tuple[str, ...] = ()
) -> Command:
    """The command at a path of the CLI, the CLI itself by default, with its
    command tree: read from the `Commands:` list and from the options that the
    help of each command prints. `help_of` gives the help text of a path."""
    text = help_of(path)
    listing = text.partition("\nCommands:\n")[2].partition("\n\n")[0]
    return Command(
        frozenset(form for forms in OPTION.findall(text) for form in forms if form),
        {
            # `help` takes a command path as its arguments, not subcommands.
            name: Command(frozenset(), {})
            if name == "help"
            else read_command(help_of, (*path, name))
            for name in SUBCOMMAND.findall(listing)
        },
    )


def frontmatter_invalido(arquivo: Path, linhas: Sequence[str]) -> Iterator[Problema]:
    """Uma skill abre com um bloco `---` que traz `name` e `description` numa linha cada."""
    if not linhas or linhas[0] != "---" or "---" not in linhas[1:]:
        yield Problema(arquivo, 1, "frontmatter ausente ou sem o `---` final")
        return
    campos = {
        campo[1]: (numero, campo[2].strip())
        for numero, linha in enumerate(linhas[: linhas.index("---", 1)], start=1)
        if (campo := CAMPO.match(linha))
    }
    for chave in ("name", "description"):
        numero, bruto = campos.get(chave, (1, ""))
        partes = VALOR.fullmatch(bruto)
        valor = next(filter(None, partes.groups()), "") if partes else ""
        if bruto and not partes:
            yield Problema(arquivo, numero, f"`{chave}` com aspas sem par")
        elif not valor:
            yield Problema(arquivo, numero, f"frontmatter sem `{chave}`")
        elif partes and partes[3] and ": " in valor:
            yield Problema(arquivo, numero, f"`{chave}` com `: ` precisa de aspas")
        elif chave == "name" and valor != arquivo.parent.name:
            yield Problema(
                arquivo, numero, f"`name` difere da pasta `{arquivo.parent.name}`"
            )


def main() -> int:
    raiz = Path(__file__).resolve().parent.parent
    run = partial(subprocess.run, check=True, capture_output=True, text=True, cwd=raiz)
    # O git lista os rastreados e os novos, sem o que o .gitignore exclui.
    listagem = run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "*.md"]
    ).stdout
    # Um arquivo apagado e ainda não commitado continua na lista do git.
    arquivos = sorted(
        Path(nome) for nome in listagem.split("\0") if nome and (raiz / nome).exists()
    )
    # The CLI is built once and run directly: `cargo run` takes over half a
    # second per help text, and there is one per command.
    run(["cargo", "build", "--quiet", "--locked"])
    metadata = run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"]
    ).stdout
    # Cargo's own JSON, trusted as it comes.
    garimpo = Path(json.loads(metadata)["target_directory"], "debug", "garimpo")
    cli = read_command(lambda path: run([garimpo, *path, "--help"]).stdout)
    problemas = list(checar(raiz, arquivos, cli))
    for problema in problemas:
        print(problema)
    return 1 if problemas else 0


if __name__ == "__main__":
    sys.exit(main())
