import { FaGithub } from "react-icons/fa";

export function TopNav() {
  return (
    <nav className="top-nav">
      <div className="top-nav-inner">
        <a href="/" className="top-nav-brand" aria-label="Kasetto home">
          <span className="top-nav-name">KASETTO</span>
        </a>
        <div className="top-nav-links">
          <a href="/docs" className="top-nav-link">
            DOCS
          </a>
          <a
            href="https://github.com/pivoshenko/kasetto"
            className="top-nav-link"
            target="_blank"
            rel="noopener noreferrer"
            aria-label="GitHub"
          >
            <FaGithub />
          </a>
        </div>
      </div>
    </nav>
  );
}
