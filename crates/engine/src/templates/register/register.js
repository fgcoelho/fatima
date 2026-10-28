function parseEnvLines(lines) {
	const env = {};
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

function populateEnv(env) {
	for (const [key, value] of Object.entries(env)) if (typeof value === "string") process.env[key] = value;
	__CACHE_NAME__ = undefined;
}

function register(...files) {
	const { readFileSync } = require("node:fs");
	const env = {};
	for (const file of files.length ? files : [".env"]) Object.assign(env, parseEnvLines(readFileSync(file, "utf8")));
	populateEnv(env);
	return env;
}

async function registerAsync(source = [".env"]) {
	const env = typeof source === "function" ? await source() : await source;
	populateEnv(env);
	return env;
}
