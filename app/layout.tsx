import type { Metadata } from "next";
import "./styles.css";

export const metadata: Metadata = {
  title: "SolSwap — Exchange Interface Showcase",
  description: "A local decentralized exchange interface concept with illustrative quotes.",
  robots: { index: false, follow: false },
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
