//! The SVG back end: the Mermaid drawing rendered by Pele, an external
//! renderer used only to draw, with the colors of a theme.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{Context, anyhow};

use crate::{
    cli::{DiagramArgs, Theme},
    error::Error,
};

/// The colors and the font of a theme, as the CSS variables Pele reads.
fn css(theme: Theme) -> &'static str {
    match theme {
        Theme::Plain => {
            ".pele{--pele-bg:#ffffff;--pele-fg:#111827;--pele-muted:#6b7280;--pele-line:#6b7280;\
             --pele-surface:#f9fafb;--pele-border:#d1d5db;--pele-font:sans-serif}"
        }
        Theme::Parchment => {
            ".pele{--pele-bg:#f5ecd7;--pele-fg:#3b2f1e;--pele-muted:#7a6248;--pele-line:#7a6248;\
             --pele-surface:#efe2c4;--pele-border:#b9a27f;--pele-font:Georgia,serif;\
             --pele-radius:2px}"
        }
        Theme::Blueprint => {
            ".pele{--pele-bg:#0b3a66;--pele-fg:#e6f1fb;--pele-muted:#9cc4e4;--pele-line:#9cc4e4;\
             --pele-surface:#0f4a80;--pele-border:#9cc4e4;--pele-font:monospace;\
             --pele-radius:0px}"
        }
        Theme::FamilySearch => {
            ".pele{--pele-bg:#ffffff;--pele-fg:#202121;--pele-muted:#5f6265;--pele-line:#878a8c;\
             --pele-surface:#f4f4f4;--pele-border:#cdd0d2;--pele-font:'Noto Sans',sans-serif;\
             --pele-radius:8px}"
        }
    }
}

/// Renders the Mermaid drawing as a themed SVG, with the kit's renderer.
pub(super) fn render(mermaid: &str, args: &DiagramArgs) -> Result<String, Error> {
    let setup = "o SVG precisa do `bun` e do Pele: rode `mise install` no kit e `bun install \
                 --frozen-lockfile` na pasta `svg/` dele; `--formato mermaid` não precisa de nada";
    let mut renderer = Command::new("bun")
        .arg(&args.renderer)
        // The drawing has no background of its own: it takes the theme's.
        .arg(format!(
            "{}.pele{{background:var(--pele-bg)}}",
            css(args.theme)
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("não consegui rodar o `bun`; {setup}"))?;
    renderer
        .stdin
        .take()
        .expect("the renderer's stdin is a pipe")
        .write_all(mermaid.as_bytes())
        .context("não consegui entregar o desenho ao renderizador")?;
    let rendered = renderer
        .wait_with_output()
        .context("não consegui esperar o renderizador")?;
    if !rendered.status.success() {
        return Err(anyhow!(
            "o renderizador {} falhou: {}; {setup}",
            args.renderer.display(),
            String::from_utf8_lossy(&rendered.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(rendered.stdout).context("o renderizador não devolveu texto")?)
}
