use leaners_render::markdown_to_html;

#[test]
fn basic_constructs() {
    assert_eq!(markdown_to_html("hello"), "<p>hello</p>\n");
    assert_eq!(
        markdown_to_html("*a* **b**"),
        "<p><em>a</em> <strong>b</strong></p>\n"
    );
    assert_eq!(
        markdown_to_html("`x < y`"),
        "<p><code>x &lt; y</code></p>\n"
    );
    assert_eq!(markdown_to_html("---"), "<hr>\n");
    assert!(markdown_to_html("- a\n- b").starts_with("<ul>\n<li>"));
    assert!(markdown_to_html("1. a").starts_with("<ol>\n<li>"));
    assert!(markdown_to_html("> quoted").contains("<blockquote>"));
}

#[test]
fn code_blocks_keep_language_and_escape() {
    let html = markdown_to_html("```lean\ntheorem t : 1 < 2 := by decide\n```");
    assert!(html.contains("class=\"language-lean\""), "{html}");
    // Highlighting puts spans between the tokens, but `<` from the input is
    // still an entity and never markup.
    assert!(html.contains("&lt;"), "{html}");
    assert!(!html.contains("1 < 2"), "{html}");
}

#[test]
fn highlighting_labels_the_three_known_languages() {
    let rust = markdown_to_html("```rust\nfn f() { let s = \"hi\"; } // done\n```");
    assert!(rust.contains("<span class=\"tok-kw\">fn</span>"), "{rust}");
    assert!(rust.contains("<span class=\"tok-kw\">let</span>"), "{rust}");
    assert!(
        rust.contains("<span class=\"tok-str\">&quot;hi&quot;</span>"),
        "{rust}"
    );
    assert!(
        rust.contains("<span class=\"tok-com\">// done</span>"),
        "{rust}"
    );

    let bash = markdown_to_html("```bash\n# note\nfor f in $DIR; do echo 1; done\n```");
    assert!(
        bash.contains("<span class=\"tok-com\"># note</span>"),
        "{bash}"
    );
    assert!(bash.contains("<span class=\"tok-kw\">for</span>"), "{bash}");
    assert!(
        bash.contains("<span class=\"tok-var\">$DIR</span>"),
        "{bash}"
    );

    let lean = markdown_to_html("```lean\n-- c\ntheorem t : True := by trivial\n```");
    assert!(
        lean.contains("<span class=\"tok-com\">-- c</span>"),
        "{lean}"
    );
    assert!(
        lean.contains("<span class=\"tok-kw\">theorem</span>"),
        "{lean}"
    );
}

#[test]
fn an_unknown_language_is_left_exactly_as_before() {
    let html = markdown_to_html("```python\ndef f(): return 1 < 2\n```");
    assert_eq!(
        html,
        "<pre><code class=\"language-python\">def f(): return 1 &lt; 2\n</code></pre>\n"
    );
    let plain = markdown_to_html("```\nfn x < y\n```");
    assert_eq!(plain, "<pre><code>fn x &lt; y\n</code></pre>\n");
}

// The property that makes highlighting safe to do inside the renderer: the only
// markup it can emit is the closed set of span tags. Strip those and no `<`
// may remain, however hostile the code block is.
#[test]
fn highlighting_emits_no_tag_outside_the_closed_set() {
    let hostile = [
        "```rust\n</span><script>alert(1)</script>\n```",
        "```rust\nlet s = \"</span><img onerror=x>\";\n```",
        "```bash\necho '</span>' # <div>\n```",
        "```bash\n$( </span> ) \"<script>\"\n```",
        "```lean\n-- </span><iframe>\ntheorem t : a < b := by\n```",
        "```lean\n/- <b> -/ def f := \"</span>\"\n```",
        "```rust\nr#\"</span>\"# 'a' '\\n' 0xFF_u8\n```",
    ];
    for src in hostile {
        let html = markdown_to_html(src);
        let mut stripped = html.clone();
        for tag in [
            "<span class=\"tok-kw\">",
            "<span class=\"tok-str\">",
            "<span class=\"tok-com\">",
            "<span class=\"tok-num\">",
            "<span class=\"tok-var\">",
            "</span>",
            "<pre>",
            "</pre>",
            "<code>",
            "</code>",
        ] {
            stripped = stripped.replace(tag, "");
        }
        // The opening <code class="language-..."> is render's, not highlight's.
        if let Some(i) = stripped.find("<code class=\"") {
            let rest = &stripped[i..];
            let j = rest.find('>').expect("unterminated tag") + 1;
            stripped = format!("{}{}", &stripped[..i], &rest[j..]);
        }
        assert!(
            !stripped.contains('<'),
            "tag outside the closed set: {src} -> {html} (left: {stripped})"
        );
        // Spans must balance.
        assert_eq!(
            html.matches("<span").count(),
            html.matches("</span>").count(),
            "unbalanced spans: {src} -> {html}"
        );
    }
}

