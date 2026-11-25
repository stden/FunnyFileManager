//! Purpose: The tokenizer classifies every kind of XML piece and keeps its
//! text verbatim, which is what the pretty-printer writes back.

use super::*;

#[test]
fn every_piece_kind_is_tokenized_with_its_text_verbatim() {
    for (raw, piece) in [
        ("<?xml?>", Piece::Decl("<?xml?>")),
        ("<!--x-->", Piece::Comment("<!--x-->")),
        ("<![CDATA[x]]>", Piece::Cdata("<![CDATA[x]]>")),
        ("</x>", Piece::Close("</x>")),
        ("<x/>", Piece::SelfClose("<x/>")),
        ("<x>", Piece::Open("<x>")),
        ("x", Piece::Text("x")),
    ] {
        assert_eq!(tokenize(raw), [piece], "{raw}");
    }
}

#[test]
fn whitespace_only_gaps_between_tags_are_ignored() {
    assert_eq!(
        tokenize("<root>  <a/> \n </root>"),
        [
            Piece::Open("<root>"),
            Piece::SelfClose("<a/>"),
            Piece::Close("</root>"),
        ]
    );
}
