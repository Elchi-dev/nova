use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

use nova_compiler::ast::{Parameter, Statement, TypeExpr};

pub fn execute(path: PathBuf, open: bool) -> Result<(), Box<dyn std::error::Error>> {
    let files = collect_nova_files(&path)?;

    if files.is_empty() {
        println!("{}", "no .nova or .nv files found".yellow().dimmed());
        return Ok(());
    }

    // Output directory
    let out_dir = if path.is_dir() {
        path.join("docs").join("generated")
    } else {
        PathBuf::from("docs").join("generated")
    };
    fs::create_dir_all(&out_dir)?;

    let mut all_items: Vec<DocItem> = Vec::new();

    for file in &files {
        let source = fs::read_to_string(file)?;
        let tokens = nova_compiler::lexer::tokenize(&source)?;
        let program = nova_compiler::parser::parse(tokens)?;

        let file_name = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        for stmt in &program.statements {
            if let Some(item) = extract_doc_item(stmt, &file_name) {
                all_items.push(item);
            }
        }
    }

    // Generate index.html
    let index_html = render_index(&all_items);
    let index_path = out_dir.join("index.html");
    fs::write(&index_path, index_html)?;

    // Generate style.css
    let css_path = out_dir.join("style.css");
    fs::write(&css_path, STYLE_CSS)?;

    println!(
        "{} generated docs for {} item(s) in {}",
        "✓".green().bold(),
        all_items.len(),
        out_dir.display().to_string().cyan()
    );
    println!(
        "  {} {}",
        "open:".dimmed(),
        index_path.display().to_string().underline()
    );

    if open {
        open_in_browser(&index_path);
    }

    Ok(())
}

#[derive(Debug)]
struct DocItem {
    kind: ItemKind,
    name: String,
    module: String,
    doc: String,
    signature: String,
    is_pub: bool,
}

#[derive(Debug, PartialEq)]
enum ItemKind {
    Function,
    Struct,
    Enum,
    Trait,
}

impl ItemKind {
    fn as_str(&self) -> &'static str {
        match self {
            ItemKind::Function => "fn",
            ItemKind::Struct => "struct",
            ItemKind::Enum => "enum",
            ItemKind::Trait => "trait",
        }
    }

    fn color_class(&self) -> &'static str {
        match self {
            ItemKind::Function => "kind-fn",
            ItemKind::Struct => "kind-struct",
            ItemKind::Enum => "kind-enum",
            ItemKind::Trait => "kind-trait",
        }
    }
}

fn extract_doc_item(stmt: &Statement, module: &str) -> Option<DocItem> {
    match stmt {
        Statement::FunctionDef {
            name,
            params,
            return_type,
            effects,
            doc_comment,
            is_pub,
            ..
        } => {
            let doc = doc_comment.clone().unwrap_or_default();
            if doc.is_empty() && !*is_pub {
                return None;
            }

            let params_str = format_params(params);
            let ret = return_type
                .as_ref()
                .map(|t| format!(" -> {}", format_type(t)))
                .unwrap_or_default();
            let eff = if effects.is_empty() {
                String::new()
            } else {
                format!(" [{}]", effects.join(", "))
            };

            Some(DocItem {
                kind: ItemKind::Function,
                name: name.clone(),
                module: module.to_string(),
                doc,
                signature: format!("fn {name}({params_str}){ret}{eff}"),
                is_pub: *is_pub,
            })
        }

        Statement::StructDef {
            name,
            fields,
            doc_comment,
            is_pub,
            ..
        } => {
            let doc = doc_comment.clone().unwrap_or_default();
            if doc.is_empty() && !*is_pub {
                return None;
            }

            let mut sig = format!("struct {name}:\n");
            for field in fields {
                sig.push_str(&format!(
                    "    {}: {}\n",
                    field.name,
                    format_type(&field.type_annotation)
                ));
            }

            Some(DocItem {
                kind: ItemKind::Struct,
                name: name.clone(),
                module: module.to_string(),
                doc,
                signature: sig.trim_end().to_string(),
                is_pub: *is_pub,
            })
        }

        Statement::EnumDef {
            name,
            doc_comment,
            is_pub,
            ..
        } => {
            let doc = doc_comment.clone().unwrap_or_default();
            if doc.is_empty() && !*is_pub {
                return None;
            }

            Some(DocItem {
                kind: ItemKind::Enum,
                name: name.clone(),
                module: module.to_string(),
                doc,
                signature: format!("enum {name}"),
                is_pub: *is_pub,
            })
        }

        Statement::TraitDef {
            name,
            doc_comment,
            is_pub,
            ..
        } => {
            let doc = doc_comment.clone().unwrap_or_default();
            if doc.is_empty() && !*is_pub {
                return None;
            }

            Some(DocItem {
                kind: ItemKind::Trait,
                name: name.clone(),
                module: module.to_string(),
                doc,
                signature: format!("trait {name}"),
                is_pub: *is_pub,
            })
        }

        _ => None,
    }
}

