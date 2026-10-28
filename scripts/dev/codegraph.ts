import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const ROOT = resolve(import.meta.dirname, "../..");

function pinnedPackage(config: unknown): string {
	const servers = Reflect.get(Object(config), "mcpServers");
	const args: unknown = Reflect.get(Object(servers), "codegraph")?.args;
	if (!Array.isArray(args)) return "";
	return (
		args.find(
			(arg): arg is string =>
				typeof arg === "string" && arg.startsWith("@colbymchenry/codegraph@"),
		) ?? ""
	);
}

if (
	process.env.CI ||
	!existsSync(join(ROOT, ".git")) ||
	existsSync(join(ROOT, ".codegraph"))
) {
	process.exit(0);
}

const pinned = pinnedPackage(
	JSON.parse(await readFile(join(ROOT, ".agents/.mcp.json"), "utf8")),
);

if (pinned === "") {
	console.error(
		".agents/.mcp.json pins no codegraph version for the MCP server.",
	);
	process.exit(1);
}

const { status } = spawnSync("npx", ["-y", pinned, "init", "-y"], {
	cwd: ROOT,
	stdio: "inherit",
});

process.exit(status ?? 1);
