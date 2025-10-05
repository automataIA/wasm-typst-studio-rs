// Configurazione globale del documento
#set math.equation(numbering: "(1)")
#set page(numbering: "1")

= Hello Typst!

This is a *Typst* editor in pure Rust WASM.

== Text Formatting Examples

You can format text in many ways:
- *Bold text* with asterisks
- _Italic text_ with underscores
- `Inline code` with backticks
- ~Strikethrough~ with tildes
- #text(fill: red)[Colored text] with functions
- #text(size: 14pt)[Different sizes]
- #smallcaps[Small Capitals]
- #super[superscript] and #sub[subscript]

== Lists and Enumerations

Unordered list:
- First item
- Second item
  - Nested item
  - Another nested
- Third item

Ordered list:
+ Step one
+ Step two
  + Sub-step A
  + Sub-step B
+ Step three

Term list:
/ Rust: A systems programming language
/ WASM: WebAssembly for web applications
/ Leptos: Reactive UI framework in Rust

== Code Blocks

Here's a code block with syntax highlighting:

```rust
fn main() {
    let message = "Hello from Typst!";
    println!("{}", message);
}
```

And some Python:

```python
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)
```

= Mathematical Formulas

== Inline and Display Math

Inline math: $ E = m c^2 $ appears in text.

Display math is centered:

$ integral_0^oo e^(-x^2) dif x = sqrt(pi)/2 $

== Complex Examples

La sezione aurea è definita come:

$ phi.alt := (1 + sqrt(5)) / 2 $ <golden-ratio>

Usando @golden-ratio, possiamo calcolare i numeri di Fibonacci:

$ F_n = floor(1 / sqrt(5) phi.alt^n) $ <fibonacci>

Come mostrato in @fibonacci, la formula è elegante.

Matrice example:
$ mat(
  1, 2, 3;
  4, 5, 6;
  7, 8, 9
) $

Sistema di equazioni:
$ cases(
  x + y = 5,
  2x - y = 1
) $

== Advanced Math Symbols

Greek letters: $ alpha, beta, gamma, Delta, Omega $

Operators: $ sum_(i=1)^n i = (n(n+1))/2 $

Calculus: $ (dif f)/(dif x) = lim_(h->0) (f(x+h) - f(x))/h $

= Tables

Simple table:

#table(
  columns: 3,
  [*Name*], [*Age*], [*City*],
  [Alice], [25], [New York],
  [Bob], [30], [London],
  [Charlie], [35], [Tokyo]
)

Styled table:

#table(
  columns: (1fr, 2fr, 1fr),
  align: (center, left, right),
  fill: (x, y) => if y == 0 { gray } else if calc.odd(y) { silver },
  [*ID*], [*Description*], [*Value*],
  [1], [First item with long text], [100],
  [2], [Second item], [250],
  [3], [Third item], [500]
)

= Figures and Images

Nel testo: vedi @mia-figura per dettagli.

#figure(
  rect(width: 80%, height: 120pt, fill: rgb("#e0e0e0")),
  caption: [Placeholder per immagine],
) <mia-figura>

// Per caricare un'immagine vera:
// 1. Clicca "Image" nella toolbar
// 2. Seleziona un file
// 3. Usa l'ID generato (es. img_123456_789) nel codice

= Advanced Layout

== Columns

#columns(2)[
  This text is displayed in two columns. Lorem ipsum dolor sit amet, consectetur adipiscing elit.

  Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.

  #colbreak()

  This is the second column. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.
]

== Boxes and Blocks

#box(
  fill: luma(230),
  inset: 8pt,
  radius: 4pt,
  [This is a highlighted box with rounded corners]
)

#block(
  fill: rgb(255, 200, 200),
  inset: 10pt,
  radius: 4pt,
  [#text(weight: "bold")[Warning:] This is an important note!]
)

= Citations Example

Click "Bibliography" button to manage references.
You can cite like this: @example2024 or @typst2023

#bibliography("refs.yml")
