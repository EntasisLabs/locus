//! Host-side sketch of reactive memory.
//!
//! `MemoryReflexService` returns an envelope. This process moves that envelope
//! across a channel the way a host would hand it to its own event bus. The
//! channel is not part of the SDK, and nothing here subscribes to a broker.

use std::sync::Arc;

use locus_sdk::prelude::{
    HeuristicSystem1, MemoryReflexKind, MemoryReflexResponseDto, MemoryReflexService, MemoryScope,
    MemoryStimulus,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let reflex = MemoryReflexService::new(Arc::new(HeuristicSystem1));
    let (tx, mut rx) = mpsc::channel::<MemoryReflexResponseDto>(8);

    let stimuli = [
        MemoryStimulus {
            text: "thanks".to_string(),
            ..Default::default()
        },
        MemoryStimulus {
            text: "do you remember what we discussed about refunds".to_string(),
            role: Some("user".to_string()),
            scope: MemoryScope {
                session_ids: Some(vec!["demo".to_string()]),
                ..Default::default()
            },
            ..Default::default()
        },
        MemoryStimulus {
            text: "please remember that I prefer aisle seats".to_string(),
            role: Some("user".to_string()),
            ..Default::default()
        },
    ];

    for stimulus in stimuli {
        let envelope = MemoryReflexResponseDto::from(reflex.decide(&stimulus).await?);
        tx.send(envelope).await?;
    }
    drop(tx);

    while let Some(envelope) = rx.recv().await {
        println!(
            "topic={} kind={:?} action={:?} gate={:?} confidence={:.2} salience={:.2}",
            envelope.topic,
            envelope.kind,
            envelope.action,
            envelope.gate,
            envelope.confidence,
            envelope.salience,
        );
        if envelope.kind == MemoryReflexKind::Dispatch {
            if let Some(recall) = &envelope.recall {
                println!("  recall query={:?}", recall.query_text);
            }
            if let Some(persist) = &envelope.persist {
                println!("  persist text={}", persist.text);
            }
        }
    }

    let sample = MemoryStimulus {
        text: "do you remember the duplicate charge".to_string(),
        ..Default::default()
    };
    let request = reflex.request_for(&sample);
    println!(
        "system1 question names: {}",
        request
            .questions
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );

    // A Laya, sys1, or Jev server replaces the heuristic without changing the gate:
    // let decider = HttpSystem1::new("http://127.0.0.1:8000").with_model("typed-decisions");
    // let reflex = MemoryReflexService::new(Arc::new(decider));
    // let parsed = System1Response::parse_wire(&request.questions, &laya_json)?;
    // let envelope = reflex.apply(&sample, &parsed)?;

    Ok(())
}