// The point of the whole split: whatever the parser does, markup from the
// input must never reach the output as markup.
#[test]
fn raw_html_cannot_reach_the_output() {
    for src in [
        "<script>alert(1)</script>",
        "<img src=x onerror=alert(1)>",
        "text <b>bold</b> text",
        "<div onclick='x'>y</div>",
        "a <iframe src=evil></iframe> b",
    ] {
        let html = markdown_to_html(src);
        assert!(!html.contains("<script"), "leaked script: {src} -> {html}");
        assert!(!html.contains("onerror"), "leaked handler: {src} -> {html}");
        assert!(!html.contains("onclick"), "leaked handler: {src} -> {html}");
        assert!(!html.contains("<iframe"), "leaked iframe: {src} -> {html}");
        assert!(!html.contains("<b>"), "leaked tag: {src} -> {html}");
        assert!(!html.contains("<div"), "leaked tag: {src} -> {html}");
    }
}

#[test]
fn math_is_escaped_and_cannot_become_markup() {
    let inline = markdown_to_html("value $a < b$ here");
    assert!(
        inline.contains("<span class=\"math\">a &lt; b</span>"),
        "{inline}"
    );

    let display = markdown_to_html("$$x </span><script>alert(1)</script> y$$");
    assert!(display.contains("class=\"math math-display\""), "{display}");
    assert!(!display.contains("<script"), "{display}");
    assert!(display.contains("&lt;/span&gt;"), "{display}");
}

#[test]
fn tables_render_with_structure_and_escaping() {
    let html = markdown_to_html("| A | B |\n|---|---|\n| 1 < 2 | *b* |\n");
    assert!(html.contains("<table>"), "{html}");
    assert!(
        html.contains("<thead>\n<tr><th>A</th><th>B</th></tr>"),
        "{html}"
    );
    assert!(
        html.contains("<tbody>\n<tr><td>1 &lt; 2</td><td><em>b</em></td></tr>"),
        "{html}"
    );
    assert!(html.contains("</table>"), "{html}");
    // A header-only table must not open a tbody it never closes.
    let head_only = markdown_to_html("| A |\n|---|\n");
    assert_eq!(
        head_only.matches("<tbody>").count(),
        head_only.matches("</tbody>").count(),
        "{head_only}"
    );
}

#[test]
fn dangerous_url_schemes_are_dropped() {
    for (src, bad) in [
        ("[c](javascript:alert(1))", "javascript"),
        ("[c](JaVaScRiPt:alert(1))", "avaScript"),
        ("[c](data:text/html;base64,PHNjcmlwdD4=)", "data:"),
        ("![i](javascript:alert(1))", "javascript"),
    ] {
        let html = markdown_to_html(src);
        assert!(!html.contains(bad), "leaked scheme: {src} -> {html}");
    }
}

#[test]
fn safe_urls_survive() {
    assert!(markdown_to_html("[a](https://example.com)").contains("href=\"https://example.com\""));
    assert!(markdown_to_html("[a](#/notes/x)").contains("href=\"#/notes/x\""));
    assert!(markdown_to_html("[a](notes/x.md)").contains("href=\"notes/x.md\""));
    assert!(markdown_to_html("[a](mailto:x@y.z)").contains("href=\"mailto:x@y.z\""));
}

// Ladder step 3: injectivity of slugify is false, so the renderer
// disambiguates. These ids must be distinct.
#[test]
fn duplicate_headings_get_distinct_ids() {
    let html = markdown_to_html("# A B\n\n# a-b\n\n# A  B\n");
    let ids: Vec<&str> = html
        .match_indices("id=\"")
        .map(|(i, _)| {
            let rest = &html[i + 4..];
            &rest[..rest.find('"').unwrap()]
        })
        .collect();
    assert_eq!(ids.len(), 3, "{html}");
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 3, "duplicate anchor ids: {ids:?}");
}

#[test]
fn heading_levels_are_clamped_and_anchored() {
    assert!(markdown_to_html("# Hi").starts_with("<h1 id=\"hi\">"));
    assert!(markdown_to_html("###### Hi").starts_with("<h6 id=\"hi\">"));
}

#[test]
fn utf8_survives() {
    let html = markdown_to_html("# Título\n\n∀ x, P x → Q x, λ y\n");
    assert!(html.contains("Título"), "{html}");
    assert!(html.contains("∀ x, P x → Q x, λ y"), "{html}");
}
