import { Search } from "lucide-react";
import type { FormEvent } from "react";
import type { SymbolRecord } from "../types";

type SymbolPickerProps = {
  readonly symbols: readonly SymbolRecord[];
  readonly selectedSymbol: string;
  readonly query: string;
  readonly onQueryChange: (query: string) => void;
  readonly onSelect: (symbol: string) => void;
  readonly onSearch: (query: string) => void;
};

export function SymbolPicker({
  symbols,
  selectedSymbol,
  query,
  onQueryChange,
  onSelect,
  onSearch,
}: SymbolPickerProps) {
  const visibleSymbols = symbols
    .filter((symbol) => symbol.kind !== "test_case")
    .filter((symbol) => matches(symbol, query))
    .slice(0, 80);

  function submit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    onSearch(query.trim());
  }

  return (
    <aside className="inspector">
      <form className="search-box" onSubmit={submit}>
        <Search size={18} aria-hidden="true" />
        <input
          aria-label="Search symbols"
          value={query}
          onChange={(event) => onQueryChange(event.currentTarget.value)}
          placeholder="symbol or file"
        />
      </form>
      <ul className="symbol-list" aria-label="Symbols">
        {visibleSymbols.map((symbol) => (
          <li key={symbol.versioned_symbol_id}>
            <button
              type="button"
              className={
                symbol.fqn === selectedSymbol ? "symbol selected" : "symbol"
              }
              onClick={() => onSelect(symbol.fqn)}
            >
              <span>{symbol.fqn}</span>
              <small>
                {symbol.kind} / {symbol.file_path}
              </small>
            </button>
          </li>
        ))}
      </ul>
    </aside>
  );
}

function matches(symbol: SymbolRecord, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return (
    symbol.fqn.toLowerCase().includes(needle) ||
    symbol.file_path.toLowerCase().includes(needle)
  );
}
