//! Purpose: XML/HTML markup highlighting tests covering `pom.xml` constructs.

use super::*;

#[test]
fn xml_tags_with_dots_hyphens_and_digits_stay_one_tag_run() {
    let out = runs(
        "pom.xml",
        "        <jackson-2-bom.version>2.13.4.20221013</jackson-2-bom.version>",
    );
    assert_eq!(token_of(&out, "<"), Token::Punctuation);
    assert_eq!(token_of(&out, "jackson-2-bom.version"), Token::Tag);
    assert_eq!(token_of(&out, ">"), Token::Punctuation);
    assert_eq!(token_of(&out, "2.13.4.20221013"), Token::Normal);
    assert_eq!(token_of(&out, "</"), Token::Punctuation);

    let self_closing = runs("pom.xml", "        <relativePath/>");
    assert_eq!(token_of(&self_closing, "relativePath"), Token::Tag);
    assert_eq!(token_of(&self_closing, "/>"), Token::Punctuation);
}

#[test]
fn multiline_opening_tag_highlights_namespaces_and_urls() {
    let mut hl = Highlighter::new(Path::new("pom.xml"));
    let line1 = hl.line(r#"<project xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#);
    for expected in [
        ("<", Token::Punctuation),
        ("project", Token::Tag),
        ("xmlns", Token::Punctuation),
        (":", Token::Str),
        ("xsi", Token::Type),
        ("=", Token::Tag),
        ("http://", Token::Punctuation),
        ("www.w3.org/2001/XMLSchema-instance", Token::Url),
    ] {
        assert!(line1.contains(&expected), "{line1:?}");
    }

    let line2 = hl.line(r#"         xmlns="http://maven.apache.org/POM/4.0.0""#);
    assert!(line2.contains(&("xmlns", Token::Punctuation)), "{line2:?}");
    assert!(line2.contains(&("maven.apache.org/POM/4.0.0", Token::Url)));

    let line3 = hl.line(
        r#"         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/maven-v4_0_0.xsd">"#,
    );
    assert!(line3.contains(&("xsi", Token::Punctuation)), "{line3:?}");
    assert!(line3.contains(&("schemaLocation", Token::Type)));
    assert!(line3.contains(&("maven.apache.org/maven-v4_0_0.xsd", Token::Url)));
    assert!(line3.contains(&(">", Token::Punctuation)), "{line3:?}");
}

#[test]
fn comments_and_element_urls_are_classified() {
    let comment = runs(
        "pom.xml",
        "    <!-- https://mvnrepository.com/artifact/org.springframework.boot/spring-boot-starter-parent -->",
    );
    assert_eq!(token_of(&comment, "<!-- "), Token::Comment);
    assert_eq!(
        token_of(
            &comment,
            "https://mvnrepository.com/artifact/org.springframework.boot/spring-boot-starter-parent"
        ),
        Token::Url
    );
    assert_eq!(token_of(&comment, " -->"), Token::Comment);

    let url_line = runs("pom.xml", "    <url>http://docs.example.org</url>");
    assert_eq!(token_of(&url_line, "url"), Token::Tag);
    assert_eq!(token_of(&url_line, "http://docs.example.org"), Token::Uri);
}

#[test]
fn property_interpolations_and_arguments_in_element_text_stay_normal_cyan() {
    let arg_line = runs(
        "pom.xml",
        "        <argument>${messagetracker.original.jar}</argument>",
    );
    assert_eq!(token_of(&arg_line, "argument"), Token::Tag);
    assert_eq!(
        token_of(&arg_line, "${messagetracker.original.jar}"),
        Token::Normal
    );

    let flag_line = runs("pom.xml", "        <argument>-profile=EE</argument>");
    assert_eq!(token_of(&flag_line, "-profile=EE"), Token::Normal);
}

#[test]
fn multiline_comments_and_adjacent_comment_urls_preserve_state() {
    let mut hl = Highlighter::new(Path::new("pom.xml"));
    let first = hl.line("<!-- multi-line start");
    assert_eq!(first, vec![("<!-- multi-line start", Token::Comment)]);

    let middle = hl.line("still inside https://example.org/docs--><b>ok</b>");
    assert_eq!(
        middle,
        vec![
            ("still inside ", Token::Comment),
            ("https://example.org/docs", Token::Url),
            ("-->", Token::Comment),
            ("<", Token::Punctuation),
            ("b", Token::Tag),
            (">", Token::Punctuation),
            ("ok", Token::Normal),
            ("</", Token::Punctuation),
            ("b", Token::Tag),
            (">", Token::Punctuation),
        ]
    );
}

#[test]
fn processing_instructions_doctypes_and_bare_angles_are_tokenized() {
    let pi = runs("a.xml", "<?xml version='1.0' encoding=\"UTF-8\"?>");
    assert_eq!(token_of(&pi, "<?"), Token::Punctuation);
    assert_eq!(token_of(&pi, "xml"), Token::Tag);
    assert_eq!(token_of(&pi, "version"), Token::Type);
    assert_eq!(token_of(&pi, "'1.0'"), Token::Str);
    assert_eq!(token_of(&pi, "\"UTF-8\""), Token::Str);
    assert_eq!(token_of(&pi, "?>"), Token::Punctuation);

    let doctype = runs("index.html", "<!DOCTYPE html>");
    assert_eq!(token_of(&doctype, "<!"), Token::Punctuation);
    assert_eq!(token_of(&doctype, "DOCTYPE"), Token::Tag);
    assert_eq!(token_of(&doctype, "html"), Token::Type);

    let bare_lt = runs("a.xml", "a < b");
    assert_eq!(token_of(&bare_lt, "<"), Token::Punctuation);
}

#[test]
fn attribute_edge_cases_cover_unterminated_quotes_mixed_urls_and_punctuation() {
    let mixed = runs(
        "a.xml",
        r#"<tag a:b="prefix https://ex.org suffix" ! raw="unterminated"#,
    );
    assert_eq!(token_of(&mixed, "a"), Token::Punctuation);
    assert_eq!(token_of(&mixed, ":"), Token::Punctuation);
    assert_eq!(token_of(&mixed, "b"), Token::Type);
    assert_eq!(token_of(&mixed, "prefix"), Token::Str);
    assert_eq!(token_of(&mixed, "https://"), Token::Punctuation);
    assert_eq!(token_of(&mixed, "ex.org"), Token::Url);
    assert_eq!(token_of(&mixed, "suffix"), Token::Str);
    assert_eq!(token_of(&mixed, "!"), Token::Punctuation);
    assert_eq!(token_of(&mixed, "\"unterminated"), Token::Str);

    let text_url = runs("a.xml", "<p>See https://example.com now</p>");
    assert_eq!(token_of(&text_url, "See "), Token::Normal);
    assert_eq!(token_of(&text_url, "https://example.com"), Token::Uri);
    assert_eq!(token_of(&text_url, " now"), Token::Normal);
}

#[test]
fn all_markup_extensions_route_through_markup_mode() {
    for ext in [
        "xml", "html", "htm", "xhtml", "svg", "xsd", "xsl", "xslt", "wsdl", "pom", "csproj",
        "props", "targets", "plist", "jmx",
    ] {
        let file = format!("sample.{ext}");
        let out = runs(&file, "<node-1>2.0</node-1>");
        assert_eq!(token_of(&out, "node-1"), Token::Tag, "{file}");
        assert_eq!(token_of(&out, "2.0"), Token::Normal, "{file}");
    }
}
