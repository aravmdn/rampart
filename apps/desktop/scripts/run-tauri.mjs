import { spawn } from "node:child_process";
import { delimiter, join } from "node:path";

const cargoBin = join(process.env.USERPROFILE ?? "", ".cargo", "bin");
const currentPath = process.env.PATH ?? "";
const pathEntries = currentPath.split(delimiter).filter(Boolean);

if (cargoBin && !pathEntries.includes(cargoBin)) {
  pathEntries.unshift(cargoBin);
}

const child = spawn(
  process.platform === "win32" ? "pnpm.cmd" : "pnpm",
  ["exec", "tauri", ...process.argv.slice(2)],
  {
    stdio: "inherit",
    shell: process.platform === "win32",
    env: {
      ...process.env,
      PATH: pathEntries.join(delimiter),
    },
  },
);

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 0);
});
