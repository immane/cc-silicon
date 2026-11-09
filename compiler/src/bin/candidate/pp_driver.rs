// ============================================================================
// candidate/pp_driver.rs — host-only PP line/directive grouping (B3-03a)
//
// Mechanical host helper for the bounded candidate driver. It groups the
// committed PP token stream into raw physical lines (the same `(source,
// line)` key the PP05/PP19 chips use), classifies a leading `#`/`%:` line,
// and rejects directives whose tokens cross a physical line. It performs no
// macro or conditional evaluation; those decisions stay in the frozen chips.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, PpTokenKind};
use cc_silicon_compiler::ids::{PpTokenId, RecordRef, SourceId};

/// One raw physical line of committed PP tokens, in stream order.
pub struct PpLine {
    /// Owning source.
    pub source: SourceId,
    /// Zero-based raw line index (count of `\n` before the line start).
    pub line: usize,
    /// Token IDs on this line, in stream order.
    pub tokens: Vec<PpTokenId>,
}

impl PpLine {
    /// All token refs on the line, in order.
    pub fn refs(&self) -> Vec<RecordRef> {
        self.tokens
            .iter()
            .map(|id| RecordRef::PpToken(*id))
            .collect()
    }

    /// Token refs after the leading directive introducer (`#`/`%:`).
    pub fn directive_body_refs(&self) -> Vec<RecordRef> {
        self.tokens
            .iter()
            .skip(1)
            .map(|id| RecordRef::PpToken(*id))
            .collect()
    }
}

/// Grouped stream: physical lines plus exactly one EOF ref when present.
pub struct PpStream {
    /// Physical lines in stream order (EOF excluded).
    pub lines: Vec<PpLine>,
    /// The single EOF token, excluded from `lines`.
    pub eof: Option<PpTokenId>,
}

/// Group committed PP-token refs into raw physical lines.
///
/// Fails on a non-PP-token ref, a dangling token/span/source, or more than
/// one EOF. EOF is not a line member; the caller appends it once at the end
/// of the final stream.
pub fn group_lines(bus: &CompilerBus, refs: &[RecordRef]) -> Result<PpStream, String> {
    let mut lines: Vec<PpLine> = Vec::new();
    let mut eof: Option<PpTokenId> = None;
    for reference in refs {
        let id = match reference {
            RecordRef::PpToken(id) => *id,
            other => {
                return Err(format!(
                    "PP driver expects pp-token refs, found {}",
                    other.label()
                ));
            }
        };
        let body = bus
            .arenas
            .pp_tokens
            .get(id)
            .map_err(|_| format!("PP driver reads missing pp-token {}", id.index()))?;
        if body.kind == PpTokenKind::Eof {
            if eof.is_some() {
                return Err("PP driver reads more than one EOF token".to_string());
            }
            eof = Some(id);
            continue;
        }
        let span = bus
            .arenas
            .spans
            .get(body.span)
            .map_err(|_| format!("PP driver reads missing span {}", body.span.index()))?;
        let bytes = bus
            .arenas
            .sources
            .get(span.source)
            .map_err(|_| format!("PP driver reads missing source {}", span.source.index()))?;
        let line = line_index(&bytes.bytes, span.start);
        let extends = lines
            .last()
            .is_some_and(|open| open.source == span.source && open.line == line);
        if extends {
            if let Some(open) = lines.last_mut() {
                open.tokens.push(id);
            }
        } else {
            lines.push(PpLine {
                source: span.source,
                line,
                tokens: vec![id],
            });
        }
    }
    Ok(PpStream { lines, eof })
}

/// True when the line is a directive: leading `#`/`%:` passing the frozen
/// raw walk-back predicate.
pub fn line_is_directive(bus: &CompilerBus, line: &PpLine) -> bool {
    let Some(first) = line.tokens.first() else {
        return false;
    };
    let Ok(body) = bus.arenas.pp_tokens.get(*first) else {
        return false;
    };
    if body.kind != PpTokenKind::Punctuator
        || (body.spelling.as_slice() != b"#" && body.spelling.as_slice() != b"%:")
    {
        return false;
    }
    let Ok(span) = bus.arenas.spans.get(body.span) else {
        return false;
    };
    let Ok(source) = bus.arenas.sources.get(span.source) else {
        return false;
    };
    directive_hash_passes(&source.bytes, span.start)
}

