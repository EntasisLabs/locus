<script lang="ts">
	import { onMount } from 'svelte';
	import { compile_note, initLocusWasm, WasmLocusClient } from '$lib/locus-wasm';

	const SESSION_ID = 'homepage-demo';

	type CompiledNote = {
		canonical: string;
		sessionId: string;
		contextSummary: string;
		anchorTopic: string;
		anchorTerms: string[];
		keyPoints: string[];
	};

	type StoredNote = CompiledNote & {
		nodeId: string;
	};

	type MemoryNode = {
		raw: string;
		contextSummary?: string | null;
		timestamp: string;
	};

	type RecallResponse = {
		nodes: MemoryNode[];
		retrievalPath: string;
	};

	type StoreResponse = {
		nodeId: string;
		valid: boolean;
		validationError?: string | null;
	};

	const samples = [
		'We decided the parser should accept both strict and tolerant STTP.',
		'The homepage demo stays in this browser. Nothing is uploaded.',
		'Monthly summaries keep the scores even after the writeup gets shorter.'
	];

	const sampleQuestions = [
		'What did we decide about the parser?',
		'Does this demo upload anything?'
	];

	let note = $state('');
	let question = $state('');
	let status = $state('Loading the memory module…');
	let error = $state('');
	let busy = $state(false);
	let ready = $state(false);
	let notes = $state<StoredNote[]>([]);
	let matches = $state<MemoryNode[]>([]);
	let asked = $state(false);
	let retrievalPath = $state('');
	let selectedRaw = $state('');

	let client: WasmLocusClient | null = null;

	onMount(() => {
		let cancelled = false;
		void (async () => {
			try {
				await initLocusWasm();
				if (cancelled) return;
				client = new WasmLocusClient();
				ready = true;
				status = 'In this browser only. Refreshing the page clears it.';
			} catch (err) {
				error = err instanceof Error ? err.message : 'The memory module failed to load.';
				status = '';
			}
		})();
		return () => {
			cancelled = true;
		};
	});

	function recallRequest(queryText: string) {
		return {
			scope: {
				tenantId: null,
				sessionIds: [SESSION_ID],
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
			queryText,
			queryEmbedding: null,
			queryTagEmbedding: null
		};
	}

	function noteFor(raw: string): StoredNote | undefined {
		return notes.find((item) => item.canonical === raw);
	}

	function formatWhen(timestamp: string): string {
		const date = new Date(timestamp);
		if (Number.isNaN(date.getTime())) return timestamp;
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: 'numeric',
			minute: '2-digit'
		});
	}

	async function saveNote(text: string) {
		const trimmed = text.trim();
		if (!client || !trimmed || busy) return;
		busy = true;
		error = '';
		try {
			const compiled = compile_note(trimmed, SESSION_ID) as CompiledNote;
			const stored = (await client.store(compiled.canonical, SESSION_ID)) as StoreResponse;
			if (!stored.valid) {
				error = stored.validationError || 'That note could not be stored.';
				return;
			}
			notes = [{ ...compiled, nodeId: stored.nodeId }, ...notes];
			selectedRaw = compiled.canonical;
			note = '';
			status = 'Saved. The record on the right is what was written.';
		} catch (err) {
			error = err instanceof Error ? err.message : 'That note could not be compiled.';
		} finally {
			busy = false;
		}
	}

	async function ask(text = question) {
		const trimmed = text.trim();
		if (!client || !trimmed || busy) return;
		question = trimmed;
		if (notes.length === 0) {
			error = 'Save a note first. Recall only searches what is stored here.';
			return;
		}
		busy = true;
		error = '';
		try {
			const result = (await client.recall(recallRequest(trimmed))) as RecallResponse;
			asked = true;
			retrievalPath = result.retrievalPath;
			matches = result.retrievalPath === 'lexical_fallback' ? result.nodes : [];
			if (matches[0]) selectedRaw = matches[0].raw;
			status =
				result.retrievalPath === 'lexical_fallback'
					? 'These are matching memories, not a written answer.'
					: 'Use at least two words from the notes. Words like what, the, and did are skipped.';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Recall failed.';
		} finally {
			busy = false;
		}
	}
</script>

