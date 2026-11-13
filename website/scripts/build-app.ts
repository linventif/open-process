import { mkdir, stat } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const website = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const tools = join(website, ".tools");
const env = { ...process.env };
const trunkVersion = "0.22.1";

async function run(command: string[], cwd = website) {
  const process = Bun.spawn(command, { cwd, env, stdin: "inherit", stdout: "inherit", stderr: "inherit" });
  if (await process.exited !== 0) throw new Error(`Build command failed: ${command.join(" ")}`);
}

async function download(url: string, path: string) {
  console.log(`Downloading build tool from ${new URL(url).hostname}…`);
  await run(["curl", "--fail", "--location", "--silent", "--show-error", "--retry", "3",
    "--connect-timeout", "10", "--max-time", "120", "--output", path, url]);
}

await mkdir(tools, { recursive: true });
let rustup = Bun.which("rustup");
if (!rustup) {
  // Cloudflare's Linux build image may not include Rust. Keep bootstrapped tools
  // in the ignored project directory and leave the user's shell profile alone.
  env.CARGO_HOME = join(tools, "cargo");
  env.RUSTUP_HOME = join(tools, "rustup");
  env.PATH = `${join(env.CARGO_HOME, "bin")}:${env.PATH ?? ""}`;
  rustup = join(env.CARGO_HOME, "bin/rustup");
  if (!(await stat(rustup).catch(() => null))) {
    const installer = join(tools, "rustup-init.sh");
    await download("https://sh.rustup.rs", installer);
    await run(["sh", installer, "-y", "--profile", "minimal", "--no-modify-path"]);
  }
}
await run([rustup, "target", "add", "wasm32-unknown-unknown"]);

let trunk = Bun.which("trunk");
if (!trunk) {
  trunk = join(tools, "trunk");
  if (!(await stat(trunk).catch(() => null))) {
    if (process.platform !== "linux" || process.arch !== "x64") {
      throw new Error(`Install Trunk ${trunkVersion} before building on this platform: cargo install --locked trunk --version ${trunkVersion}`);
    }
    const archive = join(tools, "trunk.tar.gz");
    await download(`https://github.com/trunk-rs/trunk/releases/download/v${trunkVersion}/trunk-x86_64-unknown-linux-musl.tar.gz`, archive);
    const digest = new Bun.CryptoHasher("sha256").update(await Bun.file(archive).arrayBuffer()).digest("hex");
    if (digest !== "a1fe5cd6b053f5a76f66b598d296022a7f06f56def6013cc16cd6897d53f5363") {
      throw new Error("The downloaded Trunk archive does not match the pinned release checksum.");
    }
    await run(["tar", "-xzf", archive, "-C", tools, "trunk"]);
  }
}

await run([
  trunk, "build", "--release", "--locked", "--public-url", "/app/",
  "--dist", join(website, "dist/app"),
], resolve(website, "../crates/op-app"));
console.log("Cloudflare Pages output ready: website/dist (vitrine /, application /app/).");
