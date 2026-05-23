import { createContext, useContext, useEffect, useState } from "react";

export type Theme = "dark" | "light" | "system";
export type BrandTheme = "flame" | "violet" | "cobalt" | "emerald" | "rose" | "midnight";

export const BRAND_THEMES: { value: BrandTheme; label: string; color: string }[] = [
  { value: "flame",    label: "Flame",    color: "#f97316" },
  { value: "violet",   label: "Violet",   color: "#8b5cf6" },
  { value: "cobalt",   label: "Cobalt",   color: "#3b82f6" },
  { value: "emerald",  label: "Emerald",  color: "#10b981" },
  { value: "rose",     label: "Rose",     color: "#f43f5e" },
  { value: "midnight", label: "Midnight", color: "#38bdf8" },
];

interface ThemeContextValue {
  theme: Theme;
  resolved: "dark" | "light";
  setTheme: (t: Theme) => void;
  brand: BrandTheme;
  setBrand: (b: BrandTheme) => void;
}

const ThemeContext = createContext<ThemeContextValue>({
  theme: "dark",
  resolved: "dark",
  setTheme: () => {},
  brand: "flame",
  setBrand: () => {},
});

const THEME_KEY = "rustman-theme";
const BRAND_KEY = "rustman-brand";

function getSystemTheme(): "dark" | "light" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(
    () => (localStorage.getItem(THEME_KEY) as Theme) ?? "dark",
  );
  const [brand, setBrandState] = useState<BrandTheme>(
    () => (localStorage.getItem(BRAND_KEY) as BrandTheme) ?? "flame",
  );
  const [systemTheme, setSystemTheme] = useState<"dark" | "light">(getSystemTheme);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => setSystemTheme(e.matches ? "dark" : "light");
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  const resolved = theme === "system" ? systemTheme : theme;

  useEffect(() => {
    const root = document.documentElement;
    root.classList.remove("dark", "light");
    root.classList.add(resolved);
  }, [resolved]);

  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute("data-brand", brand === "flame" ? "" : brand);
  }, [brand]);

  const setTheme = (t: Theme) => {
    localStorage.setItem(THEME_KEY, t);
    setThemeState(t);
  };

  const setBrand = (b: BrandTheme) => {
    localStorage.setItem(BRAND_KEY, b);
    setBrandState(b);
  };

  return (
    <ThemeContext.Provider value={{ theme, resolved, setTheme, brand, setBrand }}>
      {children}
    </ThemeContext.Provider>
  );
}

export const useTheme = () => useContext(ThemeContext);
