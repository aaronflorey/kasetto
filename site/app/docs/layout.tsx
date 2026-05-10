import { source } from "@/lib/source";
import { DocsLayout } from "fumadocs-ui/layouts/docs";
import { RootProvider } from "fumadocs-ui/provider";
import type { ReactNode } from "react";

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <RootProvider theme={{ enabled: false, defaultTheme: "dark" }}>
      <DocsLayout tree={source.pageTree} nav={{ enabled: false }}>
        {children}
      </DocsLayout>
    </RootProvider>
  );
}
