//! Performance benchmarks for the editor's hot paths.
//!
//! These measure the per-edit / per-scroll work performed on large files:
//! syntax highlighting of a line, line wrapping, fold-region detection, and
//! search. Run them with:
//!
//! ```text
//! cargo bench -p iced-code-editor --features bench
//! ```

use std::collections::HashSet;
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use iced_code_editor::bench_support::{
    IncrementalEditBenchmark, IncrementalLspEditBenchmark,
    IncrementalNoWrapEditBenchmark, IncrementalSearchEditBenchmark, TextBuffer,
    WrappingCalculator, calculate_visual_line_range_len,
    compute_foldable_regions, find_matches, highlight_line_spans,
};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

/// Number of lines in the synthetic source file used by the benchmarks.
const SAMPLE_LINES: usize = 10_000;

/// Builds a synthetic Rust-like source file of `lines` lines.
///
/// The content mixes function headers, expressions with comments, macro calls
/// and closing braces so wrapping, folding, search and highlighting all have
/// representative work to do.
fn sample_source(lines: usize) -> String {
    let mut out = String::with_capacity(lines * 48);
    for i in 0..lines {
        match i % 4 {
            0 => {
                out.push_str(&format!(
                    "fn function_{i}(value: usize) -> usize {{\n"
                ));
            }
            1 => {
                out.push_str(&format!(
                    "    let result = value * {i} + 1; // compute result\n"
                ));
            }
            2 => out.push_str("    println!(\"{}\", result);\n"),
            _ => out.push_str("}\n"),
        }
    }
    out
}

/// Benchmarks tokenizing a single line into colored spans.
fn bench_highlight_line(c: &mut Criterion) {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let syntax = syntax_set
        .find_syntax_by_extension("rs")
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
    let theme = ThemeSet::load_defaults()
        .themes
        .get("base16-ocean.dark")
        .cloned()
        .unwrap_or_default();

    let line = "    let result = value * 42 + 1; // compute result here";

    c.bench_function("highlight_line_spans", |b| {
        b.iter(|| {
            highlight_line_spans(black_box(line), syntax, &theme, &syntax_set)
        });
    });
}

/// Benchmarks computing visual (wrapped) lines for a large buffer.
fn bench_wrapping(c: &mut Criterion) {
    let source = sample_source(SAMPLE_LINES);
    let buffer = TextBuffer::new(&source);
    let calculator = WrappingCalculator::new(true, None, 16.8, 8.4);
    let hidden = HashSet::new();

    c.bench_function("calculate_visual_lines_10k", |b| {
        b.iter(|| {
            calculator.calculate_visual_lines(
                black_box(&buffer),
                800.0,
                45.0,
                &hidden,
            )
        });
    });

    c.bench_function("calculate_affected_visual_lines_3_of_10k", |b| {
        b.iter(|| {
            calculate_visual_line_range_len(
                black_box(&calculator),
                black_box(&buffer),
                800.0,
                45.0,
                4_999,
                5_002,
            )
        });
    });

    let mut edit_benchmark =
        IncrementalEditBenchmark::new(&source, SAMPLE_LINES / 2, 4);
    c.bench_function("localized_insert_backspace_10k", |b| {
        b.iter(|| black_box(edit_benchmark.insert_and_backspace()));
    });
}

/// Benchmarks repeated line insertion/removal in the middle of a large buffer.
///
/// The line gap is primed once so this measures steady-state local editing,
/// analogous to repeatedly pressing Enter/Backspace near the same cursor.
fn bench_text_buffer_local_edits(c: &mut Criterion) {
    let mut buffer = TextBuffer::new(&sample_source(100_000));
    let edit_line = buffer.line_count() / 2;
    let mut temporary_line = String::from("temporary");

    // Move the gap to the edit location before timing steady-state edits.
    buffer.insert_line(edit_line, temporary_line);
    temporary_line = buffer.remove_line(edit_line).unwrap_or_default();

    c.bench_function("text_buffer_local_insert_remove_100k", |b| {
        b.iter(|| {
            buffer.insert_line(edit_line, std::mem::take(&mut temporary_line));
            temporary_line = buffer.remove_line(edit_line).unwrap_or_default();
            black_box(buffer.line_count())
        });
    });
}

/// Benchmarks normal typing with an attached LSP client on a 100k-line file.
fn bench_incremental_lsp_edits(c: &mut Criterion) {
    let source = sample_source(100_000);
    let mut benchmark = IncrementalLspEditBenchmark::new(&source, 50_000, 4);

    c.bench_function("localized_lsp_insert_backspace_100k", |b| {
        b.iter(|| black_box(benchmark.insert_and_backspace()));
    });
}

/// Benchmarks typing with wrapping disabled and a 100k-line width index.
fn bench_incremental_no_wrap_edits(c: &mut Criterion) {
    let source = sample_source(100_000);
    let mut benchmark = IncrementalNoWrapEditBenchmark::new(&source, 50_000, 4);

    c.bench_function("localized_no_wrap_insert_backspace_100k", |b| {
        b.iter(|| black_box(benchmark.insert_and_backspace()));
    });
}

/// Benchmarks typing while a search with many matches is open.
fn bench_incremental_search_edits(c: &mut Criterion) {
    let source = sample_source(100_000);
    let mut benchmark =
        IncrementalSearchEditBenchmark::new(&source, "result", 20_001, 4);

    c.bench_function("localized_search_insert_backspace_100k", |b| {
        b.iter(|| black_box(benchmark.insert_and_backspace()));
    });
}

/// Benchmarks fold-region detection for a large buffer.
fn bench_folding(c: &mut Criterion) {
    let buffer = TextBuffer::new(&sample_source(SAMPLE_LINES));

    c.bench_function("compute_foldable_regions_10k", |b| {
        b.iter(|| compute_foldable_regions(black_box(&buffer)));
    });
}

/// Benchmarks searching a large buffer for a common substring.
fn bench_search(c: &mut Criterion) {
    let buffer = TextBuffer::new(&sample_source(SAMPLE_LINES));
    let large_buffer = TextBuffer::new(&sample_source(100_000));

    c.bench_function("find_matches_10k", |b| {
        b.iter(|| {
            find_matches(black_box(&buffer), "result", false, Some(10_000))
        });
    });

    c.bench_function("find_matches_100k", |b| {
        b.iter(|| {
            find_matches(
                black_box(&large_buffer),
                "result",
                false,
                Some(10_000),
            )
        });
    });
}

criterion_group!(
    benches,
    bench_highlight_line,
    bench_wrapping,
    bench_text_buffer_local_edits,
    bench_incremental_lsp_edits,
    bench_incremental_no_wrap_edits,
    bench_incremental_search_edits,
    bench_folding,
    bench_search
);
criterion_main!(benches);
