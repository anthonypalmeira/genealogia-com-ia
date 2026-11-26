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
HEADING = re.compile(r"#+\s+(.*)")
FIELD = re.compile(r"([A-Za-z][\w-]*):\s*(.*)")
# A single-line value may be unquoted or enclosed in matching quotes.
FIELD_VALUE = re.compile(r"""\"([^"]*)\"|'([^']*)'|([^"'].*)""")
INLINE_CODE = re.compile(r"`[^`]*`")
URI_SCHEME = re.compile(r"[a-z][a-z0-9+.-]*:", re.IGNORECASE)
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
# Require an area code to distinguish phone numbers from year ranges (1831-1840).
PERSONAL_DATA = {
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
class Problem:
    file: Path
    line: int
    message: str

    def __str__(self) -> str:
        return f"{self.file}:{self.line}: {self.message}"


def check(root: Path, files: Iterable[Path], cli: Command) -> Iterator[Problem]:
    """Check each Markdown file given by its path relative to the root."""
    for file in files:
        lines = (root / file).read_text(encoding="utf-8").splitlines()
        yield from broken_links(root, file, lines)
        yield from wrong_citations(file, lines, cli)
        for number, line in enumerate(lines, start=1):
            for kind, pattern in PERSONAL_DATA.items():
                if pattern.search(line):
                    yield Problem(
                        file,
                        number,
                        f"parece {kind}: troque por um campo a preencher",
                    )
        if file.name == "SKILL.md":
            yield from invalid_frontmatter(file, lines)


def outside_code_blocks(lines: Iterable[str]) -> Iterator[tuple[int, str]]:
    """Yield numbered lines outside fenced code blocks."""
    in_block = False
    for number, line in enumerate(lines, start=1):
        if line.lstrip().startswith(("```", "~~~")):
            in_block = not in_block
        elif not in_block:
            yield number, line


def anchors(lines: Iterable[str]) -> Iterator[str]:
    """Yield GitHub heading anchors with numbered suffixes for repeated headings."""
    seen: Counter[str] = Counter()
    for _, line in outside_code_blocks(lines):
        if heading := HEADING.match(line):
            anchor = re.sub(r"[^\w -]", "", heading[1].strip().lower()).replace(
                " ", "-"
            )
            yield f"{anchor}-{seen[anchor]}" if seen[anchor] else anchor
            seen[anchor] += 1


def broken_links(root: Path, file: Path, lines: Sequence[str]) -> Iterator[Problem]:
    """Report relative links whose file or anchor is missing."""
    for number, line in outside_code_blocks(lines):
        for target in LINK.findall(INLINE_CODE.sub("", line)):
            if URI_SCHEME.match(target):
                continue
            path, _, anchor = unquote(target).partition("#")
            base = root if path.startswith("/") else root / file.parent
            destination = base / path.lstrip("/") if path else root / file
            if not destination.exists():
                yield Problem(file, number, f"link quebrado: {target}")
            elif anchor and destination.suffix == ".md":
                target_lines = destination.read_text(encoding="utf-8").splitlines()
                if anchor not in anchors(target_lines):
                    yield Problem(file, number, f"âncora quebrada: {target}")


def wrong_citations(
    file: Path, lines: Sequence[str], cli: Command
) -> Iterator[Problem]:
    """Every `garimpo …` written as code must name a command the CLI has, with
    options that the command takes."""
    outside = {number for number, _ in outside_code_blocks(lines)}
    # The start of an invocation that a `\` at the end of the line carries on
    # to the next one, and the line it began on.
    carried, start = "", 0
    for number, line in enumerate(lines, start=1):
        # Inside a fenced block the whole line is code; outside, only the spans.
        code = "".join(INLINE_CODE.findall(line)) if number in outside else line
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
                    yield Problem(
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
                        yield Problem(
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


def invalid_frontmatter(file: Path, lines: Sequence[str]) -> Iterator[Problem]:
    """A skill starts with a fenced frontmatter block and single-line name and description."""
    if not lines or lines[0] != "---" or "---" not in lines[1:]:
        yield Problem(file, 1, "frontmatter ausente ou sem o `---` final")
        return
    fields = {
        field[1]: (number, field[2].strip())
        for number, line in enumerate(lines[: lines.index("---", 1)], start=1)
        if (field := FIELD.match(line))
    }
    for key in ("name", "description"):
        number, raw = fields.get(key, (1, ""))
        parts = FIELD_VALUE.fullmatch(raw)
        value = next(filter(None, parts.groups()), "") if parts else ""
        if raw and not parts:
            yield Problem(file, number, f"`{key}` com aspas sem par")
        elif not value:
            yield Problem(file, number, f"frontmatter sem `{key}`")
        elif parts and parts[3] and ": " in value:
            yield Problem(file, number, f"`{key}` com `: ` precisa de aspas")
        elif key == "name" and value != file.parent.name:
            yield Problem(file, number, f"`name` difere da pasta `{file.parent.name}`")


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    run = partial(subprocess.run, check=True, capture_output=True, text=True, cwd=root)
    # Include tracked and new files, respecting .gitignore.
    listing = run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "*.md"]
    ).stdout
    # A tracked file deleted from the working tree still appears in the list.
    files = sorted(
        Path(name) for name in listing.split("\0") if name and (root / name).exists()
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
    problems = list(check(root, files, cli))
    for problem in problems:
        print(problem)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
