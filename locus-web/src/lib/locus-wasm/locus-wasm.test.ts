import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { initSync, parse_sttp, version } from './pkg/locus_wasm.js';

const wasmPath = fileURLToPath(new URL('./pkg/locus_wasm_bg.wasm', import.meta.url));

describe('locus-wasm', () => {
	it('parses a minimal STTP node', () => {
		initSync(readFileSync(wasmPath));

		const info = version() as { core: string; sdk: string; wasm: string };
		expect(info.wasm).toBeTruthy();

		const raw = `⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "demo", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "demo", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "demo", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { note(.99): "example" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩`;

		const parsed = parse_sttp(raw, 'demo', 'tolerant') as { success: boolean };
		expect(parsed.success).toBe(true);
	});
});
