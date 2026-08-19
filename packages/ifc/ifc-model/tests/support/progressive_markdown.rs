use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Task {
    pub(super) id: String,
    pub(super) complete: bool,
}

#[derive(Clone, Copy)]
struct Fence {
    marker: u8,
    length: usize,
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn list_marker_width(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    match bytes {
        [marker @ (b'-' | b'*' | b'+'), whitespace, ..]
            if marker.is_ascii_punctuation() && whitespace.is_ascii_whitespace() =>
        {
            Some(2)
        }
        _ => {
            let digits = bytes
                .iter()
                .take(9)
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            (digits > 0
                && matches!(bytes.get(digits), Some(b'.' | b')'))
                && bytes.get(digits + 1).is_some_and(u8::is_ascii_whitespace))
            .then_some(digits + 2)
        }
    }
}

fn blockquote_content(mut line: &str) -> &str {
    loop {
        let indent = leading_spaces(line).min(3);
        let rest = &line[indent..];
        let Some(after_marker) = rest.strip_prefix('>') else {
            return rest;
        };
        line = after_marker.strip_prefix(' ').unwrap_or(after_marker);
    }
}

fn line_list_content_indent(line: &str) -> Option<usize> {
    let indent = leading_spaces(line);
    if indent <= 3 {
        if let Some(width) = list_marker_width(&line[indent..]) {
            return Some(indent + width);
        }
    }
    let content = blockquote_content(line);
    list_marker_width(content)
}

fn fence_run(mut line: &str, allow_indented: bool) -> Option<(u8, usize, &str)> {
    let mut in_container = false;
    loop {
        let indent = leading_spaces(line);
        if indent > 3 && !in_container && !allow_indented {
            return None;
        }
        let rest = &line[indent..];
        if let Some(after_marker) = rest.strip_prefix('>') {
            in_container = true;
            line = after_marker.strip_prefix(' ').unwrap_or(after_marker);
            continue;
        }
        if let Some(width) = list_marker_width(rest) {
            in_container = true;
            line = &rest[width..];
            continue;
        }
        line = rest;
        break;
    }

    let marker = *line.as_bytes().first()?;
    if marker != b'`' && marker != b'~' {
        return None;
    }
    let length = line.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then(|| (marker, length, &line[length..]))
}

fn unfenced_lines(markdown: &str) -> Vec<&str> {
    let mut fence: Option<Fence> = None;
    let mut list_content_indent = None;
    let mut lines = Vec::new();
    for line in markdown.lines() {
        if let Some(open) = fence {
            if let Some((marker, length, tail)) = fence_run(line, true) {
                if marker == open.marker && length >= open.length && tail.trim().is_empty() {
                    fence = None;
                }
            }
            continue;
        }

        let marker_indent = line_list_content_indent(line);
        if let Some((marker, length, info)) = fence_run(line, list_content_indent.is_some()) {
            if marker != b'`' || !info.contains('`') {
                if marker_indent.is_none() && leading_spaces(line) <= 3 {
                    list_content_indent = None;
                }
                fence = Some(Fence { marker, length });
                continue;
            }
        }

        let trimmed = line.trim_start();
        let rust_doc = trimmed.starts_with("///") || trimmed.starts_with("//!");
        let indentation = leading_spaces(line);
        let code_indent = list_content_indent.map_or(4, |content| content + 4);
        let indented_code = line.starts_with('\t') || indentation >= code_indent;
        if !indented_code || rust_doc {
            lines.push(line);
        }

        if let Some(marker_indent) = marker_indent {
            list_content_indent = Some(marker_indent);
        } else if !trimmed.is_empty()
            && list_content_indent.is_some_and(|content| indentation < content)
        {
            list_content_indent = None;
        }
    }
    lines
}

fn valid_task_id(token: &str) -> bool {
    let mut segments = token.split('-');
    let Some(first) = segments.next() else {
        return false;
    };
    let rest: Vec<_> = segments.collect();
    !first.is_empty()
        && !rest.is_empty()
        && first
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        && rest.iter().all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        })
}

