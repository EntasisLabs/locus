import wasmInit, {
	initSync,
	WasmLocusClient,
	compress_text,
	connect_indxdb_client,
	connect_mem_surreal_client,
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
	connect_indxdb_client,
	connect_mem_surreal_client,
	connect_surreal_client,
	memory_schema,
	parse_sttp,
	validate_sttp,
	version
};

export type ParseProfile = 'tolerant' | 'strict' | 'strictTypedIr';

/** SurrealDB connection config. Use `indxdb://name`, `mem://`, or `ws(s)://` endpoints. */
export interface SurrealConnectConfig {
	endpoint: string;
	namespace: string;
	database: string;
	useRemote?: boolean;
	user?: string;
	password?: string;
}

export function surrealIndxdbConfig(
	indexedDbName: string,
	namespace: string,
	database: string
): SurrealConnectConfig {
	return {
		endpoint: `indxdb://${indexedDbName}`,
		namespace,
		database,
		useRemote: false
	};
}

export function surrealMemConfig(namespace: string, database: string): SurrealConnectConfig {
	return {
		endpoint: 'mem://',
		namespace,
		database,
		useRemote: false
	};
}

export function surrealWebsocketConfig(
	endpoint: string,
	namespace: string,
	database: string
): SurrealConnectConfig {
	return {
		endpoint,
		namespace,
		database,
		useRemote: true
	};
}
