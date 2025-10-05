// IEEE-style paper template (simplified for WASM)
#let ieee(
  title: [],
  abstract: [],
  authors: (),
  index-terms: (),
  figure-supplement: [Fig.],
  body
) = {
  // Page setup
  set page(
    paper: "us-letter",
    margin: (x: 0.75in, top: 0.875in, bottom: 1in),
    header: context {
      if counter(page).get().first() > 1 {
        align(center, text(size: 9pt, smallcaps(title)))
      }
    },
    numbering: "1",
  )

  // Text setup
  set text(size: 10pt)
  set par(justify: true, leading: 0.58em)

  // Enable numbering for references
  set heading(numbering: "1.")
  set math.equation(numbering: "(1)")

  // Heading styles
  show heading.where(level: 1): it => {
    set text(size: 10pt, weight: "bold")
    set align(center)
    upper(it.body)
    v(0.5em)
  }

  show heading.where(level: 2): it => {
    set text(size: 10pt, weight: "bold", style: "italic")
    it
    v(0.3em)
  }

  // Figure captions
  show figure.caption: it => {
    set text(size: 9pt)
    [#figure-supplement #context counter(figure).display(): #it.body]
  }

  // Title
  align(center)[
    #set text(size: 24pt, weight: "bold")
    #block(width: 90%, title)
    #v(1em)
  ]

  // Authors
  align(center)[
    #set text(size: 11pt)
    #for author in authors [
      #author.name#if "department" in author [, #author.department]#if "organization" in author [ \
      #author.organization]#if "location" in author [, #author.location] \
      #if "email" in author [#text(size: 9pt, author.email)] \
      #v(0.5em)
    ]
  ]

  v(1em)

  // Abstract
  set par(first-line-indent: 0pt)
  block(width: 100%, inset: (x: 0.5in))[
    #text(weight: "bold", style: "italic", "Abstract")—#abstract

    #if index-terms.len() > 0 [
      #v(0.5em)
      #text(weight: "bold", style: "italic", "Index Terms")—#index-terms.join(", ")
    ]
  ]

  v(1.5em)

  // Two-column layout for body
  set par(first-line-indent: 1.5em)
  columns(2, gutter: 0.25in, body)
}

// Use the template
#show: ieee.with(
  title: [A Typesetting System to Untangle the Scientific Writing Process],
  abstract: [
    The process of scientific writing is often tangled up with the intricacies of typesetting, leading to frustration and wasted time for researchers. In this paper, we introduce Typst, a new typesetting system designed specifically for scientific writing. Typst untangles the typesetting process, allowing researchers to compose papers faster. In a series of experiments we demonstrate that Typst offers several advantages, including faster document creation, simplified syntax, and increased ease-of-use.
  ],
  authors: (
    (
      name: "Martin Haug",
      department: [Co-Founder],
      organization: [Typst GmbH],
      location: [Berlin, Germany],
      email: "haug@typst.app"
    ),
    (
      name: "Laurenz Mädje",
      department: [Co-Founder],
      organization: [Typst GmbH],
      location: [Berlin, Germany],
      email: "maedje@typst.app"
    ),
  ),
  index-terms: ("Scientific writing", "Typesetting", "Document creation", "Syntax"),
  figure-supplement: [Fig.],
)

= Introduction
Scientific writing is a crucial part of the research process, allowing researchers to share their findings with the wider scientific community. However, the process of typesetting scientific documents can often be a frustrating and time-consuming affair, particularly when using outdated tools such as LaTeX. Despite being over 30 years old, it remains a popular choice for scientific writing due to its power and flexibility. However, it also comes with a steep learning curve, complex syntax, and long compile times, leading to frustration and despair for many researchers @netwok2020 @netwok2022.

== Paper overview
In this paper we introduce Typst, a new typesetting system designed to streamline the scientific writing process and provide researchers with a fast, efficient, and easy-to-use alternative to existing systems. Our goal is to shake up the status quo and offer researchers a better way to approach scientific writing.

By leveraging advanced algorithms and a user-friendly interface, Typst offers several advantages over existing typesetting systems, including faster document creation, simplified syntax, and increased ease-of-use.

To demonstrate the potential of Typst, we conducted a series of experiments comparing it to other popular typesetting systems, including LaTeX. Our findings suggest that Typst offers several benefits for scientific writing, particularly for novice users who may struggle with the complexities of LaTeX. Additionally, we demonstrate that Typst offers advanced features for experienced users, allowing for greater customization and flexibility in document creation.

Overall, we believe that Typst represents a significant step forward in the field of scientific writing and typesetting, providing researchers with a valuable tool to streamline their workflow and focus on what really matters: their research. In the following sections, we will introduce Typst in more detail and provide evidence for its superiority over other typesetting systems in a variety of scenarios.

= Methods <sec:methods>
#lorem(45)

$ a + b = gamma $ <eq:gamma>

#lorem(80)

#figure(
  placement: none,
  circle(radius: 15pt),
  caption: [A circle representing the Sun.]
) <fig:sun>

In @fig:sun you can see a common representation of the Sun, which is a star that is located at the center of the solar system.

#lorem(120)

#figure(
  caption: [The Planets of the Solar System and Their Average Distance from the Sun],
  placement: top,
  table(
    columns: (6em, auto),
    align: (left, right),
    inset: (x: 8pt, y: 4pt),
    stroke: (x, y) => if y <= 1 { (top: 0.5pt) },
    fill: (x, y) => if y > 0 and calc.rem(y, 2) == 0  { rgb("#efefef") },

    table.header[Planet][Distance (million km)],
    [Mercury], [57.9],
    [Venus], [108.2],
    [Earth], [149.6],
    [Mars], [227.9],
    [Jupiter], [778.6],
    [Saturn], [1,433.5],
    [Uranus], [2,872.5],
    [Neptune], [4,495.1],
  )
) <tab:planets>

In @tab:planets, you see the planets of the solar system and their average distance from the Sun.
The distances were calculated with @eq:gamma that we presented in @sec:methods.

#lorem(240)

#lorem(240)

= Conclusion
This paper has introduced Typst as a modern alternative to traditional typesetting systems. Our experiments demonstrate that Typst offers significant advantages in terms of ease of use, compilation speed, and document quality.

Future work will focus on expanding Typst's capabilities and improving its performance in various scientific writing scenarios @example2024.

= References
#bibliography("refs.yml")
