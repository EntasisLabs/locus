<script lang="ts">
	import { onMount } from 'svelte';
	import { compile_note, initLocusWasm, WasmLocusClient } from '$lib/locus-wasm';

	const SESSION_ID = 'homepage-demo';

	const STOPWORDS = new Set([
		'a',
		'an',
		'the',
		'and',
		'or',
		'but',
		'if',
		'then',
		'so',
		'of',
		'to',
		'for',
		'in',
		'on',
		'at',
		'from',
		'with',
		'by',
		'as',
		'into',
		'over',
		'under',
		'about',
		'what',
		'which',
		'who',
		'whom',
		'whose',
		'when',
		'where',
		'why',
		'how',
		'did',
		'do',
		'does',
		'is',
		'are',
		'was',
		'were',
		'be',
		'been',
		'being',
		'am',
		'we',
		'i',
		'you',
		'he',
		'she',
		'they',
		'it',
		'me',
		'my',
		'our',
		'your',
		'their',
		'them',
		'us',
		'this',
		'that',
		'these',
		'those',
		'there',
		'here',
		'please',
		'tell',
		'just',
		'any',
		'some',
		'not',
		'remember',
		'recall',
		'know'
	]);

	const LAYER_MARKS = ['⊕⟨', '⦿⟨', '◈⟨', '⍉⟨'] as const;
	const LAYER_LABELS = [
		'Where it came from',
		'When and how it was stored',
		'What it says',
		'How confident it is'
	];

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
		text: string;
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

	type TextPart = {
		text: string;
		bold: boolean;
	};

	const samples = [
		'We decided the parser should accept both strict and tolerant modes.',
		'The homepage demo stays in this browser. Nothing is uploaded.',
		'Monthly summaries keep the scores even after the writeup gets shorter.'
	];

	const sampleQuestions = [
		'What did we decide about the parser?',
		'Does this demo upload anything?'
	];

	const SHORT_QUERY =
		'Use at least two words from the notes. Words like what, the, and did are skipped.';
	const NO_MATCH = 'No memories matched that question.';

	let note = $state('');
	let question = $state('');
	let status = $state('');
	let resultMessage = $state('');
	let error = $state('');
	let busy = $state(false);
	let busyAction = $state<'save' | 'search' | ''>('');
	let ready = $state(false);
	let notes = $state<StoredNote[]>([]);
	let matches = $state<MemoryNode[]>([]);
	let asked = $state(false);
	let selectedRaw = $state('');
	let showRaw = $state(false);

	let client: WasmLocusClient | null = null;

	onMount(() => {
		let cancelled = false;
		void (async () => {
			try {
				await initLocusWasm();
				if (cancelled) return;
				client = new WasmLocusClient();
				ready = true;
			} catch (err) {
				error = err instanceof Error ? err.message : 'The memory module failed to load.';
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

	function meaningfulWords(query: string): string[] {
		const words: string[] = [];
		for (const raw of query.toLowerCase().split(/[^a-z0-9_-]+/)) {
			const token = raw.replace(/^[-_]+|[-_]+$/g, '');
			if (token.length < 2 || STOPWORDS.has(token) || words.includes(token)) continue;
			words.push(token);
		}
		return words;
	}

	function noteTitle(text: string): string {
		const trimmed = text.trim();
		const sentence = trimmed.split(/(?<=[.!?])\s+/)[0] || trimmed;
		if (sentence.length <= 60) return sentence;
		const cut = sentence.slice(0, 60);
		const space = cut.lastIndexOf(' ');
		const base = (space > 40 ? cut.slice(0, space) : cut).trimEnd().replace(/[.,;:]+$/, '');
		return `${base}…`;
	}

	function noteFor(raw: string): StoredNote | undefined {
		return notes.find((item) => item.canonical === raw);
	}

	function isMatch(item: StoredNote): boolean {
		return matches.some((match) => match.raw === item.canonical);
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

	function snippetSource(text: string, terms: string[]): string {
		const sentences = text
			.trim()
			.split(/(?<=[.!?])\s+/)
			.filter(Boolean);
		const ranked = sentences
			.map((sentence) => ({
				sentence,
				hits: terms.filter((term) => sentence.toLowerCase().includes(term)).length
			}))
			.sort((left, right) => right.hits - left.hits);
		const chosen = ranked[0]?.sentence || text.trim();
		if (chosen.length <= 140) return chosen;
		const cut = chosen.slice(0, 140);
		const space = cut.lastIndexOf(' ');
		return `${(space > 80 ? cut.slice(0, space) : cut).trimEnd()}…`;
	}

	function snippetParts(text: string, terms: string[]): TextPart[] {
		const source = snippetSource(text, terms);
		if (terms.length === 0) return [{ text: source, bold: false }];
		const pattern = new RegExp(
			terms
				.slice()
				.sort((left, right) => right.length - left.length)
				.map((term) => `${term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}[a-z0-9]*`)
				.join('|'),
			'ig'
		);
		const parts: TextPart[] = [];
		let last = 0;
		for (const match of source.matchAll(pattern)) {
			const index = match.index ?? 0;
			if (index > last) parts.push({ text: source.slice(last, index), bold: false });
			parts.push({ text: match[0], bold: true });
			last = index + match[0].length;
		}
		if (last < source.length) parts.push({ text: source.slice(last), bold: false });
		return parts.length > 0 ? parts : [{ text: source, bold: false }];
	}

	function layersOf(raw: string): { label: string; text: string }[] {
		const indexes = LAYER_MARKS.map((mark) => raw.indexOf(mark));
		return LAYER_MARKS.flatMap((mark, index) => {
			const start = indexes[index];
			if (start < 0) return [];
			const later = indexes.filter(
				(position, laterIndex) => laterIndex > index && position > start
			);
			const end = later.length > 0 ? Math.min(...later) : raw.length;
			return [{ label: LAYER_LABELS[index], text: raw.slice(start, end).trimEnd() }];
		});
	}

	function clearDemo() {
		if (!ready || busy) return;
		client = new WasmLocusClient();
		notes = [];
		matches = [];
		asked = false;
		selectedRaw = '';
		showRaw = false;
		status = '';
		resultMessage = '';
		error = '';
		note = '';
		question = '';
	}

	async function saveNote(text: string) {
		const trimmed = text.trim();
		if (!client || !trimmed || busy) return;
		busy = true;
		busyAction = 'save';
		error = '';
		try {
			const compiled = compile_note(trimmed, SESSION_ID) as CompiledNote;
			const stored = (await client.store(compiled.canonical, SESSION_ID)) as StoreResponse;
			if (!stored.valid) {
				error = stored.validationError || 'That note could not be stored.';
				return;
			}
			notes = [{ ...compiled, nodeId: stored.nodeId, text: trimmed }, ...notes];
			selectedRaw = compiled.canonical;
			showRaw = false;
			note = '';
			status = 'Saved.';
		} catch (err) {
			error = err instanceof Error ? err.message : 'That note could not be compiled.';
		} finally {
			busy = false;
			busyAction = '';
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
		busyAction = 'search';
		error = '';
		try {
			const result = (await client.recall(recallRequest(trimmed))) as RecallResponse;
			asked = true;
			const terms = meaningfulWords(trimmed);
			if (terms.length < 2) {
				matches = [];
				resultMessage = SHORT_QUERY;
				status = SHORT_QUERY;
				return;
			}
			const found = result.retrievalPath === 'lexical_fallback' ? result.nodes : [];
			matches = found;
			if (found.length === 0) {
				resultMessage = NO_MATCH;
				status = NO_MATCH;
				return;
			}
			if (found[0]) selectedRaw = found[0].raw;
			resultMessage = '';
			status = '';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Recall failed.';
		} finally {
			busy = false;
			busyAction = '';
		}
	}

	let selected = $derived(noteFor(selectedRaw));
	let queryTerms = $derived(meaningfulWords(question));
</script>

<section class="ss try" id="try">
	<div class="wrap">
		<div class="sr center-copy">
			<span class="ml">Try it</span>
			<h2 class="dh">Save a note. Ask for it back.</h2>
			<p class="try-intro">
				Type a note, save it, then ask for it back.
			</p>
			<p class="try-privacy">In this browser only. Refreshing the page clears it.</p>
			{#if status}
				<p class="try-status" role="status">{status}</p>
			{/if}
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
							onclick={() => (note = sample)}
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
					{busyAction === 'save' ? 'Saving…' : 'Save note'}
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
					{busyAction === 'search' ? 'Searching…' : 'Find memories'}
				</button>
				{#if notes.length > 0 || asked}
					<button type="button" class="text-btn" disabled={busy} onclick={clearDemo}
						>Clear demo</button
					>
				{/if}
				{#if error}
					<p class="try-error" role="alert">{error}</p>
				{/if}
			</div>

			<div class="try-panel">
				{#if !ready && !error}
					<p class="try-empty">Loading…</p>
				{:else}
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
										class:hit={isMatch(item)}
										onclick={() => (selectedRaw = item.canonical)}
									>
										<span class="title">{noteTitle(item.text)}</span>
										{#if item.anchorTerms.length > 0}
											<span class="tags">{item.anchorTerms.join(' · ')}</span>
										{/if}
									</button>
								</li>
							{/each}
						</ul>
					{/if}

					<h3>Matches</h3>
					{#if !asked}
						<p class="try-empty">Ask after you have saved a note.</p>
					{:else if matches.length === 0}
						<p class="try-empty">{resultMessage}</p>
					{:else}
						<ul class="try-list">
							{#each matches as item, index (`${item.timestamp}-${index}`)}
								<li>
									<button
										type="button"
										class:selected={selectedRaw === item.raw}
										onclick={() => (selectedRaw = item.raw)}
									>
										<span class="title"
											>{noteTitle(noteFor(item.raw)?.text || item.contextSummary || 'Memory')}</span
										>
										<span class="snippet">
											{#each snippetParts(noteFor(item.raw)?.text || '', queryTerms) as part, partIndex (`${index}-${partIndex}`)}
												{#if part.bold}<strong>{part.text}</strong>{:else}{part.text}{/if}
											{/each}
										</span>
										<span class="when">{formatWhen(item.timestamp)}</span>
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				{/if}
			</div>
		</div>

		{#if selected}
			<div class="try-record sr">
				<p class="record-title">{noteTitle(selected.text)}</p>
				<p class="record-note">{selected.text}</p>
				{#if selected.anchorTerms.length > 0}
					<p class="tags">{selected.anchorTerms.join(' · ')}</p>
				{/if}
				<button
					type="button"
					class="text-btn"
					aria-expanded={showRaw}
					onclick={() => (showRaw = !showRaw)}
				>
					{showRaw ? 'Hide what Locus wrote' : 'See what Locus wrote'}
				</button>
				{#if showRaw}
					{#each layersOf(selected.canonical) as layer (layer.label)}
						<p class="layer-label">{layer.label}</p>
						<pre>{layer.text}</pre>
					{/each}
				{/if}
			</div>
		{/if}
	</div>
</section>

<style>
	.try {
		padding: 120px 0 140px;
		scroll-margin-top: 92px;
	}

	.try-intro {
		font-size: 17px;
		color: var(--text-dim);
		line-height: 1.82;
		max-width: 640px;
		margin: 0 auto;
	}

	.try-privacy,
	.try-status,
	.try-empty,
	.try-error {
		font-family: var(--fm);
		font-size: 13px;
		line-height: 1.6;
	}

	.try-privacy,
	.try-status {
		color: var(--text-faint);
		margin: 18px auto 0;
	}

	.try-status {
		margin-top: 8px;
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
		min-width: 0;
	}

	.try-panel,
	.try-record {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--mist2);
		border-radius: 4px;
		padding: 28px;
		min-width: 0;
	}

	.try-record {
		margin-top: 28px;
	}

	label,
	h3,
	.layer-label {
		display: block;
		font-family: var(--fm);
		font-size: 11px;
		letter-spacing: 0.18em;
		text-transform: uppercase;
		color: var(--teal);
		margin: 22px 0 10px;
	}

	.try-panel label:first-child,
	.try-panel h3:first-child {
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
		max-width: 100%;
	}

	.chip:hover,
	.try-list button:hover,
	.try-list button.selected,
	.try-list button.hit {
		color: var(--star);
		border-color: rgba(77, 191, 160, 0.55);
	}

	textarea:focus-visible,
	input:focus-visible,
	.chip:focus-visible,
	.btn:focus-visible,
	.try-list button:focus-visible,
	.text-btn:focus-visible {
		outline: 2px solid var(--teal, #4dbfa0);
		outline-offset: 3px;
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

	.text-btn {
		display: inline-block;
		margin-top: 16px;
		padding: 0;
		background: none;
		border: none;
		color: var(--text-faint);
		font-family: var(--fu);
		font-size: 13px;
		line-height: 1.4;
		text-decoration: underline;
		text-underline-offset: 3px;
		cursor: pointer;
	}

	.text-btn:hover {
		color: var(--star);
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

	.title,
	.tags,
	.snippet,
	.when {
		display: block;
	}

	.title {
		color: var(--star);
		font-weight: 600;
	}

	.tags,
	.when,
	.try-empty {
		color: var(--text-faint);
	}

	.snippet {
		margin-top: 6px;
		color: var(--text-dim);
	}

	.snippet strong {
		color: var(--star);
		font-weight: 700;
	}

	.record-title {
		margin: 0 0 12px;
		color: var(--star);
		font-size: 18px;
		line-height: 1.4;
		font-weight: 600;
	}

	.record-note {
		margin: 0 0 12px;
		color: var(--text-dim);
	}

	.try-record .tags {
		margin: 0;
	}

	pre {
		margin: 0;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
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
			scroll-margin-top: 80px;
		}

		.try-panel,
		.try-record {
			padding: 20px 16px;
		}
	}
</style>
