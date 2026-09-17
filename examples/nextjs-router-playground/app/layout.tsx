import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Next.js Router Playground",
  description: "A minimal Next.js App Router test target for Web Analytics Platform",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
