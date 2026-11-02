//! Purpose: Validate and pretty-print XML documents with two-space indentation.

const MAX_XML_DEPTH: usize = 256;
const MAX_XML_NODES: u32 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Piece<'a> {
    Decl(&'a str),
    Comment(&'a str),
    Cdata(&'a str),
    Close(&'a str),
    SelfClose(&'a str),
    Open(&'a str),
    Text(&'a str),
}

pub(super) fn format_xml(source: &str) -> Result<String, String> {
    let pieces = tokenize(source);
    if exceeds_max_depth(&pieces, MAX_XML_DEPTH) {
        return Err(format!(
            "Invalid XML: nesting depth exceeds {MAX_XML_DEPTH}"
        ));
    }
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        nodes_limit: MAX_XML_NODES,
        entity_resolver: None,
    };
    roxmltree::Document::parse_with_options(source, options)
        .map_err(|err| format!("Invalid XML: {err}"))?;
    Ok(render_pieces(&pieces))
}

fn exceeds_max_depth(pieces: &[Piece<'_>], limit: usize) -> bool {
    let mut depth = 0_usize;
    for piece in pieces {
        match piece {
            Piece::Open(_) | Piece::SelfClose(_) if depth >= limit => return true,
            Piece::Open(_) => depth += 1,
            Piece::Close(_) => depth = depth.saturating_sub(1),
            Piece::SelfClose(_)
            | Piece::Decl(_)
            | Piece::Comment(_)
            | Piece::Cdata(_)
            | Piece::Text(_) => {}
        }
    }
    false
}

fn tokenize(source: &str) -> Vec<Piece<'_>> {
    let mut pieces = Vec::new();
    let mut rest = source.trim();
    while !rest.is_empty() {
        if rest.starts_with('<') {
            let end = markup_end(rest);
            let raw = rest.get(..end).unwrap_or(rest).trim();
            pieces.push(classify_markup(raw));
            rest = rest.get(end..).unwrap_or("");
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            let text = rest.get(..end).unwrap_or(rest);
            if !text.trim().is_empty() {
                pieces.push(Piece::Text(text.trim()));
            }
            rest = rest.get(end..).unwrap_or("");
        }
    }
    pieces
}

fn markup_end(rest: &str) -> usize {
    if rest.starts_with("<!--") {
        return rest.find("-->").map_or(rest.len(), |pos| pos + 3);
    }
    if rest.starts_with("<![CDATA[") {
        return rest.find("]]>").map_or(rest.len(), |pos| pos + 3);
    }
    if rest.starts_with("<?") {
        return rest.find("?>").map_or(rest.len(), |pos| pos + 2);
    }
    tag_end(rest)
}

fn tag_end(rest: &str) -> usize {
    let mut quote: Option<char> = None;
    let mut brackets = 0_usize;
    for (idx, ch) in rest.char_indices() {
        match quote {
            Some(q) if ch == q => quote = None,
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '[' => brackets += 1,
            None if ch == ']' => brackets = brackets.saturating_sub(1),
            None if ch == '>' && brackets == 0 => return idx + 1,
            Some(_) | None => {}
        }
    }
    rest.len()
}

fn classify_markup(raw: &str) -> Piece<'_> {
    if raw.starts_with("<!--") {
        Piece::Comment(raw)
    } else if raw.starts_with("<![CDATA[") {
        Piece::Cdata(raw)
    } else if raw.starts_with("<?") || raw.starts_with("<!") {
        Piece::Decl(raw)
    } else if raw.starts_with("</") {
        Piece::Close(raw)
    } else if raw.ends_with("/>") {
        Piece::SelfClose(raw)
    } else {
        Piece::Open(raw)
    }
}

fn render_pieces(pieces: &[Piece<'_>]) -> String {
    let mut lines = Vec::new();
    let mut depth = 0_usize;
    let mut index = 0_usize;
    while let Some(&piece) = pieces.get(index) {
        if let Piece::Open(open) = piece {
            if let Some((line, consumed)) = try_inline_element(open, pieces, index, depth) {
                lines.push(line);
                index += consumed;
                continue;
            }
            lines.push(indented(depth, open));
            depth += 1;
        } else if let Piece::Close(close) = piece {
            depth = depth.saturating_sub(1);
            lines.push(indented(depth, close));
        } else {
            lines.push(indented(depth, piece_text(piece)));
        }
        index += 1;
    }
    lines.join("\n")
}

fn try_inline_element(
    open: &str,
    pieces: &[Piece<'_>],
    index: usize,
    depth: usize,
) -> Option<(String, usize)> {
    match (pieces.get(index + 1), pieces.get(index + 2)) {
        (Some(Piece::Close(close)), _) => Some((indented(depth, &format!("{open}{close}")), 2)),
        (Some(Piece::Text(body) | Piece::Cdata(body)), Some(Piece::Close(close))) => {
            Some((indented(depth, &format!("{open}{body}{close}")), 3))
        }
        _ => None,
    }
}

const fn piece_text(piece: Piece<'_>) -> &str {
    match piece {
        Piece::Decl(s)
        | Piece::Comment(s)
        | Piece::Cdata(s)
        | Piece::Close(s)
        | Piece::SelfClose(s)
        | Piece::Open(s)
        | Piece::Text(s) => s,
    }
}

fn indented(depth: usize, text: &str) -> String {
    format!("{}{text}", "  ".repeat(depth))
}

#[cfg(test)]
mod tests_tags;
