"""Checagem mecânica do Markdown do repositório.

Confere três coisas: os links relativos entre documentos resolvem (arquivo e
âncora), o frontmatter de cada SKILL.md é válido e nenhum arquivo traz CPF,
e-mail ou telefone.

Uso: python3 scripts/checar_docs.py
Sai com 1 e lista `arquivo:linha: problema` quando acha algo.
"""

import re
import subprocess
import sys
from collections import Counter
from collections.abc import Iterable, Iterator, Sequence
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote

LINK = re.compile(r"\]\(([^)\s]+)[^)]*\)")
TITULO = re.compile(r"#+\s+(.*)")
CAMPO = re.compile(r"([A-Za-z][\w-]*):\s*(.*)")
# Valor de uma linha: entre aspas (sem a mesma aspa dentro) ou sem aspas.
VALOR = re.compile(r"""\"([^"]*)\"|'([^']*)'|([^"'].*)""")
CODIGO_NA_LINHA = re.compile(r"`[^`]*`")
COM_ESQUEMA = re.compile(r"[a-z][a-z0-9+.-]*:", re.IGNORECASE)
# O telefone exige DDD para não confundir com intervalos de anos (1831-1840).
DADOS_PESSOAIS = {
    "CPF": re.compile(r"(?<!\d)\d{3}\.?\d{3}\.?\d{3}-?\d{2}(?!\d)"),
    "e-mail": re.compile(r"[\w.+-]+@[\w-]+(\.[\w-]+)*\.[A-Za-z]{2,}"),
    "telefone": re.compile(
        r"(?<!\d)(?:\+?55 ?\d{2} ?|\(\d{2}\) ?|\d{2}[ .-])9?\d{4}[ .-]?\d{4}(?!\d)"
    ),
}


@dataclass(frozen=True)
class Problema:
    arquivo: Path
    linha: int
    mensagem: str

    def __str__(self) -> str:
        return f"{self.arquivo}:{self.linha}: {self.mensagem}"


def checar(raiz: Path, arquivos: Iterable[Path]) -> Iterator[Problema]:
    """Confere cada arquivo Markdown, dado por caminho relativo à raiz."""
    for arquivo in arquivos:
        linhas = (raiz / arquivo).read_text(encoding="utf-8").splitlines()
        yield from links_quebrados(raiz, arquivo, linhas)
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
    # O git lista os rastreados e os novos, sem o que o .gitignore exclui.
    listagem = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "*.md"],
        check=True,
        capture_output=True,
        text=True,
        cwd=raiz,
    ).stdout
    # Um arquivo apagado e ainda não commitado continua na lista do git.
    arquivos = sorted(
        Path(nome) for nome in listagem.split("\0") if nome and (raiz / nome).exists()
    )
    problemas = list(checar(raiz, arquivos))
    for problema in problemas:
        print(problema)
    return 1 if problemas else 0


if __name__ == "__main__":
    sys.exit(main())
