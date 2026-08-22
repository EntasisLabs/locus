import wasmInit, {
	initSync,
	WasmLocusClient,
	compress_text,
	connect_surreal_client,
	memory_schema,
	parse_sttp,
	validate_sttp,
	version
} from './pkg/locus_wasm.js';

let initPromise: Promise<void> | null = null;

/** Load and initialize the Locus WASM module (idempotent). */
export async function initLocusWasm(): Promise<void> {
	if (!initPromise) {
		initPromise = wasmInit().then(() => undefined);
	}
	return initPromise;
}

export {
	initSync,
	WasmLocusClient,
	WasmLocusClient as LocusClient,
	compress_text,
	connect_surreal_client,
	memory_schema,
	parse_sttp,
	validate_sttp,
	version
};

export type ParseProfile = 'tolerant' | 'strict' | 'strictTypedIr';

export interface SurrealConnectConfig {
	endpoint: string;
	namespace: string;
	database: string;
	useRemote?: boolean;
	user?: string;
	password?: string;
}