<section class="ss try" id="try">
	<div class="wrap">
		<div class="sr center-copy">
			<span class="ml">Try it</span>
			<h2 class="dh">Save a note. Ask for it back.</h2>
			<p class="bp2">
				This runs in your browser. Locus compiles the note into an STTP record, keeps it in memory,
				and returns the memories that match your question. It does not write an answer. The same
				notes and the same question return the same memories. Nothing is uploaded.
			</p>
			<p class="try-status" role="status">{status}</p>
		</div>

		<div class="try-grid sr">
			<div class="try-panel">
				<label for="try-note">A note</label>
				<textarea
					id="try-note"
					rows="4"
					maxlength="4000"
					bind:value={note}
					placeholder="Write something you want to find later."
					disabled={!ready || busy}
				></textarea>
				<div class="try-chips" aria-label="Sample notes">
					{#each samples as sample (sample)}
						<button
							type="button"
							class="chip"
							disabled={!ready || busy}
							onclick={() => saveNote(sample)}
						>
							{sample}
						</button>
					{/each}
				</div>
				<button
					type="button"
					class="btn bp"
					disabled={!ready || busy || !note.trim()}
					onclick={() => saveNote(note)}
				>
					Save note
				</button>

				<label for="try-question">A question</label>
				<input
					id="try-question"
					type="text"
					bind:value={question}
					placeholder="What should it look up?"
					disabled={!ready || busy}
					onkeydown={(event) => {
						if (event.key === 'Enter') void ask();
					}}
				/>
				<div class="try-chips" aria-label="Sample questions">
					{#each sampleQuestions as sample (sample)}
						<button
							type="button"
							class="chip"
							disabled={!ready || busy}
							onclick={() => ask(sample)}
						>
							{sample}
						</button>
					{/each}
				</div>
				<button
					type="button"
					class="btn bg"
					disabled={!ready || busy || !question.trim()}
					onclick={() => ask()}
				>
					Find memories
				</button>
				{#if error}
					<p class="try-error" role="alert">{error}</p>
				{/if}
			</div>

			<div class="try-panel">
				<h3>Memories</h3>
				{#if notes.length === 0}
					<p class="try-empty">Nothing stored yet.</p>
				{:else}
					<ul class="try-list">
						{#each notes as item (item.nodeId)}
							<li>
								<button
									type="button"
									class:selected={selectedRaw === item.canonical}
									onclick={() => (selectedRaw = item.canonical)}
								>
									<strong>{item.contextSummary}</strong>
									<span>{item.anchorTerms.join(' · ') || 'note'}</span>
								</button>
							</li>
						{/each}
					</ul>
				{/if}

				<h3>Matches</h3>
				{#if !asked}
					<p class="try-empty">Ask after you have saved a note.</p>
				{:else if retrievalPath !== 'lexical_fallback'}
					<p class="try-empty">{status}</p>
				{:else if matches.length === 0}
					<p class="try-empty">No memories matched that question.</p>
				{:else}
					<ul class="try-list">
						{#each matches as item, index (`${item.timestamp}-${index}`)}
							<li>
								<button
									type="button"
									class:selected={selectedRaw === item.raw}
									onclick={() => (selectedRaw = item.raw)}
								>
									<strong
										>{item.contextSummary || noteFor(item.raw)?.contextSummary || 'Memory'}</strong
									>
									<span>{formatWhen(item.timestamp)}</span>
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>

		{#if selectedRaw}
			<div class="try-record sr">
				<h3>The record that was written</h3>
				{#if noteFor(selectedRaw)}
					<p>{noteFor(selectedRaw)?.contextSummary}</p>
				{/if}
				<pre>{selectedRaw}</pre>
			</div>
		{/if}
	</div>
</section>

<style>
	.try {
		padding: 120px 0 140px;
	}

	.try-status,
	.try-empty,
	.try-error {
		font-family: var(--fm);
		font-size: 13px;
		line-height: 1.6;
	}

	.try-status {
		color: var(--text-faint);
		margin: 22px auto 0;
	}

	.try-error {
		color: #e2a2b4;
		margin: 16px 0 0;
	}

	.try-grid {
		display: grid;
		grid-template-columns: 1.1fr 0.9fr;
		gap: 28px;
		margin-top: 48px;
		text-align: left;
	}

	.try-panel,
	.try-record {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--mist2);
		border-radius: 4px;
		padding: 28px;
	}

	.try-record {
		margin-top: 28px;
	}

	label,
	h3 {
		display: block;
		font-family: var(--fm);
		font-size: 11px;
		letter-spacing: 0.18em;
		text-transform: uppercase;
		color: var(--teal);
		margin: 22px 0 10px;
	}

	.try-panel label:first-child,
	.try-panel h3:first-child,
	.try-record h3 {
		margin-top: 0;
	}

	textarea,
	input {
		width: 100%;
		box-sizing: border-box;
		background: rgba(0, 0, 0, 0.28);
		color: var(--star);
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: 2px;
		padding: 14px 16px;
		font-family: var(--fu);
		font-size: 16px;
		line-height: 1.5;
	}

	textarea {
		resize: vertical;
		min-height: 112px;
	}

	.try-chips {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin: 12px 0 16px;
	}

	.chip,
	.try-list button {
		text-align: left;
		background: transparent;
		color: var(--text-dim);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 2px;
		padding: 10px 12px;
		font-family: var(--fu);
		font-size: 14px;
		line-height: 1.45;
		cursor: pointer;
	}

	.chip:hover,
	.try-list button:hover,
	.try-list button.selected {
		color: var(--star);
		border-color: rgba(77, 191, 160, 0.55);
	}

	button:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.btn {
		font-family: var(--fu);
		font-size: 12px;
		font-weight: 600;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		padding: 16px 30px;
		border-radius: 2px;
		cursor: pointer;
	}

	.bp {
		background: var(--purple);
		color: #fff;
		border: none;
	}

	.bg {
		background: transparent;
		color: var(--text-dim);
		border: 1px solid rgba(255, 255, 255, 0.18);
	}

	.try-panel .btn {
		margin-top: 4px;
	}

	.try-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.try-list button {
		width: 100%;
	}

	.try-list strong,
	.try-list span {
		display: block;
	}

	.try-list strong {
		color: var(--star);
		font-weight: 600;
	}

	.try-list span,
	.try-empty {
		color: var(--text-faint);
	}

	.try-record p {
		color: var(--text-dim);
		margin: 0 0 14px;
	}

	pre {
		margin: 0;
		white-space: pre-wrap;
		word-break: break-word;
		font-family: var(--fm);
		font-size: 12px;
		line-height: 1.65;
		color: rgba(255, 255, 255, 0.72);
	}

	@media (max-width: 980px) {
		.try-grid {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 760px) {
		.try {
			padding: 84px 0 96px;
		}

		.try-panel,
		.try-record {
			padding: 20px 16px;
		}
	}
</style>
