import { Footer, Layout, Navbar } from "nextra-theme-docs";
import { Head } from "nextra/components";
import { getPageMap } from "nextra/page-map";
import type { Metadata } from "next";
import "nextra-theme-docs/style.css";

export const metadata: Metadata = {
  title: {
    default: "LadderMD",
    template: "%s - LadderMD",
  },
  description: "PLC ladder diagram to Markdown converter",
};

const navbar = (
  <Navbar
    logo={<b>LadderMD</b>}
    projectLink="https://github.com/PeterTakahashi/LadderMD"
  />
);

const footer = <Footer>MIT {new Date().getFullYear()} © LadderMD</Footer>;

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head />
      <body>
        <Layout
          navbar={navbar}
          footer={footer}
          pageMap={await getPageMap()}
          docsRepositoryBase="https://github.com/PeterTakahashi/LadderMD/tree/main/docs-site"
        >
          {children}
        </Layout>
      </body>
    </html>
  );
}
