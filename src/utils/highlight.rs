use typst_syntax::{parse, SyntaxKind, SyntaxNode};

/// Convert Typst code into HTML with syntax highlighting.
pub fn highlight_typst(source: &str) -> String {
    let root = parse(source);
    let mut html = String::new();

    html.push_str("<pre class=\"typst-highlighted\"><code>");
    highlight_node(&root, &mut html);
    html.push_str("</code></pre>");

    html
}

/// Recursively highlight a node and its children.
fn highlight_node(node: &SyntaxNode, html: &mut String) {
    for child in node.children() {
        let kind = child.kind();
        let text = child.text();

        // Pick the CSS class based on the token kind
        let class = match kind {
            // Comments
            SyntaxKind::LineComment | SyntaxKind::BlockComment => "comment",

            // Keywords and markup
            SyntaxKind::Hash => "keyword",
            SyntaxKind::Star | SyntaxKind::Underscore => "markup",

            // Strings
            SyntaxKind::Str => "string",

            // Numbers
            SyntaxKind::Int | SyntaxKind::Float => "number",

            // Heading
            SyntaxKind::Heading => "heading",

            // Code block
            SyntaxKind::Raw => "code",

            // Math
            SyntaxKind::Math | SyntaxKind::MathIdent | SyntaxKind::Equation => "math",

            // Functions and identifiers
            SyntaxKind::FuncCall | SyntaxKind::Ident => "function",

            // Labels and references
            SyntaxKind::Label | SyntaxKind::Ref => "label",

            // Operators (Star is already covered by markup)
            SyntaxKind::Plus | SyntaxKind::Minus | SyntaxKind::Slash |
            SyntaxKind::Eq | SyntaxKind::EqEq => "operator",

            // Default: no class
            _ => "",
        };

        if child.children().next().is_some() {
            // Node with children: recurse
            if !class.is_empty() {
                html.push_str(&format!("<span class=\"{}\">", class));
            }
            highlight_node(child, html);
            if !class.is_empty() {
                html.push_str("</span>");
            }
        } else {
            // Leaf node: write the text
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
