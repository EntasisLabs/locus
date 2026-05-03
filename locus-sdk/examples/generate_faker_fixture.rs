use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use locus_sdk::prelude::{FakerConfig, SttpFakerBuilder, write_jsonl_fixture};

fn main() -> Result<()> {
    let mut config = FakerConfig::default();
    let mut output = PathBuf::from("../docs/example_data/pipeline/sttp_faker_fixture_v1.jsonl");

    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => {
                output = PathBuf::from(arg_value(&args, i, "--output")?);
                i += 2;
            }
            "--seed" => {
                config.seed = arg_value(&args, i, "--seed")?.parse().context("invalid --seed")?;
                i += 2;
            }
            "--sessions" => {
                config.sessions = arg_value(&args, i, "--sessions")?
                    .parse()
                    .context("invalid --sessions")?;
                i += 2;
            }
            "--min-nodes" => {
                config.min_nodes_per_session = arg_value(&args, i, "--min-nodes")?
                    .parse()
                    .context("invalid --min-nodes")?;
                i += 2;
            }
            "--max-nodes" => {
                config.max_nodes_per_session = arg_value(&args, i, "--max-nodes")?
                    .parse()
                    .context("invalid --max-nodes")?;
                i += 2;
            }
            "--filler-ratio" => {
                config.filler_ratio = arg_value(&args, i, "--filler-ratio")?
                    .parse()
                    .context("invalid --filler-ratio")?;
                i += 2;
            }
            "--topic-drift" => {
                config.topic_drift = arg_value(&args, i, "--topic-drift")?
                    .parse()
                    .context("invalid --topic-drift")?;
                i += 2;
            }
            "--span-days" => {
                config.timestamp_span_days = arg_value(&args, i, "--span-days")?
                    .parse()
                    .context("invalid --span-days")?;
                i += 2;
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => {
                anyhow::bail!("unknown argument: {other}");
            }
        }
    }

    if config.max_nodes_per_session < config.min_nodes_per_session {
        config.max_nodes_per_session = config.min_nodes_per_session;
    }

    let builder = SttpFakerBuilder::new(config);
    let records = builder.generate();

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory: {}", parent.display()))?;
    }

    write_jsonl_fixture(&output, &records)
        .with_context(|| format!("failed to write fixture: {}", output.display()))?;

    println!("generated {} records", records.len());
    println!("output: {}", output.display());

    Ok(())
}

fn arg_value(args: &[String], index: usize, flag: &str) -> Result<String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing value for {flag}"))
}

fn print_help() {
    println!("generate_faker_fixture options:");
    println!("  --output <path>           Output JSONL path");
    println!("  --seed <u64>              RNG seed");
    println!("  --sessions <usize>        Number of sessions");
    println!("  --min-nodes <usize>       Minimum nodes per session");
    println!("  --max-nodes <usize>       Maximum nodes per session");
    println!("  --filler-ratio <f32>      Filler noise ratio");
    println!("  --topic-drift <f32>       Topic drift ratio");
    println!("  --span-days <usize>       Timestamp span in days");
}
