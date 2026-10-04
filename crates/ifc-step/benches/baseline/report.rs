//! Results, the environment they were taken in, and their two outputs: a
//! Markdown table on stdout and, with `--json`, every sample on disk for
//! `benchmarks/baseline.py` to pool across runs and compare.

use crate::alloc::Memory;
use crate::stats::{Plan, Summary};
use std::fmt::Write as _;
use std::path::Path;

/// One benchmark on one workload.
pub(crate) struct Record {
    pub(crate) workload: String,
    pub(crate) bench: &'static str,
    pub(crate) entities: usize,
    pub(crate) bytes: usize,
    /// Milliseconds per sample, warm-up excluded.
    pub(crate) samples: Vec<f64>,
    pub(crate) memory: Option<Memory>,
    /// The asserted output checksum: equal across runs and machines.
    pub(crate) checksum: u64,
}

/// Collects records under one sampling plan.
pub(crate) struct Recorder {
    pub(crate) plan: Plan,
    pub(crate) records: Vec<Record>,
}

impl Recorder {
    pub(crate) fn push(&mut self, record: Record) {
        self.records.push(record);
    }
}

fn first_line_with(path: &str, prefix: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let line = text.lines().find(|line| line.starts_with(prefix))?;
    Some(
        line[prefix.len()..]
            .trim()
            .trim_matches('"')
            .trim_start_matches(':')
            .trim()
            .to_owned(),
    )
}

/// The 1, 5 and 15 minute load averages.
pub(crate) fn load() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .map(|text| {
            text.split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| "unknown".into())
}

/// What the run needs to be read against. Linux sources; elsewhere a field
/// reads `unknown` and the run script's own record has to stand in.
pub(crate) fn environment(plan: Plan, probe: bool) -> Vec<(&'static str, String)> {
    let unknown = || "unknown".to_owned();
    let profile = if cfg!(debug_assertions) {
        "test (debug assertions on: a smoke run, not a measurement)"
    } else {
        "bench (release: opt-level 3, thin LTO, 1 codegen unit)"
    };
    vec![
        (
            "cpu",
            first_line_with("/proc/cpuinfo", "model name").unwrap_or_else(unknown),
        ),
        (
            "threads available",
            std::thread::available_parallelism().map_or_else(|_| unknown(), |n| n.to_string()),
        ),
        (
            "cpus allowed",
            first_line_with("/proc/self/status", "Cpus_allowed_list").unwrap_or_else(unknown),
        ),
        (
            "memory",
            first_line_with("/proc/meminfo", "MemTotal").unwrap_or_else(unknown),
        ),
        (
            "os",
            first_line_with("/etc/os-release", "PRETTY_NAME=").unwrap_or_else(unknown),
        ),
        (
            "kernel",
            std::fs::read_to_string("/proc/sys/kernel/osrelease")
                .map_or_else(|_| unknown(), |text| text.trim().to_owned()),
        ),
        ("profile", profile.to_owned()),
        (
            "unix time",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or_else(|_| unknown(), |d| d.as_secs().to_string()),
        ),
        ("warm-up", plan.warmup.to_string()),
        ("samples", plan.samples.to_string()),
        ("probe", probe.to_string()),
    ]
}

fn mb(bytes: isize) -> String {
    format!("{:.2}", bytes as f64 / 1_048_576.0)
}

fn ms(x: f64) -> String {
    if x < 1.0 {
        format!("{x:.4}")
    } else if x < 100.0 {
        format!("{x:.2}")
    } else {
        format!("{x:.0}")
    }
}

/// The run as Markdown: environment, then one row per record.
pub(crate) fn markdown(env: &[(&str, String)], load: (&str, &str), records: &[Record]) -> String {
    let mut out = String::new();
    for (key, value) in env {
        let _ = writeln!(out, "- {key}: {value}");
    }
    let _ = writeln!(out, "- load before (1/5/15 min): {}", load.0);
    let _ = writeln!(out, "- load after (1/5/15 min): {}\n", load.1);
    out.push_str(
        "| workload | entities | bench | median ms | IQR ms | MAD ms | min..max ms | \
         heap retained MB | heap peak MB | checksum |\n\
         | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n",
    );
    for record in records {
        let s = Summary::of(&record.samples);
        let (retained, peak) = record
            .memory
            .map_or(("".into(), "".into()), |m| (mb(m.retained), mb(m.peak)));
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {}..{} | {} | {}..{} | {} | {} | {:016x} |",
            record.workload,
            record.entities,
            record.bench,
            ms(s.median),
            ms(s.p25),
            ms(s.p75),
            ms(s.mad),
            ms(s.min),
            ms(s.max),
            retained,
            peak,
            record.checksum,
        );
    }
    out
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Every sample, as JSON, for pooling runs and comparing against a
/// baseline.
pub(crate) fn write_json(
    path: &Path,
    env: &[(&str, String)],
    load: (&str, &str),
    records: &[Record],
) -> std::io::Result<()> {
    let mut out = String::from("{\n  \"environment\": {\n");
    for (key, value) in env {
        let _ = writeln!(out, "    {}: {},", json_string(key), json_string(value));
    }
    let _ = writeln!(out, "    \"load before\": {},", json_string(load.0));
    let _ = writeln!(out, "    \"load after\": {}\n  }},", json_string(load.1));
    out.push_str("  \"results\": [\n");
    for (i, record) in records.iter().enumerate() {
        let samples: Vec<String> = record.samples.iter().map(|x| format!("{x:.6}")).collect();
        let memory = record.memory.map_or("null".into(), |m| {
            format!("{{\"retained\": {}, \"peak\": {}}}", m.retained, m.peak)
        });
        let _ = writeln!(
            out,
            "    {{\"workload\": {}, \"bench\": {}, \"entities\": {}, \"bytes\": {}, \
             \"checksum\": \"{:016x}\", \"heap_bytes\": {}, \"ms\": [{}]}}{}",
            json_string(&record.workload),
            json_string(record.bench),
            record.entities,
            record.bytes,
            record.checksum,
            memory,
            samples.join(", "),
            if i + 1 < records.len() { "," } else { "" },
        );
    }
    out.push_str("  ]\n}\n");
    std::fs::write(path, out)
}
