use typst_syntax::{parse, SyntaxKind, SyntaxNode};

/// Converte codice Typst in HTML con syntax highlighting
pub fn highlight_typst(source: &str) -> String {
    let root = parse(source);
    let mut html = String::new();

    html.push_str("<pre class=\"typst-highlighted\"><code>");
    highlight_node(&root, &mut html);
    html.push_str("</code></pre>");

    html
}

/// Ricorsivamente highlighta un nodo e i suoi figli
fn highlight_node(node: &SyntaxNode, html: &mut String) {
    for child in node.children() {
        let kind = child.kind();
        let text = child.text();

        // Determina la classe CSS basata sul tipo di token
        let class = match kind {
            // Commenti
            SyntaxKind::LineComment | SyntaxKind::BlockComment => "comment",

            // Keywords e markup
            SyntaxKind::Hash => "keyword",
            SyntaxKind::Star | SyntaxKind::Underscore => "markup",

            // Stringhe
            SyntaxKind::Str => "string",

            // Numeri
            SyntaxKind::Int | SyntaxKind::Float => "number",

            // Heading
            SyntaxKind::Heading => "heading",

            // Code block
            SyntaxKind::Raw => "code",

            // Math
            SyntaxKind::Math | SyntaxKind::MathIdent | SyntaxKind::Equation => "math",

            // Funzioni e identificatori
            SyntaxKind::FuncCall | SyntaxKind::Ident => "function",

            // Label e reference
            SyntaxKind::Label | SyntaxKind::Ref => "label",

            // Operatori (Star è già in markup)
            SyntaxKind::Plus | SyntaxKind::Minus | SyntaxKind::Slash |
            SyntaxKind::Eq | SyntaxKind::EqEq => "operator",

            // Default: nessuna classe
            _ => "",
        };

        if child.children().next().is_some() {
            // Nodo con figli: ricorsione
            if !class.is_empty() {
                html.push_str(&format!("<span class=\"{}\">", class));
            }
            highlight_node(child, html);
            if !class.is_empty() {
                html.push_str("</span>");
            }
        } else {
            // Nodo foglia: scrivi il testo
            let escaped = html_escape(text.as_str());
            if !class.is_empty() {
                html.push_str(&format!("<span class=\"{}\">{}</span>", class, escaped));
            } else {
                html.push_str(&escaped);
            }
        }
    }
}

/// Escape HTML entities
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
