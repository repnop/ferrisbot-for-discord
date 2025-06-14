use anyhow::Error;
use core::fmt::Write as _;
use syn::{parse_file, Item, ItemFn, Visibility};

use crate::types::Context;

use super::{
    api::{CrateType, Mode, PlayResult, PlaygroundRequest},
    util::{
        format_play_eval_stderr, generic_help, hoise_crate_attributes, parse_flags, send_reply, stub_message,
        GenericHelp,
    },
};

const BENCH_FUNCTION: &str = r#"
fn bench(functions: &[(&str, fn())]) {
    const CHUNK_SIZE: usize = 1000;

    // Warm up
    for (_, function) in functions.iter() {
        for _ in 0..CHUNK_SIZE {
            (function)();
        }
    }

    let mut functions_chunk_times = functions.iter().map(|_| Vec::new()).collect::<Vec<_>>();

    let start = std::time::Instant::now();
    while (std::time::Instant::now() - start).as_secs() < 5 {
        for (chunk_times, (_, function)) in functions_chunk_times.iter_mut().zip(functions) {
            let start = std::time::Instant::now();
            for _ in 0..CHUNK_SIZE {
                (function)();
            }
            chunk_times.push((std::time::Instant::now() - start).as_secs_f64() / CHUNK_SIZE as f64);
        }
    }

    for (chunk_times, (function_name, _)) in functions_chunk_times.iter().zip(functions) {
        let mean_time: f64 = chunk_times.iter().sum::<f64>() / chunk_times.len() as f64;
        
        let mut sum_of_squared_deviations = 0.0;
        let mut n = 0;
        for &time in chunk_times {
            // Filter out outliers (there are some crazy outliers, I've checked)
            if time < mean_time * 3.0 {
                sum_of_squared_deviations += (time - mean_time).powi(2);
                n += 1;
            }
        }
        let standard_deviation = f64::sqrt(sum_of_squared_deviations / n as f64);

        println!(
            "{}: {:.1}ns ± {:.1}",
            function_name,
            mean_time * 1_000_000_000.0,
            standard_deviation * 1_000_000_000.0,
        );
    }
}"#;

/// Benchmark small snippets of code
#[poise::command(prefix_command, track_edits, help_text_fn = "microbench_help", category = "Playground")]
pub async fn microbench(ctx: Context<'_>, flags: poise::KeyValueArgs, code: poise::CodeBlock) -> Result<(), Error> {
    ctx.say(stub_message(ctx)).await?;

    let user_code = &code.code;
    let black_box_hint = !user_code.contains("black_box");

    // insert convenience import for users
    let after_crate_attrs = "#[allow(unused_imports)] use std::hint::black_box;\n";

    let pub_fn_names: Vec<String> = extract_pub_fn_names_from_user_code(user_code);
    match pub_fn_names.len() {
        0 => {
            ctx.say("No public functions (`pub fn`) found for benchmarking :thinking:").await?;
            return Ok(());
        }
        1 => {
            ctx.say("Please include multiple functions. Times are not comparable across runs").await?;
            return Ok(());
        }
        _ => {}
    }

    // insert this after user code
    let mut after_code = BENCH_FUNCTION.to_owned();
    after_code += "fn main() {\nbench(&[";
    for function_name in pub_fn_names {
        writeln!(after_code, "(\"{function_name}\", {function_name}),").expect("Writing to a String should never fail");
    }
    after_code += "]);\n}\n";

    // final assembled code
    let code = hoise_crate_attributes(user_code, after_crate_attrs, &after_code);

    let (flags, mut flag_parse_errors) = parse_flags(flags);
    let mut result: PlayResult = ctx
        .data()
        .http
        .post("https://play.rust-lang.org/execute")
        .json(&PlaygroundRequest {
            code: &code,
            channel: flags.channel,
            crate_type: CrateType::Binary,
            edition: flags.edition,
            mode: Mode::Release, // benchmarks on debug don't make sense
            tests: false,
        })
        .send()
        .await?
        .json()
        .await?;

    result.stderr = format_play_eval_stderr(&result.stderr, flags.warn);

    if black_box_hint {
        flag_parse_errors += "Hint: use the black_box function to prevent computations from being optimized out\n";
    }
    send_reply(ctx, result, &code, &flags, &flag_parse_errors).await
}

#[must_use]
pub fn microbench_help() -> String {
    generic_help(GenericHelp {
        command: "microbench",
        desc: "\
Benchmarks small snippets of code by running them repeatedly. Public functions \
are run in blocks of 1000 repetitions in a cycle until 5 seconds have \
passed. Measurements are averaged and standard deviation is calculated for each

Use the `std::hint::black_box` function, which is already imported, to wrap results of \
computations that shouldn't be optimized out. Also wrap computation inputs in `black_box(...)` \
that should be opaque to the optimizer: `number * 2` produces optimized integer doubling assembly while \
`number * black_box(2)` produces a generic integer multiplication instruction",
        mode_and_channel: false,
        warn: true,
        run: false,
        aliasing_model: false,
        example_code: "
pub fn add() {
    black_box(black_box(42.0) + black_box(99.0));
}
pub fn mul() {
    black_box(black_box(42.0) * black_box(99.0));
}
",
    })
}

fn extract_pub_fn_names_from_user_code(code: &str) -> Vec<String> {
    let Ok(file) = parse_file(code) else {
        return vec![];
    };

    file.items
        .iter()
        .filter_map(|item| {
            if let Item::Fn(ItemFn { vis, sig, .. }) = item {
                if matches!(vis, Visibility::Public(_)) {
                    return Some(sig.ident.to_string());
                }
            }
            None
        })
        .collect()
}
