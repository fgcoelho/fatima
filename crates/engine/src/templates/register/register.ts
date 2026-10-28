declare const process: { env: Record<string, string | undefined> };
declare function readFileSync(path: string, encoding: "utf8"): string;

export type FatimaRegisterAsyncSource = string | string[] | Record<string, string | undefined> | Promise<Record<string, string | undefined>> | (() => Record<string, string | undefined> | Promise<Record<string, string | undefined>>);

function parseEnvLines(lines: string): Record<string, string> {
	const env: Record<string, string> = {};
	for (const line of lines.replace(/\r\n?/gm, "\n").split("\n")) {
		const match = /^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)?$/.exec(line);
		if (!match) continue;
		const [, key, rawValue = ""] = match;
		let value = rawValue.trim().replace(/^(['\"`])([\s\S]*)\1$/gm, "$2");
		if (rawValue[0] === '"') value = value.replace(/\\n/g, "\n").replace(/\\r/g, "\r");
		env[key] = value;
	}
	return env;
}

function populateEnv(env: Record<string, string | undefined>) {
	for (const [key, value] of Object.entries(env)) if (typeof value === "string") process.env[key] = value;
	__CACHE_NAME__ = undefined;
}

export function register(...files: string[]) {
	const env: Record<string, string> = {};
	for (const file of files.length ? files : [".env"]) Object.assign(env, parseEnvLines(readFileSync(file, "utf8")));
	populateEnv(env);
	return env;
}

export async function registerAsync(source: FatimaRegisterAsyncSource = [".env"]) {
	const env = typeof source === "function" ? await source() : await source;
	populateEnv(env as Record<string, string | undefined>);
	return env;
}
