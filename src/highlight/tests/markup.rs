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