fn format_params(params: &[Parameter]) -> String {
    params
        .iter()
        .map(|p| format!("{}: {}", p.name, format_type(&p.type_annotation)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::Generic(n, args) => {
            let args_str: Vec<String> = args.iter().map(format_type).collect();
            format!("{n}[{}]", args_str.join(", "))
        }
        TypeExpr::Function(params, ret) => {
            let p: Vec<String> = params.iter().map(format_type).collect();
            format!("({}) -> {}", p.join(", "), format_type(ret))
        }
        TypeExpr::Optional(inner) => format!("{}?", format_type(inner)),
        TypeExpr::Result(ok, err) => format!("{} or {}", format_type(ok), format_type(err)),
        TypeExpr::Tuple(elems) => {
            let e: Vec<String> = elems.iter().map(format_type).collect();
            format!("({})", e.join(", "))
        }
    }
}

fn collect_nova_files(path: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        collect_recursive(path, &mut files)?;
    }
    Ok(files)
}

fn collect_recursive(
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        // Skip hidden, target, and generated dirs
        if let Some(name) = p.file_name().and_then(|n| n.to_str())
            && (name.starts_with('.') || name == "target" || name == "generated")
        {
            continue;
        }
        if p.is_dir() {
            collect_recursive(&p, files)?;
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str())
            && (ext == "nova" || ext == "nv")
        {
            files.push(p);
        }
    }
    Ok(())
}

// ── HTML Rendering ──────────────────────────────────────────

fn render_index(items: &[DocItem]) -> String {
    // Group by module
    let mut modules: std::collections::BTreeMap<String, Vec<&DocItem>> =
        std::collections::BTreeMap::new();
    for item in items {
        modules.entry(item.module.clone()).or_default().push(item);
    }

    let mut body = String::new();

    // Sidebar
    body.push_str(r#"<aside class="sidebar"><h2>Modules</h2><ul>"#);
    for module in modules.keys() {
        body.push_str(&format!(
            "<li><a href=\"#module-{module}\">{module}</a></li>"
        ));
    }
    body.push_str("</ul></aside>");

    // Main content
    body.push_str(r#"<main class="content">"#);
    body.push_str(&format!(
        r#"<h1>Nova Documentation</h1><p class="meta">{} item(s) across {} module(s)</p>"#,
        items.len(),
        modules.len()
    ));

    for (module, module_items) in &modules {
        body.push_str(&format!(
            r#"<section class="module" id="module-{module}"><h2>module <span class="module-name">{module}</span></h2>"#
        ));

        for item in module_items {
            let kind_label = item.kind.as_str();
            let kind_class = item.kind.color_class();
            let visibility = if item.is_pub {
                r#"<span class="vis pub">pub</span> "#
            } else {
                ""
            };

            let doc_html = render_doc_body(&item.doc);

            body.push_str(&format!(
                r##"<article class="item {kind_class}" id="{module}-{name}">
                    <header>
                        <span class="kind">{visibility}{kind_label}</span>
                        <h3>{name}</h3>
                    </header>
                    <pre class="signature"><code>{signature}</code></pre>
                    <div class="doc-body">{doc_html}</div>
                </article>"##,
                name = item.name,
                signature = html_escape(&item.signature),
            ));
        }

        body.push_str("</section>");
    }

    body.push_str("</main>");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Nova Documentation</title>
<link rel="stylesheet" href="style.css">
</head>
<body>
<header class="site-header">
    <span class="logo">✦ Nova</span>
    <span class="tagline">Documentation</span>
</header>
<div class="layout">{body}</div>
</body>
</html>"##
    )
}

fn render_doc_body(doc: &str) -> String {
    if doc.is_empty() {
        return r#"<p class="no-doc">No documentation.</p>"#.to_string();
    }

    let mut html = String::new();
    let mut in_code = false;
    let mut code_buffer = String::new();
    let mut para_buffer = String::new();

    for line in doc.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code {
                html.push_str(&format!(
                    "<pre class=\"code\"><code>{}</code></pre>",
                    html_escape(&code_buffer)
                ));
                code_buffer.clear();
                in_code = false;
            } else {
                if !para_buffer.is_empty() {
                    html.push_str(&format!("<p>{}</p>", html_escape(para_buffer.trim())));
                    para_buffer.clear();
                }
                in_code = true;
            }
            continue;
        }

        if in_code {
            code_buffer.push_str(line);
            code_buffer.push('\n');
            continue;
        }

        if trimmed.is_empty() {
            if !para_buffer.is_empty() {
                html.push_str(&format!("<p>{}</p>", html_escape(para_buffer.trim())));
                para_buffer.clear();
            }
        } else {
            if !para_buffer.is_empty() {
                para_buffer.push(' ');
            }
            para_buffer.push_str(trimmed);
        }
    }

    if !para_buffer.is_empty() {
        html.push_str(&format!("<p>{}</p>", html_escape(para_buffer.trim())));
    }
    if in_code && !code_buffer.is_empty() {
        html.push_str(&format!(
            "<pre class=\"code\"><code>{}</code></pre>",
            html_escape(&code_buffer)
        ));
    }

    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn open_in_browser(path: &Path) {
    let url = path.to_string_lossy().to_string();

    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&url).spawn();

    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", &url])
        .spawn();
}

