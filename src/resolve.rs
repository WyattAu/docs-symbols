//! The resolver: walk the tree-sitter AST by ordered name matching.

use tree_sitter::{Node, Parser, Tree};

use crate::error::SymbolError;
use crate::span::{anchor_id, Span};
use crate::Language;

/// Maximum descent depth: `a::b::c::...` — deep enough for modules inside
/// modules and impl methods; beyond that the path is rejected as malformed.
const MAX_SEGMENTS: usize = 8;

/// Extract the exact source span of the symbol at `symbol_path`.
///
/// Resolution walks the AST in document order at each scope, matching
/// item names (functions, structs, enums, traits, types, consts, statics,
/// modules, macros) and inherent/trait impls keyed on their type name.
/// The first match wins; that makes extraction independent of any line
/// shifts above the symbol (REQ-S1). The span is the item node itself:
/// byte-exact, untrimmed, doc-comment attributes excluded.
pub fn extract(source: &str, lang: Language, symbol_path: &str) -> Result<Span, SymbolError> {
    let segments = parse_path(symbol_path)?;
    let tree = parse(source, lang)?;
    let root = tree.root_node();
    let node = resolve(root, source, &segments, symbol_path)?;
    let code = source
        .get(node.start_byte()..node.end_byte())
        .unwrap_or_default()
        .to_string();
    Ok(Span {
        anchor_id: anchor_id(lang.label(), symbol_path),
        language: lang.label().to_string(),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        code,
    })
}

/// Split and validate the symbol path.
fn parse_path(symbol_path: &str) -> Result<Vec<&str>, SymbolError> {
    let segments: Vec<&str> = symbol_path.split("::").map(str::trim).collect();
    if segments.iter().any(|s| s.is_empty()) {
        return Err(SymbolError::BadPath {
            reason: "empty segment (leading, trailing, or doubled ::)".into(),
        });
    }
    if segments.len() > MAX_SEGMENTS {
        return Err(SymbolError::BadPath {
            reason: format!("deeper than {MAX_SEGMENTS} segments"),
        });
    }
    if segments.iter().any(|s| !is_ident(s)) {
        return Err(SymbolError::BadPath {
            reason: "segments must be plain identifiers".into(),
        });
    }
    Ok(segments)
}

/// Conservative identifier check: alphanumerics plus `_`, not starting
/// with a digit. Matches the item names the resolver can find anyway.
fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with(|c: char| c.is_ascii_digit())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse with the language's grammar; reject any tree with ERROR or
/// MISSING nodes anywhere (docs sources must compile cleanly; a broken
/// source must not silently yield a wrong span).
fn parse(source: &str, lang: Language) -> Result<Tree, SymbolError> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.grammar())
        .map_err(|e| SymbolError::Grammar(e.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| SymbolError::Grammar("parser returned no tree".into()))?;
    if tree.root_node().has_error() {
        let mut cursor = tree.root_node().walk();
        let reason = if find_error(&mut cursor) {
            let n = cursor.node();
            format!(
                "{} at byte {} (line {})",
                n.kind(),
                n.start_byte(),
                n.start_position().row + 1
            )
        } else {
            "ERROR or MISSING node".to_string()
        };
        return Err(SymbolError::Parse { reason });
    }
    Ok(tree)
}

fn find_error(cursor: &mut tree_sitter::TreeCursor) -> bool {
    loop {
        let node = cursor.node();
        if node.is_error() || node.is_missing() {
            return true;
        }
        if cursor.goto_first_child() {
            if find_error(cursor) {
                return true;
            }
            cursor.goto_parent();
        }
        if !cursor.goto_next_sibling() {
            return false;
        }
    }
}

/// Resolve `segments` inside `scope`, descending through modules and impls.
fn resolve<'a>(
    scope: Node<'a>,
    source: &str,
    segments: &[&str],
    full_path: &str,
) -> Result<Node<'a>, SymbolError> {
    let mut current = scope;
    for (depth, segment) in segments.iter().enumerate() {
        let last = depth + 1 == segments.len();
        match find_named(current, source, segment) {
            Match::Item(node) => {
                if last {
                    return Ok(node);
                }
                if node.kind() == "mod_item" {
                    let Some(body) = node.child_by_field_name("body") else {
                        return Err(unresolved(full_path, segment, "module has no body"));
                    };
                    current = body;
                } else {
                    return Err(unresolved(
                        full_path,
                        segment,
                        "deeper path requested, but the item has no child scope",
                    ));
                }
            }
            Match::ImplBodies(bodies) => {
                let Some(remaining) = segments.get(depth + 1..) else {
                    return Err(unresolved(
                        full_path,
                        segment,
                        "impl type name given as final segment",
                    ));
                };
                // Every impl of this type, in document order: the method
                // may live in the inherent impl or any trait impl.
                let mut first_err = None;
                for body in bodies {
                    match resolve(body, source, remaining, full_path) {
                        Ok(node) => return Ok(node),
                        Err(e) => first_err.get_or_insert(e),
                    };
                }
                return Err(first_err
                    .unwrap_or_else(|| unresolved(full_path, segment, "no impl body resolves")));
            }
            Match::None => {
                return Err(unresolved(
                    full_path,
                    segment,
                    "no item with that name in scope",
                ))
            }
        }
    }
    let Some(first) = segments.first() else {
        return Err(unresolved(full_path, "", "unreachable"));
    };
    Err(unresolved(full_path, first, "unreachable"))
}

/// The outcome of one scope scan for a single segment.
enum Match<'a> {
    Item(Node<'a>),
    /// Every impl body whose type name matched, in document order.
    ImplBodies(Vec<Node<'a>>),
    None,
}

/// One pass over a scope's children, in document order.
///
/// All impls of a type are collected (a method may live in the inherent
/// impl or any trait impl; the caller tries each in order). For a final
/// segment the first named item wins; document order is the deterministic
/// tiebreak.
fn find_named<'a>(scope: Node<'a>, source: &str, name: &str) -> Match<'a> {
    let mut cursor = scope.walk();
    if cursor.goto_first_child() {
        let mut item_hit: Option<Node<'a>> = None;
        let mut impl_bodies: Vec<Node<'a>> = Vec::new();
        loop {
            let node = cursor.node();
            match node.kind() {
                "function_item" | "struct_item" | "enum_item" | "trait_item" | "type_item"
                | "const_item" | "static_item" | "mod_item" | "macro_definition" => {
                    if item_hit.is_none()
                        && field_name(node, source, "name").as_deref() == Some(name)
                    {
                        item_hit = Some(node);
                    }
                }
                "impl_item" if field_name(node, source, "type").as_deref() == Some(name) => {
                    if let Some(body) = node.child_by_field_name("body") {
                        impl_bodies.push(body);
                    }
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        if !impl_bodies.is_empty() {
            return Match::ImplBodies(impl_bodies);
        }
        if let Some(node) = item_hit {
            return Match::Item(node);
        }
    }
    Match::None
}

fn unresolved(path: &str, segment: &str, reason: &str) -> SymbolError {
    SymbolError::Unresolvable {
        path: path.to_string(),
        reason: format!("segment `{segment}`: {reason}"),
    }
}

fn field_name(node: Node<'_>, source: &str, field: &str) -> Option<String> {
    let text = node
        .child_by_field_name(field)?
        .utf8_text(source.as_bytes())
        .ok()?;
    Some(text.to_string())
}