fn task_from_line(line: &str) -> Option<Task> {
    let line = line.trim_start();
    let (complete, rest) = [(false, "- [ ] "), (true, "- [x] "), (true, "- [X] ")]
        .into_iter()
        .find_map(|(complete, prefix)| line.strip_prefix(prefix).map(|rest| (complete, rest)))?;
    let rest = rest.strip_prefix('`')?;
    let (id, description) = rest.split_once('`')?;
    (valid_task_id(id) && description.starts_with(" - ")).then(|| Task {
        id: id.to_owned(),
        complete,
    })
}

pub(super) fn task_entries(plan: &str) -> Vec<Task> {
    unfenced_lines(plan)
        .into_iter()
        .filter_map(task_from_line)
        .collect()
}

pub(super) fn task_ids(plan: &str) -> Vec<String> {
    task_entries(plan).into_iter().map(|task| task.id).collect()
}

pub(super) fn task_checkbox_line_count(plan: &str) -> usize {
    unfenced_lines(plan)
        .into_iter()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("- [ ] ") || line.starts_with("- [x] ") || line.starts_with("- [X] ")
        })
        .count()
}

#[derive(Debug)]
struct CodeSpan {
    start: usize,
    end: usize,
    token: String,
}

fn is_escaped(bytes: &[u8], index: usize) -> bool {
    let mut slashes = 0;
    let mut cursor = index;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        slashes += 1;
        cursor -= 1;
    }
    slashes % 2 == 1
}

fn delimiter_run(bytes: &[u8], start: usize, marker: u8) -> usize {
    bytes[start..]
        .iter()
        .take_while(|byte| **byte == marker)
        .count()
}

fn code_spans(line: &str) -> Vec<CodeSpan> {
    let bytes = line.as_bytes();
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'`' || is_escaped(bytes, cursor) {
            cursor += 1;
            continue;
        }
        let opening = delimiter_run(bytes, cursor, b'`');
        let mut close = cursor + opening;
        let mut matched = None;
        while close < bytes.len() {
            if bytes[close] != b'`' || is_escaped(bytes, close) {
                close += 1;
                continue;
            }
            let closing = delimiter_run(bytes, close, b'`');
            if closing == opening {
                matched = Some(close);
                break;
            }
            close += closing;
        }
        let Some(close) = matched else {
            cursor += opening;
            continue;
        };
        let mut token = &line[cursor + opening..close];
        if token.starts_with(' ') && token.ends_with(' ') && token.bytes().any(|byte| byte != b' ')
        {
            token = &token[1..token.len() - 1];
        }
        spans.push(CodeSpan {
            start: cursor,
            end: close + opening,
            token: token.to_owned(),
        });
        cursor = close + opening;
    }
    spans
}

pub(super) fn inline_code_tokens(markdown: &str) -> Vec<String> {
    unfenced_lines(markdown)
        .into_iter()
        .flat_map(code_spans)
        .map(|span| span.token)
        .collect()
}

fn in_code_span(spans: &[CodeSpan], index: usize) -> bool {
    spans
        .iter()
        .any(|span| index >= span.start && index < span.end)
}

fn matching_label_end(line: &str, start: usize, spans: &[CodeSpan]) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut depth = 1;
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        if let Some(span) = spans
            .iter()
            .find(|span| cursor >= span.start && cursor < span.end)
        {
            cursor = span.end;
            continue;
        }
        if is_escaped(bytes, cursor) {
            cursor += 1;
            continue;
        }
        match bytes[cursor] {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn unescape_destination(destination: &str) -> String {
    let mut unescaped = String::with_capacity(destination.len());
    let mut chars = destination.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                unescaped.push(ch);
                break;
            };
            if next.is_ascii_punctuation() {
                unescaped.push(next);
            } else {
                unescaped.push(ch);
                unescaped.push(next);
            }
        } else {
            unescaped.push(ch);
        }
    }
    unescaped
}

