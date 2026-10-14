import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { WasmLocusClient, compile_note, initSync, parse_sttp, version } from './pkg/locus_wasm.js';

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

	it('stores a compiled note and recalls it by the words in the question', async () => {
		initSync(readFileSync(wasmPath));

		const client = new WasmLocusClient();
		const compiled = compile_note(
			'We decided the parser should accept both strict and tolerant STTP.',
			'homepage-demo'
		) as { canonical: string; contextSummary: string };

		expect(compiled.canonical).toContain('note(.99)');

		const stored = (await client.store(compiled.canonical, 'homepage-demo')) as {
			valid: boolean;
			validationError?: string;
		};
		expect(stored.valid, stored.validationError).toBe(true);

		const recalled = (await client.recall({
			scope: {
				tenantId: null,
				sessionIds: ['homepage-demo'],
				tiers: null,
				fromUtc: null,
				toUtc: null
			},
			filter: {
				hasEmbedding: null,
				embeddingModel: null,
				psi: null,
				rho: null,
				kappa: null,
				textContains: null,
				tagsContains: null,
				hasTag: null,
				indexedTags: null,
				tagPrefix: null,
				hasSemanticLinks: null,
				linkRel: null,
				linkTarget: null,
				linksToRef: null
			},
			page: { limit: 5, cursor: null },
			scoring: {
				resonanceWeight: 1,
				semanticWeight: 0,
				lexicalWeight: 0,
				alpha: 0.7,
				beta: 0.3,
				gamma: 0,
				fallbackPolicy: 'on_empty',
				strictness: 'balanced'
			},
			currentAvec: null,
			queryText: 'What did we decide about the parser?',
			queryEmbedding: null,
			queryTagEmbedding: null
		})) as { retrievalPath: string; nodes: { raw: string }[] };

		expect(recalled.retrievalPath).toBe('lexical_fallback');
		expect(recalled.nodes.length).toBeGreaterThan(0);
		expect(recalled.nodes[0]?.raw).toContain('parser');
	});
});