const STYLE_CSS: &str = r#"
:root {
    --bg: #0e1116;
    --bg-alt: #151a21;
    --fg: #e6edf3;
    --fg-dim: #8b949e;
    --accent: #7aa2f7;
    --accent-2: #bb9af7;
    --green: #9ece6a;
    --orange: #ff9e64;
    --red: #f7768e;
    --border: #30363d;
    --code-bg: #1a1f27;
}

* { box-sizing: border-box; }
body {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: var(--bg);
    color: var(--fg);
    line-height: 1.6;
}

.site-header {
    padding: 16px 32px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-alt);
    display: flex;
    align-items: baseline;
    gap: 16px;
}
.logo {
    font-size: 20px;
    font-weight: 700;
    color: var(--accent);
}
.tagline {
    color: var(--fg-dim);
    font-size: 14px;
}

.layout {
    display: grid;
    grid-template-columns: 240px 1fr;
    min-height: calc(100vh - 57px);
}

.sidebar {
    background: var(--bg-alt);
    border-right: 1px solid var(--border);
    padding: 24px;
    position: sticky;
    top: 0;
    max-height: calc(100vh - 57px);
    overflow-y: auto;
}
.sidebar h2 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-dim);
    margin: 0 0 12px 0;
}
.sidebar ul {
    list-style: none;
    padding: 0;
    margin: 0;
}
.sidebar a {
    display: block;
    padding: 6px 10px;
    color: var(--fg);
    text-decoration: none;
    border-radius: 4px;
    font-size: 14px;
}
.sidebar a:hover {
    background: var(--code-bg);
    color: var(--accent);
}

.content {
    padding: 32px 48px;
    max-width: 960px;
}
h1 {
    margin-top: 0;
    font-size: 32px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 16px;
}
.meta {
    color: var(--fg-dim);
    font-size: 14px;
}

.module {
    margin-top: 48px;
}
.module h2 {
    font-size: 20px;
    color: var(--fg-dim);
    font-weight: 400;
}
.module-name {
    color: var(--accent-2);
    font-weight: 600;
}

.item {
    margin-top: 32px;
    padding: 20px;
    background: var(--bg-alt);
    border: 1px solid var(--border);
    border-radius: 8px;
}
.item header {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 12px;
}
.item h3 {
    margin: 0;
    font-size: 18px;
    font-family: "SF Mono", Menlo, Consolas, monospace;
    color: var(--accent);
}
.kind {
    color: var(--fg-dim);
    font-family: monospace;
    font-size: 13px;
}
.vis.pub { color: var(--green); font-weight: 600; }

.kind-fn h3 { color: var(--accent); }
.kind-struct h3 { color: var(--orange); }
.kind-enum h3 { color: var(--accent-2); }
.kind-trait h3 { color: var(--red); }

.signature {
    background: var(--code-bg);
    padding: 12px 16px;
    border-radius: 6px;
    margin: 0 0 16px 0;
    overflow-x: auto;
    font-size: 13px;
}
.signature code {
    font-family: "SF Mono", Menlo, Consolas, monospace;
    color: var(--fg);
}

.doc-body p {
    margin: 8px 0;
}
.doc-body .code {
    background: var(--code-bg);
    padding: 12px 16px;
    border-radius: 6px;
    overflow-x: auto;
    font-size: 13px;
}
.no-doc {
    color: var(--fg-dim);
    font-style: italic;
}
"#;