fn inline_destination(line: &str, opening: usize) -> Option<(String, usize)> {
    let bytes = line.as_bytes();
    let mut cursor = opening + 1;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }

    if bytes.get(cursor) == Some(&b'<') {
        let start = cursor + 1;
        cursor = start;
        while cursor < bytes.len() && (bytes[cursor] != b'>' || is_escaped(bytes, cursor)) {
            cursor += 1;
        }
        if cursor == bytes.len() {
            return None;
        }
        let destination = unescape_destination(&line[start..cursor]);
        cursor += 1;
        while cursor < bytes.len() && (bytes[cursor] != b')' || is_escaped(bytes, cursor)) {
            cursor += 1;
        }
        return (cursor < bytes.len()).then_some((destination, cursor + 1));
    }

    let start = cursor;
    let mut depth = 0;
    let mut destination_end = None;
    while cursor < bytes.len() {
        if is_escaped(bytes, cursor) {
            cursor += 2;
            continue;
        }
        match bytes[cursor] {
            b'(' if destination_end.is_none() => depth += 1,
            b')' if depth > 0 && destination_end.is_none() => depth -= 1,
            b')' if depth == 0 => {
                let end = destination_end.unwrap_or(cursor);
                let destination = unescape_destination(&line[start..end]);
                return (!destination.is_empty()).then_some((destination, cursor + 1));
            }
            byte if byte.is_ascii_whitespace() && depth == 0 => {
                destination_end.get_or_insert(cursor);
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn reference_destination(line: &str) -> Option<String> {
    let line = blockquote_content(line);
    let (_, tail) = line.strip_prefix('[')?.split_once("]:")?;
    let tail = tail.trim_start();
    if let Some(tail) = tail.strip_prefix('<') {
        let end = tail
            .bytes()
            .enumerate()
            .find(|(index, byte)| *byte == b'>' && !is_escaped(tail.as_bytes(), *index))?
            .0;
        return Some(unescape_destination(&tail[..end]));
    }

    let bytes = tail.as_bytes();
    let mut depth = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if is_escaped(bytes, cursor) {
            cursor += 2;
            continue;
        }
        match bytes[cursor] {
            b'(' => depth += 1,
            b')' if depth > 0 => depth -= 1,
            byte if byte.is_ascii_whitespace() && depth == 0 => break,
            _ => {}
        }
        cursor += 1;
    }
    (cursor > 0).then(|| unescape_destination(&tail[..cursor]))
}

fn markdown_link_destinations(markdown: &str) -> Vec<String> {
    let mut destinations = Vec::new();
    for line in unfenced_lines(markdown) {
        if let Some(destination) = reference_destination(line) {
            destinations.push(destination);
        }

        let spans = code_spans(line);
        let bytes = line.as_bytes();
        let mut cursor = 0;
        while cursor < bytes.len() {
            if let Some(span) = spans
                .iter()
                .find(|span| cursor >= span.start && cursor < span.end)
            {
                cursor = span.end;
                continue;
            }
            if bytes[cursor] != b'[' || is_escaped(bytes, cursor) {
                cursor += 1;
                continue;
            }
            let Some(label_end) = matching_label_end(line, cursor, &spans) else {
                cursor += 1;
                continue;
            };
            let opening = label_end + 1;
            if bytes.get(opening) != Some(&b'(') || in_code_span(&spans, opening) {
                cursor = label_end + 1;
                continue;
            }
            let Some((destination, consumed)) = inline_destination(line, opening) else {
                cursor = opening + 1;
                continue;
            };
            destinations.push(destination);
            cursor = consumed;
        }
    }
    destinations
}

pub(super) fn context_pointer_tokens(markdown: &str) -> Vec<String> {
    let mut tokens = inline_code_tokens(markdown);
    tokens.extend(
        markdown_link_destinations(markdown)
            .into_iter()
            .map(|destination| match destination.find(['#', '?']) {
                Some(end) => destination[..end].to_owned(),
                None => destination,
            }),
    );
    tokens
}

pub(super) fn task_references(plan: &str) -> BTreeSet<String> {
    inline_code_tokens(plan)
        .into_iter()
        .filter(|token| valid_task_id(token))
        .collect()
}

fn prerequisite_payload(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let line = ["- ", "* ", "+ "]
        .into_iter()
        .find_map(|prefix| line.strip_prefix(prefix))
        .unwrap_or(line);
    ["Requires:", "Prerequisites:"]
        .into_iter()
        .find_map(|prefix| line.strip_prefix(prefix))
}

fn is_setext_underline(line: &str) -> bool {
    let indent = leading_spaces(line);
    if indent > 3 {
        return false;
    }
    let trimmed = line[indent..].trim_end();
    !trimmed.is_empty()
        && (trimmed.bytes().all(|b| b == b'-') || trimmed.bytes().all(|b| b == b'='))
}

pub(super) fn task_prerequisites(plan: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut current = None;
    let mut collecting = false;
    let mut prev_plain_text = false;
    let mut prerequisites = BTreeMap::<String, BTreeSet<String>>::new();
    for line in unfenced_lines(plan) {
        if let Some(task) = task_from_line(line) {
            current = Some(task.id);
            collecting = false;
            prev_plain_text = false;
            continue;
        }
        if line.trim().is_empty() {
            collecting = false;
            prev_plain_text = false;
            continue;
        }
        if line.trim_start().starts_with('#') {
            current = None;
            collecting = false;
            prev_plain_text = false;
            continue;
        }
        if is_setext_underline(line) {
            if prev_plain_text {
                current = None;
            }
            collecting = false;
            prev_plain_text = false;
            continue;
        }
        if let Some(payload) = prerequisite_payload(line) {
            let owner = current
                .as_ref()
                .expect("Requires line must follow a task declaration");
            prerequisites
                .entry(owner.clone())
                .or_default()
                .extend(task_references(payload));
            collecting = true;
            prev_plain_text = false;
            continue;
        }
        if collecting {
            let owner = current
                .as_ref()
                .expect("Requires line must follow a task declaration");
            prerequisites
                .entry(owner.clone())
                .or_default()
                .extend(task_references(line.trim_start()));
            prev_plain_text = false;
            continue;
        }
        prev_plain_text = true;
    }
    prerequisites
}

#[test]
fn parser_ignores_commonmark_code_and_preserves_real_plan_state() {
    let plan = r#"
- [ ] `REAL-TASK` - pending
  * Prerequisites: `DONE-TASK`.
- [X] `DONE-TASK` - complete
~~~text
- [ ] `FAKE-TASK` - tilde-fenced example
  - Requires: `REAL-TASK`.
~~~
```text
- [ ] `OTHER-FAKE` - backtick-fenced example
```
    - [ ] `INDENTED-FAKE` - indented code
unmatched `BROKEN-TASK
- [ ] `not-a-task` - invalid grammar
"#;
    assert_eq!(
        task_entries(plan),
        [
            Task {
                id: "REAL-TASK".to_owned(),
                complete: false,
            },
            Task {
                id: "DONE-TASK".to_owned(),
                complete: true,
            },
        ]
    );
    assert_eq!(task_checkbox_line_count(plan), 3);
    assert_eq!(
        task_references(plan),
        BTreeSet::from(["DONE-TASK".to_owned(), "REAL-TASK".to_owned()])
    );
    assert_eq!(
        task_prerequisites(plan),
        BTreeMap::from([(
            "REAL-TASK".to_owned(),
            BTreeSet::from(["DONE-TASK".to_owned()]),
        )])
    );
}

#[test]
fn wrapped_prerequisites_remain_owned_by_the_current_task() {
    let plan = "- [ ] `OWNER-TASK` - pending\n  - Requires: `FIRST-TASK`,\n    `SECOND-TASK`.\n- [ ] `FIRST-TASK` - pending\n- [ ] `SECOND-TASK` - pending\n";
    assert_eq!(
        task_prerequisites(plan),
        BTreeMap::from([(
            "OWNER-TASK".to_owned(),
            BTreeSet::from(["FIRST-TASK".to_owned(), "SECOND-TASK".to_owned()]),
        )])
    );
}

#[test]
#[should_panic(expected = "Requires line must follow a task declaration")]
fn setext_heading_ends_task_ownership() {
    task_prerequisites(
        "- [ ] `OWNER-TASK` - pending\nRelease boundary\n----------------\nRequires: `ORPHAN-TASK`.\n",
    );
}

#[test]
fn nested_prerequisites_and_indented_rust_docs_remain_visible() {
    let plan =
        "- [ ] `REAL-TASK` - pending\n    - Requires: `WAIT-TASK`.\n- [ ] `WAIT-TASK` - pending\n";
    assert_eq!(
        task_prerequisites(plan),
        BTreeMap::from([(
            "REAL-TASK".to_owned(),
            BTreeSet::from(["WAIT-TASK".to_owned()]),
        )])
    );

    let rust = "fn marker() {\n    /// See `../../PLAN.md`.\n}\n";
    let pointers: Vec<_> = context_pointer_tokens(rust)
        .into_iter()
        .filter(|token| super::is_context_pointer(token))
        .collect();
    assert_eq!(pointers, ["../../PLAN.md"]);
}

#[test]
fn context_pointer_filter_accepts_only_local_documents() {
    assert!(super::is_context_pointer("../../PLAN.md"));
    assert!(!super::is_context_pointer(
        "https://example.invalid/PLAN.md"
    ));
    assert!(!super::is_context_pointer("mailto:owner@PLAN.md"));
}

#[test]
fn balanced_link_destinations_and_fragments_resolve_to_files() {
    // NOTES.md deliberately avoids the AGENTS.md/PLAN.md filenames that
    // `is_context_pointer` recognizes, so this fixture cannot be picked up
    // by the real corpus-pointer gate that also scans this source file's
    // own text for context pointers.
    let markdown = r#"
[balanced](docs/(draft)/NOTES.md#phase)
[escaped](docs/\(draft\)/OTHER-NOTES.md?view=full)
[invalid](missing/NOTES.md
"#;
    assert_eq!(
        context_pointer_tokens(markdown)
            .into_iter()
            .filter(|token| !token.contains("://"))
            .collect::<Vec<_>>(),
        ["docs/(draft)/NOTES.md", "docs/(draft)/OTHER-NOTES.md"]
    );
}

#[test]
fn delimiter_runs_follow_commonmark_code_span_scope() {
    assert_eq!(inline_code_tokens(r"``a ` b`` and \`not code\`"), ["a ` b"]);
}

#[test]
fn code_spans_and_escaped_links_do_not_emit_destinations() {
    let markdown = r#"
Show `[literal](missing/PLAN.md)` and \[escaped](other/AGENTS.md).
Also show `` `[nested](third/PLAN.md)` `` literally.
Follow [real](../../PLAN.md).
"#;
    let pointers: Vec<_> = context_pointer_tokens(markdown)
        .into_iter()
        .filter(|token| super::is_context_pointer(token))
        .collect();
    assert_eq!(pointers, ["../../PLAN.md"]);
}

#[test]
fn container_fences_hide_example_links_and_references() {
    let markdown = r#"
> ```markdown
> [quoted fake](missing/PLAN.md)
> `missing/AGENTS.md`
> ```
- Example:
    ~~~markdown
    [listed fake](other/PLAN.md)
    ~~~
[real](../../PLAN.md)
"#;
    let pointers: Vec<_> = context_pointer_tokens(markdown)
        .into_iter()
        .filter(|token| super::is_context_pointer(token))
        .collect();
    assert_eq!(pointers, ["../../PLAN.md"]);
}

#[test]
fn context_tokens_include_links_but_ignore_fenced_examples() {
    let markdown = r#"
Follow `../../PLAN.md` and [the parent](../../AGENTS.md).
[reference]: ../../PLAN.md
~~~markdown
`missing/AGENTS.md`
[fake](missing/PLAN.md)
~~~
"#;
    assert_eq!(
        context_pointer_tokens(markdown),
        [
            "../../PLAN.md".to_owned(),
            "../../AGENTS.md".to_owned(),
            "../../PLAN.md".to_owned(),
        ]
    );
}
