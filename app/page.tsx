"use client";

import { useMemo, useState } from "react";

type Token = {
  symbol: string;
  name: string;
  mark: string;
  color: string;
  price: number;
  balance: number;
};
const tokens: Token[] = [
  { symbol: "SOL", name: "Solana", mark: "◎", color: "sol", price: 142.8, balance: 18.42 },
  { symbol: "USDC", name: "USD Coin", mark: "$", color: "usdc", price: 1, balance: 2450.0 },
  { symbol: "BONK", name: "Bonk", mark: "B", color: "bonk", price: 0.000021, balance: 8200000 },
  { symbol: "JUP", name: "Jupiter", mark: "J", color: "jup", price: 0.84, balance: 625.5 },
];
const pools = [
  ["SOL / USDC", "$4.82m", "1.24%", "6.8%"],
  ["JUP / USDC", "$1.26m", "0.82%", "8.1%"],
  ["SOL / JUP", "$842k", "0.64%", "5.2%"],
];

export default function SolSwapPage() {
  const [from, setFrom] = useState(tokens[0]);
  const [to, setTo] = useState(tokens[1]);
  const [amount, setAmount] = useState("");
  const [slippage, setSlippage] = useState("0.5");
  const [activeTab, setActiveTab] = useState("Swap");
  const [notice, setNotice] = useState("");
  const [favorite, setFavorite] = useState<string[]>(["SOL / USDC"]);
  const [poolAmount, setPoolAmount] = useState("");
  const output = useMemo(() => {
    const input = Number(amount);
    if (!input || !Number.isFinite(input)) return "0.00";
    const feeAdjusted = input * from.price * 0.997;
    const value = feeAdjusted / to.price;
    return value < 0.001
      ? value.toPrecision(4)
      : value.toLocaleString("en", { maximumFractionDigits: 5 });
  }, [amount, from, to]);
  const priceImpact =
    Number(amount) * from.price > 0
      ? Math.min((Number(amount) * from.price) / 85000, 4.9).toFixed(2)
      : "0.00";
  const switchTokens = () => {
    setFrom(to);
    setTo(from);
    setAmount("");
  };
  const chooseToken = (side: "from" | "to") => {
    const current = side === "from" ? from : to;
    const selection = window.prompt(
      `Choose token: ${tokens.map((token) => token.symbol).join(", ")}`,
      current.symbol,
    );
    const token = tokens.find(
      (item) => item.symbol.toLowerCase() === selection?.trim().toLowerCase(),
    );
    if (!token) return;
    if (side === "from") {
      if (token.symbol === to.symbol) setTo(current);
      setFrom(token);
    } else {
      if (token.symbol === from.symbol) setFrom(current);
      setTo(token);
    }
  };
  const toggleFavorite = (pair: string) =>
    setFavorite((current) =>
      current.includes(pair) ? current.filter((item) => item !== pair) : [...current, pair],
    );
  const continueSwap = () =>
    setNotice(
      "Swap is disabled. Quotes are illustrative; no wallet, RPC, or contract is connected.",
    );

  return (
    <main className="swap-app">
      <aside className="swap-sidebar">
        <a className="swap-brand" href="#swap">
          <span>◈</span> SOLSWAP <small>PREVIEW</small>
        </a>
        <div className="network-chip">
          <i /> Solana network <b>not connected</b>
        </div>
        <nav>
          <button
            className={activeTab === "Swap" ? "active" : ""}
            onClick={() => setActiveTab("Swap")}
          >
            ⇄ <span>Swap</span>
          </button>
          <button
            className={activeTab === "Pools" ? "active" : ""}
            onClick={() => setActiveTab("Pools")}
          >
            ▤ <span>Liquidity pools</span>
          </button>
          <button
            className={activeTab === "Activity" ? "active" : ""}
            onClick={() => setActiveTab("Activity")}
          >
            ◷ <span>Activity</span>
          </button>
        </nav>
        <div className="sidebar-market">
          <span className="eyebrow">SAMPLE MARKET</span>
          <div>
            <span>SOL</span>
            <b>$142.80</b>
            <i>+2.41%</i>
          </div>
          <div>
            <span>USDC</span>
            <b>$1.00</b>
            <i>+0.01%</i>
          </div>
          <div>
            <span>JUP</span>
            <b>$0.84</b>
            <i>−1.22%</i>
          </div>
          <small>Illustrative token values · not live</small>
        </div>
        <div className="sidebar-bottom">
          <button onClick={() => setNotice("Documentation is not linked in this local preview.")}>
            ⓘ Documentation
          </button>
          <span>NO WALLET CONNECTED</span>
        </div>
      </aside>
      <section className="swap-main" id="swap">
        <header className="swap-header">
          <div className="header-crumb">
            Exchange <span>/</span> <b>{activeTab}</b>
          </div>
          <div>
            <span className="demo-pill">LOCAL DEMO</span>
            <button
              className="wallet-button"
              onClick={() =>
                setNotice(
                  "Wallet connection is disabled. No account authorization has been requested.",
                )
              }
            >
              ◎ Connect wallet
            </button>
          </div>
        </header>
        <div className="swap-content">
          <div className="swap-title">
            <span className="eyebrow">SOLANA / TOKEN EXCHANGE CONCEPT</span>
            <h1>
              {activeTab === "Swap"
                ? "A clearer way to exchange."
                : activeTab === "Pools"
                  ? "Liquidity, in view."
                  : "Transaction activity."}
            </h1>
            <p>Illustrative interface for exploring token swaps and sample pools.</p>
          </div>
          <div className="swap-tabs">
            {["Swap", "Pools", "Activity"].map((tab) => (
              <button
                key={tab}
                className={activeTab === tab ? "tab-active" : ""}
                onClick={() => setActiveTab(tab)}
              >
                {tab}
              </button>
            ))}
            <button
              className="settings-button"
              onClick={() => setNotice("Advanced settings are a local interface preview.")}
            >
              ⚙
            </button>
          </div>
          {activeTab === "Swap" ? (
            <div className="swap-layout">
              <section className="swap-card">
                <div className="swap-card-head">
                  <strong>Swap tokens</strong>
                  <button
                    onClick={() =>
                      setNotice("Quote settings are illustrative; no aggregator is queried.")
                    }
                  >
                    ⚙
                  </button>
                </div>
                <div className="token-panel">
                  <div className="token-label">
                    <span>You pay</span>
                    <span>
                      Balance {from.balance.toLocaleString()}{" "}
                      <button onClick={() => setAmount(String(from.balance))}>MAX</button>
                    </span>
                  </div>
                  <div className="token-entry">
                    <input
                      inputMode="decimal"
                      value={amount}
                      onChange={(event) => setAmount(event.target.value.replace(/[^\d.]/g, ""))}
                      placeholder="0.00"
                      aria-label="Amount to swap"
                    />
                    <button className="token-select" onClick={() => chooseToken("from")}>
                      <span className={`token-mark ${from.color}`}>{from.mark}</span>
                      <b>{from.symbol}</b>
                      <i>⌄</i>
                    </button>
                  </div>
                  <small>
                    {amount && Number(amount)
                      ? `≈ $${(Number(amount) * from.price).toLocaleString("en", { maximumFractionDigits: 2 })}`
                      : "Enter an amount"}
                  </small>
                </div>
                <button
                  className="switch-button"
                  onClick={switchTokens}
                  aria-label="Switch token pair"
                >
                  ↕
                </button>
                <div className="token-panel receive-panel">
                  <div className="token-label">
                    <span>You receive</span>
                    <span>Balance {to.balance.toLocaleString()}</span>
                  </div>
                  <div className="token-entry">
                    <output>{output}</output>
                    <button className="token-select" onClick={() => chooseToken("to")}>
                      <span className={`token-mark ${to.color}`}>{to.mark}</span>
                      <b>{to.symbol}</b>
                      <i>⌄</i>
                    </button>
                  </div>
                  <small>Illustrative quote only</small>
                </div>
                <div className="swap-details">
                  <span>
                    Rate{" "}
                    <b>
                      1 {from.symbol} ≈{" "}
                      {(from.price / to.price).toLocaleString("en", { maximumFractionDigits: 5 })}{" "}
                      {to.symbol}
                    </b>
                  </span>
                  <span>
                    Network fee <b>~0.000005 SOL</b>
                  </span>
                  <span>
                    Price impact{" "}
                    <b className={Number(priceImpact) > 1 ? "impact-high" : ""}>{priceImpact}%</b>
                  </span>
                  <label>
                    Slippage tolerance{" "}
                    <select value={slippage} onChange={(event) => setSlippage(event.target.value)}>
                      <option value="0.1">0.1%</option>
                      <option value="0.5">0.5%</option>
                      <option value="1">1.0%</option>
                      <option value="custom">Custom</option>
                    </select>
                  </label>
                </div>
                <button className="swap-submit" onClick={continueSwap}>
                  {Number(amount) > 0 ? "Preview swap" : "Enter an amount"}
                </button>
                <div className="swap-disclaimer">
                  Simulated quote · No transaction will be signed or submitted
                </div>
              </section>
              <aside className="market-card">
                <header>
                  <div>
                    <span className="eyebrow">MARKET PULSE</span>
                    <h2>SOL / USDC</h2>
                  </div>
                  <span className="market-change">+2.41%</span>
                </header>
                <div className="market-price">
                  $142.80 <small>Illustrative</small>
                </div>
                <div className="chart-tabs">
                  <button className="chart-active">1H</button>
                  <button>1D</button>
                  <button>1W</button>
                  <button>1M</button>
                </div>
                <div className="fake-price-chart">
                  <svg
                    viewBox="0 0 400 120"
                    preserveAspectRatio="none"
                    aria-label="Illustrative price chart"
                  >
                    <defs>
                      <linearGradient id="area" x1="0" x2="0" y1="0" y2="1">
                        <stop offset="0" stopColor="#8c78d3" stopOpacity=".2" />
                        <stop offset="1" stopColor="#8c78d3" stopOpacity="0" />
                      </linearGradient>
                    </defs>
                    <path
                      d="M0 92 C22 80 30 94 48 71 S78 62 99 71 123 55 145 65 174 44 194 58 226 32 245 47 273 43 294 30 323 43 343 28 373 34 400 15 V120 H0Z"
                      fill="url(#area)"
                    />
                    <path
                      d="M0 92 C22 80 30 94 48 71 S78 62 99 71 123 55 145 65 174 44 194 58 226 32 245 47 273 43 294 30 323 43 343 28 373 34 400 15"
                      fill="none"
                      stroke="#806acb"
                      strokeWidth="2"
                    />
                  </svg>
                </div>
                <div className="market-labels">
                  <span>10:00</span>
                  <span>12:00</span>
                  <span>14:00</span>
                  <span>16:00</span>
                </div>
                <div className="market-stat">
                  <span>
                    24h volume <b>$18.4m</b>
                  </span>
                  <span>
                    Liquidity <b>$42.8m</b>
                  </span>
                </div>
              </aside>
            </div>
          ) : activeTab === "Pools" ? (
            <section className="pools-view">
              <div className="pools-heading">
                <div>
                  <span className="eyebrow">SAMPLE LIQUIDITY</span>
                  <h2>Explore pools</h2>
                </div>
                <button
                  onClick={() =>
                    setNotice(
                      "Pool creation is disabled in this local demo; no funds can be deposited.",
                    )
                  }
                >
                  ＋ Create pool
                </button>
              </div>
              <div className="pool-table">
                <div className="pool-row pool-head">
                  <span>PAIR</span>
                  <span>TVL</span>
                  <span>VOLUME (24H)</span>
                  <span>SAMPLE APY</span>
                  <span />
                </div>
                {pools.map(([pair, tvl, volume, apy]) => (
                  <div className="pool-row" key={pair}>
                    <span className="pair-cell">
                      <button
                        className={`pool-fav ${favorite.includes(pair) ? "fav-on" : ""}`}
                        onClick={() => toggleFavorite(pair)}
                      >
                        {favorite.includes(pair) ? "★" : "☆"}
                      </button>
                      <span className="pair-icons">
                        <i>{pair.split(" / ")[0].slice(0, 1)}</i>
                        <i>{pair.split(" / ")[1].slice(0, 1)}</i>
                      </span>
                      <b>{pair}</b>
                    </span>
                    <span>{tvl}</span>
                    <span>{volume}</span>
                    <span className="apy">{apy}</span>
                    <button
                      className="deposit-button"
                      onClick={() =>
                        setNotice(
                          "Pool deposits are disabled. No wallet or liquidity program is connected.",
                        )
                      }
                    >
                      Add liquidity
                    </button>
                  </div>
                ))}
              </div>
              <p className="pool-disclosure">
                All pool metrics are invented for the interface concept; they do not describe actual
                liquidity or returns.
              </p>
            </section>
          ) : (
            <section className="activity-view">
              <span className="eyebrow">RECENT ACTIVITY · SAMPLE ONLY</span>
              <h2>No wallet activity to show.</h2>
              <p>
                This showcase does not connect to a wallet or query the Solana network. Transactions
                will not appear here.
              </p>
              <div>
                ◎{" "}
                <span>
                  Connect a reviewed wallet integration to display real transaction history.
                </span>
              </div>
            </section>
          )}
          <footer className="swap-footer">
            SOLSWAP SHOWCASE <span>Illustrative interface · No wallet or network connection</span>
          </footer>
        </div>
      </section>
      {notice && (
        <div className="swap-toast" role="status">
          {notice}
          <button onClick={() => setNotice("")} aria-label="Dismiss">
            ×
          </button>
        </div>
      )}
    </main>
  );
}
