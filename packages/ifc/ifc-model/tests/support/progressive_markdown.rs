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

fn fence_run(line: &str) -> Option<(u8, usize, &str)> {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let marker = *rest.as_bytes().first()?;
    if marker != b'`' && marker != b'~' {
        return None;
    }
    let length = rest.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then(|| (marker, length, &rest[length..]))
}

fn unfenced_lines(markdown: &str) -> Vec<&str> {
    let mut fence: Option<Fence> = None;
    let mut lines = Vec::new();
    for line in markdown.lines() {
        if let Some(open) = fence {
            if let Some((marker, length, tail)) = fence_run(line) {
                if marker == open.marker && length >= open.length && tail.trim().is_empty() {
                    fence = None;
                }
            }
            continue;
        }
        if let Some((marker, length, info)) = fence_run(line) {
            if marker != b'`' || !info.contains('`') {
                fence = Some(Fence { marker, length });
                continue;
            }
        }
        if !line.starts_with("    ") && !line.starts_with('\t') {
            lines.push(line);
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

pub(super) fn inline_code_tokens(markdown: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for line in unfenced_lines(markdown) {
        let mut rest = line;
        while let Some(open) = rest.find('`') {
            rest = &rest[open + 1..];
            let Some(close) = rest.find('`') else {
                break;
            };
            tokens.push(rest[..close].to_owned());
            rest = &rest[close + 1..];
        }
    }
    tokens
}

fn markdown_link_destinations(markdown: &str) -> Vec<String> {
    let mut destinations = Vec::new();
    for line in unfenced_lines(markdown) {
        let trimmed = line.trim_start();
        if let Some((_, tail)) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.split_once("]:"))
        {
            let destination = tail.trim_start();
            let destination = if let Some(destination) = destination.strip_prefix('<') {
                destination.split_once('>').map(|(value, _)| value)
            } else {
                destination
                    .split(|ch: char| ch.is_whitespace())
                    .next()
                    .filter(|value| !value.is_empty())
            };
            if let Some(destination) = destination {
                destinations.push(destination.to_owned());
            }
        }

        let mut rest = line;
        while let Some(open) = rest.find("](") {
            rest = &rest[open + 2..];
            let (destination, consumed) = if let Some(angle) = rest.strip_prefix('<') {
                match angle.find('>') {
                    Some(end) => (&angle[..end], end + 2),
                    None => break,
                }
            } else {
                let end = rest
                    .find(|ch: char| ch == ')' || ch.is_whitespace())
                    .unwrap_or(rest.len());
                (&rest[..end], end)
            };
            if !destination.is_empty() {
                destinations.push(destination.to_owned());
            }
            rest = &rest[consumed.min(rest.len())..];
        }
    }
    destinations
}

pub(super) fn context_pointer_tokens(markdown: &str) -> Vec<String> {
    let mut tokens = inline_code_tokens(markdown);
    tokens.extend(markdown_link_destinations(markdown));
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

pub(super) fn task_prerequisites(plan: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut current = None;
    let mut prerequisites = BTreeMap::<String, BTreeSet<String>>::new();
    for line in unfenced_lines(plan) {
        if let Some(task) = task_from_line(line) {
            current = Some(task.id);
            continue;
        }
        if line.trim_start().starts_with('#') {
            current = None;
            continue;
        }
        let Some(payload) = prerequisite_payload(line) else {
            continue;
        };
        let owner = current
            .as_ref()
            .expect("Requires line must follow a task declaration");
        prerequisites
            .entry(owner.clone())
            .or_default()
            .extend(task_references(payload));
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
fn context_pointer_filter_accepts_only_local_documents() {
    assert!(super::is_context_pointer("../../PLAN.md"));
    assert!(!super::is_context_pointer(
        "https://example.invalid/PLAN.md"
    ));
    assert!(!super::is_context_pointer("mailto:owner@PLAN.md"));
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
