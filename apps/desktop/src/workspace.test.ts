import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const desktopRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(desktopRoot, "..", "..");

describe("desktop workspace skeleton", () => {
  it("defines tauri-backed desktop scripts", () => {
    const packageJson = JSON.parse(
      readFileSync(resolve(desktopRoot, "package.json"), "utf8"),
    ) as {
      scripts?: Record<string, string>;
      dependencies?: Record<string, string>;
      devDependencies?: Record<string, string>;
    };

    expect(packageJson.scripts?.dev).toContain("run-tauri.mjs dev");
    expect(packageJson.scripts?.build).toContain("run-tauri.mjs build");
    expect(packageJson.scripts?.["dev:web"]).toBe("vite");
    expect(packageJson.scripts?.["build:web"]).toContain("vite build");
    expect(packageJson.devDependencies?.["@tauri-apps/cli"]).toBeTruthy();
    expect(packageJson.dependencies?.["@tauri-apps/api"]).toBeTruthy();
  });

  it("includes src-tauri config and crate wiring", () => {
    expect(existsSync(resolve(desktopRoot, "src-tauri", "Cargo.toml"))).toBe(true);
    expect(existsSync(resolve(desktopRoot, "src-tauri", "src", "main.rs"))).toBe(true);
    expect(existsSync(resolve(desktopRoot, "src-tauri", "tauri.conf.json"))).toBe(true);

    const cargoToml = readFileSync(resolve(repoRoot, "Cargo.toml"), "utf8");
    expect(cargoToml).toContain('  "apps/desktop/src-tauri",');
  });

  it("includes headless CLI crate in workspace", () => {
    const cargoToml = readFileSync(resolve(repoRoot, "Cargo.toml"), "utf8");
    expect(cargoToml).toContain('"apps/cli"');

    const cliCargoToml = readFileSync(resolve(repoRoot, "apps", "cli", "Cargo.toml"), "utf8");
    expect(cliCargoToml).toContain('name = "rampart"');
    expect(cliCargoToml).toContain("rampartd");
  });
});