/// The directive name (first identifier after `#`), or `None` when the line
/// is null/malformed. Malformed lines stay the frozen PP05 decision.
pub fn directive_name(bus: &CompilerBus, line: &PpLine) -> Option<Vec<u8>> {
    let id = *line.tokens.get(1)?;
    let body = bus.arenas.pp_tokens.get(id).ok()?;
    (body.kind == PpTokenKind::Identifier).then(|| body.spelling.clone())
}

/// True when the directive continues onto the next raw physical line.
///
/// This is the bounded W1 continuation guard: raw-line grouping cannot
/// preserve a backslash-continued directive, so such a line fails closed
/// until the T03/T01 logical-line seam freezes. It checks both a token whose
/// mapped span starts on a later raw line and a trailing `\` before the
/// physical newline (the latter is how a spliced directive appears after
/// PP02).
pub fn line_has_continuation(bus: &CompilerBus, line: &PpLine) -> bool {
    let mut expected = line.line;
    let mut last_end: Option<(SourceId, u64)> = None;
    for (index, id) in line.tokens.iter().enumerate() {
        let Ok(body) = bus.arenas.pp_tokens.get(*id) else {
            return true;
        };
        let Ok(span) = bus.arenas.spans.get(body.span) else {
            return true;
        };
        let Ok(source) = bus.arenas.sources.get(span.source) else {
            return true;
        };
        let actual = line_index(&source.bytes, span.start);
        let actual_end = line_index(&source.bytes, span.end.saturating_sub(1));
        if actual != actual_end {
            // A spliced token can start on the raw line before the deleted
            // backslash-newline and end after it.
            return true;
        }
        if index == 0 {
            expected = actual;
        } else if actual != expected {
            return true;
        }
        last_end = Some((span.source, span.end));
    }
    let Some((source_id, end)) = last_end else {
        return false;
    };
    let Ok(source) = bus.arenas.sources.get(source_id) else {
        return true;
    };
    let bytes = &source.bytes;
    let start = (end as usize).min(bytes.len());
    let mut cursor = start;
    while cursor < bytes.len() && bytes[cursor] != b'\n' {
        cursor += 1;
    }
    let mut check = cursor;
    while check > start && matches!(bytes[check - 1], b' ' | b'\t' | b'\r') {
        check -= 1;
    }
    check > start && bytes[check - 1] == b'\\'
}

/// True for the six conditional directive names.
pub fn is_conditional_name(name: &[u8]) -> bool {
    matches!(
        name,
        b"if" | b"ifdef" | b"ifndef" | b"elif" | b"else" | b"endif"
    )
}

/// True for `define`.
pub fn is_define(name: &[u8]) -> bool {
    name == b"define"
}

/// True for `undef`.
pub fn is_undef(name: &[u8]) -> bool {
    name == b"undef"
}

/// Zero-based raw line index of a byte offset.
fn line_index(bytes: &[u8], offset: u64) -> usize {
    let end = (offset as usize).min(bytes.len());
    bytes[..end].iter().filter(|&&byte| byte == b'\n').count()
}

/// Raw walk-back from a `#`/`%:` offset (frozen `/21` predicate copy).
fn directive_hash_passes(bytes: &[u8], hash_start: u64) -> bool {
    let mut cursor = (hash_start as usize).min(bytes.len());
    loop {
        while cursor > 0 && (bytes[cursor - 1] == b' ' || bytes[cursor - 1] == b'\t') {
            cursor -= 1;
        }
        if cursor == 0 {
            return true;
        }
        if cursor >= 2 && bytes[cursor - 2] == b'*' && bytes[cursor - 1] == b'/' {
            match walk_back_block_comment(bytes, cursor) {
                Some(open) => {
                    cursor = open;
                    continue;
                }
                None => return false,
            }
        }
        return bytes[cursor - 1] == b'\n';
    }
}

/// Walk back over one closing `*/` to a same-line `/*`.
fn walk_back_block_comment(bytes: &[u8], close_end: usize) -> Option<usize> {
    let mut scan = close_end.saturating_sub(2);
    loop {
        if bytes.get(scan) == Some(&b'/') && bytes.get(scan + 1) == Some(&b'*') {
            if bytes[scan..close_end].contains(&b'\n') {
                return None;
            }
            return Some(scan);
        }
        if scan == 0 {
            return None;
        }
        scan -= 1;
    }
}
