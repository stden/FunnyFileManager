//! Purpose: Prints the benchmark's results table, statistics and failures.

use std::time::Duration;

#[expect(clippy::print_stdout, reason = "the benchmark's report is its output")]
pub fn print_results(results: &[(&str, Duration, bool)]) {
    // Print results
    println!("┌─────────────────────────┬───────────┬────────┐");
    println!("│ Tool                    │ Time      │ Status │");
    println!("├─────────────────────────┼───────────┼────────┤");

    let mut total = Duration::ZERO;
    let mut pass_count = 0u32;

    for (name, dur, ok) in results {
        let status = if *ok { "  OK  " } else { " FAIL " };
        let time_str = format_duration(*dur);
        println!("│ {name:<23} │ {time_str:>9} │{status}│");
        total += *dur;
        if *ok {
            pass_count += 1;
        }
    }

    print_skipped_tools();

    println!("├─────────────────────────┼───────────┼────────┤");
    println!(
        "│ TOTAL ({} tools)         │ {:>9} │{}/{} ok│",
        results.len(),
        format_duration(total),
        pass_count,
        results.len()
    );
    println!("└─────────────────────────┴───────────┴────────┘");
    println!();
    println!("Skipped: {} S3 tools (need AWS credentials)", 6);

    print_statistics(results, total);
    print_failures(results);
}

fn format_duration(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms < 1.0 {
        format!("{:.0} µs", ms * 1000.0)
    } else if ms < 1000.0 {
        format!("{ms:.2} ms")
    } else {
        format!("{:.2} s", ms / 1000.0)
    }
}

#[expect(clippy::print_stdout, reason = "the benchmark's report is its output")]
fn print_statistics(results: &[(&str, Duration, bool)], total: Duration) {
    // No results, no stats.
    let (Some(&(min_name, _, _)), Some(&(max_name, _, _))) = (
        results.iter().min_by_key(|(_, d, _)| *d),
        results.iter().max_by_key(|(_, d, _)| *d),
    ) else {
        return;
    };
    let times: Vec<f64> = results
        .iter()
        .map(|(_, d, _)| d.as_secs_f64() * 1000.0)
        .collect();
    let count = u32::try_from(times.len()).unwrap_or(u32::MAX);
    let avg = times.iter().sum::<f64>() / f64::from(count);
    let min = times.iter().copied().fold(f64::INFINITY, f64::min);
    let max = times.iter().copied().fold(0.0f64, f64::max);
    println!();
    println!("Stats:");
    println!("  Avg: {avg:.2} ms");
    println!("  Min: {min:.2} ms ({min_name})");
    println!("  Max: {max:.2} ms ({max_name})");
    println!("  Total: {:.2} ms", total.as_secs_f64() * 1000.0);
}

#[expect(clippy::print_stdout, reason = "the benchmark's report is its output")]
fn print_failures(results: &[(&str, Duration, bool)]) {
    if results.iter().any(|(_, _, ok)| !ok) {
        println!();
        println!("FAILURES:");
        for (name, _, ok) in results {
            if !ok {
                println!("  - {name}");
            }
        }
    }
}

#[expect(clippy::print_stdout, reason = "the benchmark's report is its output")]
fn print_skipped_tools() {
    // S3 tools that require real AWS credentials — skip with note
    let s3_skipped = [
        "s3_list_buckets",
        "s3_list_objects",
        "s3_download",
        "s3_upload",
        "s3_delete",
        "s3_copy",
    ];

    for name in &s3_skipped {
        println!("│ {name:<23} │   skipped │  S3   │");
    }
}
