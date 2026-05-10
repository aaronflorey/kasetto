import { FaGithub, FaLinkedinIn } from "react-icons/fa";
import { GoStar } from "react-icons/go";
import { AgentsGrid } from "./components/agents-grid";
import { CopyButton } from "./components/copy-button";
import { ConfigExample, FeatureList } from "./components/feature-tabs";

const INSTALL = [
  {
    label: "MACOS / LINUX",
    cmd: "curl -fsSL kasetto.dev/install | sh",
  },
  {
    label: "WINDOWS (POWERSHELL)",
    cmd: 'powershell -ExecutionPolicy Bypass -c "irm kasetto.dev/install.ps1 | iex"',
  },
  {
    label: "HOMEBREW",
    cmd: "brew install pivoshenko/tap/kasetto",
  },
  {
    label: "CARGO",
    cmd: "cargo install kasetto",
  },
];

async function getRepoData(): Promise<{ stars: string | null; version: string | null }> {
  try {
    const [repoRes, releaseRes] = await Promise.all([
      fetch("https://api.github.com/repos/pivoshenko/kasetto", {
        next: { revalidate: 3600 },
      }),
      fetch("https://api.github.com/repos/pivoshenko/kasetto/releases/latest", {
        next: { revalidate: 3600 },
      }),
    ]);
    const repo = repoRes.ok ? ((await repoRes.json()) as { stargazers_count: number }) : null;
    const release = releaseRes.ok ? ((await releaseRes.json()) as { tag_name: string }) : null;
    const n = repo?.stargazers_count ?? 0;
    return {
      stars: repo ? (n >= 1000 ? `${(n / 1000).toFixed(1)}K` : String(n)) : null,
      version: release?.tag_name ?? null,
    };
  } catch {
    return { stars: null, version: null };
  }
}

function Track({
  num,
  title,
  children,
}: {
  num: string;
  title: string;
  children: React.ReactNode;
}) {
  const side = num.charAt(0);
  return (
    <section className="track" data-side={side}>
      <div className="track-marker">
        <span className="track-num">{num}</span>
        <span className="track-bar" />
        <span className="track-title">{title}</span>
      </div>
      <div className="track-body">{children}</div>
    </section>
  );
}

function SideBanner({ side, label }: { side: "A" | "B"; label: string }) {
  return (
    <div className="tape-side" data-side={side}>
      <span className="tape-side-tag">SIDE {side}</span>
      <span className="tape-side-line" />
      <span className="tape-side-label">{label}</span>
    </div>
  );
}

export default async function Page() {
  const { stars, version } = await getRepoData();

  return (
    <div className="page-wrap">
      {/* ── Cassette label ── */}
      <div className="logo-wrap">
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img src="/logo.svg" alt="Kasetto" className="logo-img" />
      </div>

      <div className="cassette-label">
        <p className="hero-tagline">DECLARATIVE AI AGENT ENVIRONMENT MANAGER</p>
        <p className="hero-etymology">
          <span className="hero-etymology-jp">カセット</span> — cassette. plug in, swap out, share.
        </p>
      </div>

      {/* ─────── SIDE A ─────── */}
      <SideBanner side="A" label="GET STARTED · FEATURES · EXAMPLE" />

      <Track num="A1" title="QUICKSTART">
        <div className="grid-box">
          <div className="action-row">
            <span className="action-label">INSTALL</span>
            <div className="install-right">
              <code className="install-cmd">curl -fsSL kasetto.dev/install | sh</code>
              <CopyButton text="curl -fsSL kasetto.dev/install | sh" />
            </div>
          </div>
          <div className="action-row">
            <span className="action-label">
              GITHUB
              {stars && (
                <span className="accent-warm star-count">
                  <GoStar aria-hidden="true" />
                  {stars}
                </span>
              )}
              {version && <span className="version-badge">{version}</span>}
            </span>
            <a
              href="https://github.com/pivoshenko/kasetto"
              className="action-link"
              target="_blank"
              rel="noopener noreferrer"
            >
              github.com/pivoshenko/kasetto <span className="arrow">↗</span>
            </a>
          </div>
          <div className="action-row">
            <span className="action-label">DOCS</span>
            <a href="/docs" className="action-link">
              kasetto.dev/docs <span className="arrow">↗</span>
            </a>
          </div>
        </div>
      </Track>

      <Track num="A2" title="FEATURES">
        <FeatureList />
      </Track>

      <Track num="A3" title="EXAMPLE">
        <ConfigExample />
      </Track>

      {/* ─────── SIDE B ─────── */}
      <SideBanner side="B" label="AGENTS · INSTALL" />

      <Track num="B1" title="SUPPORTED AGENTS">
        <AgentsGrid />
      </Track>

      <Track num="B2" title="INSTALL">
        <div className="grid-box">
          {INSTALL.map((m) => (
            <div key={m.label} className="install-row">
              <span className="install-label">{m.label}</span>
              <div className="install-right">
                <code className="install-cmd">{m.cmd}</code>
                <CopyButton text={m.cmd} />
              </div>
            </div>
          ))}
        </div>
      </Track>

      {/* ── Footer ── */}
      <footer className="footer">
        <span className="footer-left">
          2026 Volodymyr Pivoshenko &lt;contact@pivoshenko.dev&gt;
        </span>
        <div className="footer-links">
          <a
            href="https://github.com/pivoshenko"
            className="footer-icon"
            target="_blank"
            rel="noopener noreferrer"
            aria-label="GitHub"
          >
            <FaGithub />
          </a>
          <a
            href="https://linkedin.com/in/pivoshenko"
            className="footer-icon"
            target="_blank"
            rel="noopener noreferrer"
            aria-label="LinkedIn"
          >
            <FaLinkedinIn />
          </a>
        </div>
      </footer>
    </div>
  );
}
